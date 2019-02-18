
use ash::vk;
use ash::version::DeviceV1_0;

use crate::gltf::asset::{GltfDocument, AssetAbstract, AssetElementList};
use crate::gltf::scene::Scene;
use crate::gltf::meshes::mesh::Mesh;
use crate::gltf::meshes::attributes::{AttributesData, AttributeFlags};
use crate::gltf::meshes::indices::IndicesData;

use crate::ci::VkObjectBuildableCI;
use crate::ci::memory::MemoryAI;

use crate::context::VkDevice;
use crate::utils::memory::get_memory_type_index;
use crate::error::{VkResult, VkError, VkTryFrom};
use crate::vkbytes;

use std::ptr;


pub struct MeshAsset {

    attributes: AttributesData,
    indices: IndicesData,

    meshes: AssetElementList<Mesh>,
}

pub struct MeshAssetBlock {

    vertex: (vk::Buffer, vkbytes),
    index: Option<(vk::Buffer, vkbytes)>,

    memory: vk::DeviceMemory,
}

impl VkTryFrom<AttributeFlags> for MeshAsset {

    fn try_from(flag: AttributeFlags) -> VkResult<MeshAsset> {

        let result = MeshAsset {
            attributes: AttributesData::try_from(flag)?,
            indices: Default::default(),
            meshes : Default::default(),
        };
        Ok(result)
    }
}

impl AssetAbstract for MeshAsset {
    const ASSET_NAME: &'static str = "Meshes";

    fn read_doc(&mut self, source: &GltfDocument, _scene: &Scene) -> VkResult<()> {

        for doc_mesh in source.doc.meshes() {

            let json_index = doc_mesh.index();
            let mesh = Mesh::from_doc(doc_mesh, source, &mut self.attributes, &mut self.indices)?;

            self.meshes.push(json_index, mesh);
        }

        Ok(())
    }
}

impl MeshAsset {

    fn allocate(self, device: &VkDevice) -> VkResult<MeshAssetBlock> {

        // allocate staging buffer.
        let staging_block = self.allocate_staging(device)?;
        // allocate mesh buffer.
        let mesh_block = self.allocate_mesh(device)?;

        // copy data from staging buffer to mesh buffer.
        MeshAsset::copy_staging2mesh(device, &staging_block, &mesh_block)?;

        // discard staging resource.
        staging_block.discard(device);

        Ok(mesh_block)
    }

    fn allocate_mesh(&self, device: &VkDevice) -> VkResult<MeshAssetBlock> {

        // create buffer and allocate memory for glTF mesh.
        let (vertex_buffer, vertex_requirement) = self.attributes.buffer_ci()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
            .build(device)?;

        let mesh_block = if let Some(indices_ci) = self.indices.buffer_ci() {
            let (index_buffer, index_requirement) = indices_ci
                .usage(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                .build(device)?;

            let memory_type = get_memory_type_index(device, vertex_requirement.memory_type_bits & index_requirement.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL);
            let mesh_memory = MemoryAI::new(vertex_requirement.size + index_requirement.size, memory_type)
                .build(device)?;

            MeshAssetBlock {
                vertex: (vertex_buffer, vertex_requirement.size),
                index: Some((index_buffer, index_requirement.size)),
                memory: mesh_memory,
            }
        } else {
            let memory_type = get_memory_type_index(device, vertex_requirement.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::HOST_COHERENT);
            let mesh_memory = MemoryAI::new(vertex_requirement.size, memory_type)
                .build(device)?;

            MeshAssetBlock {
                vertex: (vertex_buffer, vertex_requirement.size),
                index: None,
                memory: mesh_memory,
            }
        };

        Ok(mesh_block)
    }

    fn allocate_staging(&self, device: &VkDevice) -> VkResult<MeshAssetBlock> {

        // create staging buffer and allocate memory.
        let (vertex_buffer, vertex_requirement) = self.attributes.buffer_ci()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .build(device)?;

        let mesh_block = if let Some(indices_ci) = self.indices.buffer_ci() {
            let (index_buffer, index_requirement) = indices_ci
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .build(device)?;

            let memory_type = get_memory_type_index(device, vertex_requirement.memory_type_bits & index_requirement.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
            let mesh_memory = MemoryAI::new(vertex_requirement.size + index_requirement.size, memory_type)
                .build(device)?;

            MeshAssetBlock {
                vertex: (vertex_buffer, vertex_requirement.size),
                index: Some((index_buffer, index_requirement.size)),
                memory: mesh_memory,
            }

        } else {
            let memory_type = get_memory_type_index(device, vertex_requirement.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
            let mesh_memory = MemoryAI::new(vertex_requirement.size, memory_type)
                .build(device)?;

            MeshAssetBlock {
                vertex: (vertex_buffer, vertex_requirement.size),
                index: None,
                memory: mesh_memory,
            }
        };

        // map and bind staging buffer to memory.
        unsafe {

            // map vertex data.
            let vertex_data_ptr = device.logic.handle.map_memory(mesh_block.memory, 0, mesh_block.vertex.1, vk::MemoryMapFlags::empty())
                .map_err(|_| VkError::device("Map Memory"))?;
            self.attributes.data_content.map_data(vertex_data_ptr);

            // map index data.
            if let Some(ref index_buffer) = mesh_block.index {
                let index_data_ptr = device.logic.handle.map_memory(mesh_block.memory, mesh_block.vertex.1, index_buffer.1.clone(), vk::MemoryMapFlags::empty())
                    .map_err(|_| VkError::device("Map Memory"))?;
                self.indices.map_data(index_data_ptr);
            }

            // unmap the memory.
            device.logic.handle.unmap_memory(mesh_block.memory);
        }

        // bind vertex buffer to memory.
        device.bind(mesh_block.vertex.0, mesh_block.memory, 0)?;
        // bind index buffer to memory.
        if let Some(ref index_buffer) = mesh_block.index {
            device.bind(index_buffer.0, mesh_block.memory, mesh_block.vertex.1)?;
        }

        Ok(mesh_block)
    }

    fn copy_staging2mesh(device: &VkDevice, staging: &MeshAssetBlock, mesh: &MeshAssetBlock) -> VkResult<()> {

        use crate::ci::command::{CommandBufferAI, CommandPoolCI};
        use crate::command::{VkCmdRecorder, ITransfer, CmdTransferApi};

        let command_pool = CommandPoolCI::new(device.logic.queues.transfer.family_index)
            .build(device)?;

        let copy_command = CommandBufferAI::new(command_pool, 1)
            .build(device)?
            .remove(0);

        let cmd_recorder: VkCmdRecorder<ITransfer> = VkCmdRecorder::new(device, copy_command);

        let vertex_copy_region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: staging.vertex.1,
        };

        cmd_recorder.begin_record()?
            .copy_buf2buf(staging.vertex.0, mesh.vertex.0, &[vertex_copy_region]);


        if let Some(ref index_buffer) = staging.index {
            let index_copy_region = vk::BufferCopy {
                src_offset: staging.vertex.1,
                dst_offset: staging.vertex.1,
                size: index_buffer.1,
            };
            cmd_recorder.copy_buf2buf(index_buffer.0, mesh.index.unwrap().0, &[index_copy_region]);
        }

        cmd_recorder.end_record()?;

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: ptr::null(),
            wait_semaphore_count   : 0,
            p_wait_semaphores      : ptr::null(),
            p_wait_dst_stage_mask  : ptr::null(),
            command_buffer_count   : 1,
            p_command_buffers      : &copy_command,
            signal_semaphore_count : 0,
            p_signal_semaphores    : ptr::null(),
        };

        use crate::ci::sync::FenceCI;
        use crate::utils::time::VkTimeDuration;
        let fence = device.build(&FenceCI::new(false))?;

        unsafe {
            device.logic.handle.queue_submit(device.logic.queues.transfer.handle, &[submit_info], fence)
                .map_err(|_| VkError::device("Queue Submit"))?;

            device.logic.handle.wait_for_fences(&[fence], true, VkTimeDuration::Infinite.into())
                .map_err(|_| VkError::device("Wait for fences"))?;
        }

        // release temporary resource.
        device.discard(fence);
        // free the command poll will automatically destroy all command buffers created by this pool.
        device.discard(command_pool);

        Ok(())
    }
}

impl MeshAssetBlock {

    fn discard(&self, device: &VkDevice) {

        device.discard(self.vertex.0);
        if let Some(ref index_buffer) = self.index {
            device.discard(index_buffer.0);
        }
        device.discard(self.memory);
    }
}
