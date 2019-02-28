
use ash::vk;

use std::ptr;
use std::mem;
use std::path::Path;

use vkbase::context::{VkDevice, VkSwapchain};
use vkbase::ci::VkObjectBuildableCI;
use vkbase::ci::buffer::BufferCI;
use vkbase::ci::memory::MemoryAI;
use vkbase::ci::shader::{ShaderModuleCI, ShaderStageCI};
use vkbase::gltf::VkglTFModel;
use vkbase::ui::{TextInfo, TextHAlign};
use vkbase::context::VulkanContext;
use vkbase::utils::color::VkColor;
use vkbase::{FlightCamera, FrameAction};
use vkbase::{vkbytes, vkptr, Point3F, Matrix4F, Vector4F};
use vkbase::VkResult;

use vkexamples::VkExampleBackendRes;

const PHONG_VERTEX_SHADER_SOURCE_PATH      : &'static str = "examples/src/pipelines/phong.vert.glsl";
const PHONG_FRAGMENT_SHADER_SOURCE_PATH    : &'static str = "examples/src/pipelines/phong.frag.glsl";
const TOON_VERTEX_SHADER_SOURCE_PATH       : &'static str = "examples/src/pipelines/toon.vert.glsl";
const TOON_FRAGMENT_SHADER_SOURCE_PATH     : &'static str = "examples/src/pipelines/toon.frag.glsl";
const WIREFRAME_VERTEX_SHADER_SOURCE_PATH  : &'static str = "examples/src/pipelines/wireframe.vert.glsl";
const WIREFRAME_FRAGMENT_SHADER_SOURCE_PATH: &'static str = "examples/src/pipelines/wireframe.frag.glsl";
const MODEL_PATH: &'static str = "assets/models/treasure_smooth.gltf";


pub struct VulkanExample {

    backend_res: VkExampleBackendRes,

    model: VkglTFModel,
    uniform_buffer: UniformBuffer,

    pipelines: PipelineStaff,
    descriptors: DescriptorStaff,

    ubo_data: [UboVS; 1],
    camera: FlightCamera,

    is_toggle_event: bool,
}

struct PipelineStaff {
    phong     : vk::Pipeline,
    wireframe : vk::Pipeline,
    toon      : vk::Pipeline,
    layout: vk::PipelineLayout,
}

impl VulkanExample {

    pub fn new(context: &VulkanContext) -> VkResult<VulkanExample> {

        let device = &context.device;
        let swapchain = &context.swapchain;
        let dimension = swapchain.dimension;

        let mut camera = FlightCamera::new()
            .place_at(Point3F::new(0.25, 6.25, 8.75))
            .screen_aspect_ratio((dimension.width as f32 / 3.0) / dimension.height as f32)
            .pitch(-45.0)
            .build();
        camera.set_move_speed(50.0);

        let ubo_data = [
            UboVS {
                projection : camera.proj_matrix(),
                view       : camera.view_matrix(),
                model      : Matrix4F::identity(),
                light_pos  : Vector4F::new(0.0, 2.0, 1.0, 0.0),
            },
        ];

        let render_pass = setup_renderpass(device, &context.swapchain)?;
        let backend_res = VkExampleBackendRes::new(device, swapchain, render_pass)?;

        let model = prepare_model(device)?;
        let uniform_buffer = prepare_uniform(device, &ubo_data)?;
        let descriptors = setup_descriptor(device, &uniform_buffer, &model)?;

        let pipelines = prepare_pipelines(device, &model, backend_res.render_pass, descriptors.layout)?;

        let target = VulkanExample {
            backend_res, model, uniform_buffer, descriptors, pipelines, camera, ubo_data,
            is_toggle_event: false,
        };
        Ok(target)
    }
}

impl vkbase::RenderWorkflow for VulkanExample {

    fn init(&mut self, device: &VkDevice) -> VkResult<()> {

        self.backend_res.set_basic_ui(device, super::WINDOW_TITLE)?;

        let screen_width  = self.backend_res.dimension.width  as i32;
        let screen_height = self.backend_res.dimension.height as i32;

        let phong_text = TextInfo {
            content: String::from("Phong Shading Pipeline"),
            scale: 16.0,
            align: TextHAlign::Left,
            color: VkColor::WHITE,
            location: vk::Offset2D { x: screen_width / 12, y: screen_height / 8 * 7 },
            capacity: None,
        };
        self.backend_res.ui_renderer.add_text(phong_text)?;

        let toon_text = TextInfo {
            content: String::from("Toon Shading Pipeline"),
            scale: 16.0,
            align: TextHAlign::Left,
            color: VkColor::WHITE,
            location: vk::Offset2D { x: screen_width / 12 * 5, y: screen_height / 8 * 7 },
            capacity: None,
        };
        self.backend_res.ui_renderer.add_text(toon_text)?;

        let wireframe_text = TextInfo {
            content: String::from("Wireframe Pipeline"),
            scale: 16.0,
            align: TextHAlign::Left,
            color: VkColor::WHITE,
            location: vk::Offset2D { x: screen_width / 12 * 9, y: screen_height / 8 * 7 },
            capacity: None,
        };
        self.backend_res.ui_renderer.add_text(wireframe_text)?;

        self.record_commands(device, self.backend_res.dimension)?;

        Ok(())
    }

    fn render_frame(&mut self, device: &VkDevice, device_available: vk::Fence, await_present: vk::Semaphore, image_index: usize, _delta_time: f32) -> VkResult<vk::Semaphore> {

        if self.is_toggle_event {
            self.update_uniforms(device)?;
        }

        let submit_ci = vkbase::ci::device::SubmitCI::new()
            .add_wait(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, await_present)
            .add_command(self.backend_res.commands[image_index])
            .add_signal(self.backend_res.await_rendering);

        // Submit to the graphics queue passing a wait fence.
        device.submit(submit_ci, device.logic.queues.graphics.handle, device_available)?;

        Ok(self.backend_res.await_rendering)
    }

    fn swapchain_reload(&mut self, device: &VkDevice, new_chain: &VkSwapchain) -> VkResult<()> {

        // recreate the resources.
        device.discard(self.pipelines.phong);
        device.discard(self.pipelines.toon);
        device.discard(self.pipelines.wireframe);

        let render_pass = setup_renderpass(device, new_chain)?;
        self.backend_res.swapchain_reload(device, new_chain, render_pass)?;
        self.pipelines = prepare_pipelines(device, &self.model, self.backend_res.render_pass, self.descriptors.layout)?;

        self.record_commands(device, self.backend_res.dimension)?;

        Ok(())
    }

    fn receive_input(&mut self, inputer: &vkbase::EventController, delta_time: f32) -> FrameAction {

        if inputer.is_key_active() || inputer.is_cursor_active() {

            if inputer.key.is_key_pressed(winit::VirtualKeyCode::Escape) {
                return FrameAction::Terminal
            }

            self.is_toggle_event = true;
            self.camera.receive_input(inputer, delta_time);
        } else {
            self.is_toggle_event = false;
        }

        self.backend_res.update_fps_text(inputer);

        FrameAction::Rendering
    }

    fn deinit(&mut self, device: &VkDevice) -> VkResult<()> {

        self.discard(device);
        Ok(())
    }
}

impl VulkanExample {

    fn record_commands(&self, device: &VkDevice, dimension: vk::Extent2D) -> VkResult<()> {

        let clear_values = [
            vkexamples::DEFAULT_CLEAR_COLOR.clone(),
            vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
        ];

        let scissor = vk::Rect2D {
            extent: dimension.clone(),
            offset: vk::Offset2D { x: 0, y: 0 },
        };

        for (i, &command) in self.backend_res.commands.iter().enumerate() {

            use vkbase::command::{VkCmdRecorder, CmdGraphicsApi, IGraphics};
            use vkbase::ci::pipeline::RenderPassBI;

            let render_params = vkbase::gltf::ModelRenderParams {
                descriptor_set : self.descriptors.set,
                pipeline_layout: self.pipelines.layout,
                material_stage : vk::ShaderStageFlags::VERTEX,
            };

            let mut viewport = vk::Viewport {
                x: 0.0, y: 0.0,
                width: dimension.width as f32, height: dimension.height as f32,
                min_depth: 0.0, max_depth: 1.0,
            };

            let recorder: VkCmdRecorder<IGraphics> = VkCmdRecorder::new(device, command);

            let render_pass_bi = RenderPassBI::new(self.backend_res.render_pass, self.backend_res.framebuffers[i])
                .render_extent(dimension)
                .clear_values(&clear_values);

            recorder.begin_record()?
                .begin_render_pass(render_pass_bi)
                .set_scissor(0, &[scissor]);

            { // Left: Solid colored
                viewport.width = dimension.width as f32 / 3.0;
                recorder
                    .set_viewport(0, &[viewport])
                    .bind_pipeline(self.pipelines.phong);
                self.model.record_command(&recorder, &render_params);
            }

            { // Center: Toon
                viewport.x = dimension.width as f32 / 3.0;
                recorder
                    .set_viewport(0, &[viewport])
                    .bind_pipeline(self.pipelines.toon);

                // Line width > 1.0f only if wide lines feature is supported.
                if device.phy.enable_features().wide_lines == vk::TRUE {
                    recorder.set_line_width(2.0);
                }
                self.model.record_command(&recorder, &render_params);
            }

            { // Right: Wireframe
                if device.phy.enable_features().fill_mode_non_solid == vk::TRUE {
                    viewport.x = dimension.width as f32 / 3.0 * 2.0;
                    recorder
                        .set_viewport(0, &[viewport])
                        .bind_pipeline(self.pipelines.wireframe);
                    self.model.record_command(&recorder, &render_params);
                }
            }

            self.backend_res.ui_renderer.record_command(&recorder);

            recorder
                .end_render_pass()
                .end_record()?;
        }

        Ok(())
    }

    fn update_uniforms(&mut self, device: &VkDevice) -> VkResult<()> {

        self.ubo_data[0].view = self.camera.view_matrix();

        // dbg!(self.ubo_data[0].view);
        device.copy_to_ptr(self.uniform_buffer.data_ptr, &self.ubo_data);

        Ok(())
    }

    fn discard(&self, device: &VkDevice) {

        device.discard(self.descriptors.layout);
        device.discard(self.descriptors.pool);

        device.discard(self.pipelines.phong);
        device.discard(self.pipelines.toon);
        device.discard(self.pipelines.wireframe);
        device.discard(self.pipelines.layout);

        device.unmap_memory(self.uniform_buffer.memory);
        device.discard(self.uniform_buffer.buffer);
        device.discard(self.uniform_buffer.memory);

        self.model.discard(device);
        self.backend_res.discard(device);
    }
}

// Prepare model from glTF file.
pub fn prepare_model(device: &VkDevice) -> VkResult<VkglTFModel> {

    use vkbase::gltf::{GltfModelInfo, load_gltf};
    use vkbase::gltf::{AttributeFlags, NodeAttachmentFlags};

    let model_info = GltfModelInfo {
        path: Path::new(MODEL_PATH),
        attribute: AttributeFlags::POSITION | AttributeFlags::NORMAL, // specify model's vertex layout.
        node: NodeAttachmentFlags::TRANSFORM_MATRIX, // specify model's node attachment layout.
    };

    let model = load_gltf(device, model_info)?;
    Ok(model)
}


/// Uniform buffer block object.
struct UniformBuffer {

    data_ptr: vkptr,
    memory: vk::DeviceMemory,
    buffer: vk::Buffer,
    descriptor: vk::DescriptorBufferInfo,
}

// The uniform data that will be transferred to shader.
//
// layout (set = 0, binding = 0) uniform UBO {
//     mat4 projection;
//     mat4 view;
//     mat4 model;
//     vec4 lightPos;
// } ubo;
#[derive(Debug, Clone, Copy)]
struct UboVS {
    projection   : Matrix4F,
    view         : Matrix4F,
    model        : Matrix4F,
    light_pos    : Vector4F,
}

fn prepare_uniform(device: &VkDevice, ubo_data: &[UboVS; 1]) -> VkResult<UniformBuffer> {

    let (uniform_buffer, memory_requirement) = BufferCI::new(mem::size_of::<[UboVS; 1]>() as vkbytes)
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .build(device)?;

    let memory_type = device.get_memory_type(memory_requirement.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
    let uniform_memory = MemoryAI::new(memory_requirement.size, memory_type)
        .build(device)?;
    device.bind_memory(uniform_buffer, uniform_memory, 0)?;

    // Map uniform buffer and update it.
    // keep the uniform memory map during the program running.
    let data_ptr = device.map_memory(uniform_memory, 0, mem::size_of::<[UboVS; 1]>() as vkbytes)?;
    device.copy_to_ptr(data_ptr, ubo_data);

    let uniforms = UniformBuffer {
        data_ptr,
        buffer: uniform_buffer,
        memory: uniform_memory,
        descriptor: vk::DescriptorBufferInfo {
            buffer: uniform_buffer,
            offset: 0,
            range : mem::size_of::<[UboVS; 1]>() as vkbytes,
        },
    };

    Ok(uniforms)
}

struct DescriptorStaff {
    pool   : vk::DescriptorPool,
    set    : vk::DescriptorSet,
    layout : vk::DescriptorSetLayout,
}

fn setup_descriptor(device: &VkDevice, uniforms: &UniformBuffer, model: &VkglTFModel) -> VkResult<DescriptorStaff> {

    use vkbase::ci::descriptor::{DescriptorPoolCI, DescriptorSetLayoutCI};
    use vkbase::ci::descriptor::{DescriptorSetAI, DescriptorBufferSetWI, DescriptorSetsUpdateCI};

    // Descriptor Pool.
    let descriptor_pool = DescriptorPoolCI::new(1)
        .add_descriptor(vk::DescriptorType::UNIFORM_BUFFER, 1)
        .add_descriptor(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, 1)
        .build(device)?;

    // ubo_descriptor represent shader codes as follows:
    // layout (set = 0, binding = 0) uniform UBO {
    //     mat4 projection;
    //     mat4 view;
    //     mat4 model;
    //     mat4 y_correction;
    //     vec4 lightPos;
    // } ubo;
    let ubo_descriptor = vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX,
        p_immutable_samplers: ptr::null(),
    };

    // node_descriptor represent shader codes as follows:
    // layout (set = 0, binding = 1) uniform NodeAttachments {
    //     mat4 transform;
    // } node_attachments;
    let node_descriptor = vk::DescriptorSetLayoutBinding {
        binding: 1,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX,
        p_immutable_samplers: ptr::null(),
    };

    let set_layout = DescriptorSetLayoutCI::new()
        .add_binding(ubo_descriptor)
        .add_binding(node_descriptor)
        .build(device)?;

    // Descriptor set.
    let mut descriptor_sets = DescriptorSetAI::new(descriptor_pool)
        .add_set_layout(set_layout)
        .build(device)?;
    let descriptor_set = descriptor_sets.remove(0);

    let ubo_write_info = DescriptorBufferSetWI::new(descriptor_set, 0, vk::DescriptorType::UNIFORM_BUFFER)
        .add_buffer(uniforms.descriptor.clone());
    let node_write_info = DescriptorBufferSetWI::new(descriptor_set, 1, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
        .add_buffer(model.nodes.node_descriptor());

    DescriptorSetsUpdateCI::new()
        .add_write(ubo_write_info.value())
        .add_write(node_write_info.value())
        .update(device);

    let descriptors = DescriptorStaff {
        pool   : descriptor_pool,
        set    : descriptor_set,
        layout : set_layout,
    };
    Ok(descriptors)
}

fn setup_renderpass(device: &VkDevice, swapchain: &VkSwapchain) -> VkResult<vk::RenderPass> {

    use vkbase::ci::pipeline::RenderPassCI;
    use vkbase::ci::pipeline::{AttachmentDescCI, SubpassDescCI, SubpassDependencyCI};

    let color_attachment = AttachmentDescCI::new(swapchain.backend_format)
        .op(vk::AttachmentLoadOp::CLEAR, vk::AttachmentStoreOp::STORE)
        .layout(vk::ImageLayout::UNDEFINED, vk::ImageLayout::PRESENT_SRC_KHR);

    let depth_attachment = AttachmentDescCI::new(device.phy.depth_format)
        .op(vk::AttachmentLoadOp::CLEAR, vk::AttachmentStoreOp::DONT_CARE)
        .layout(vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let subpass_description = SubpassDescCI::new(vk::PipelineBindPoint::GRAPHICS)
        .add_color_attachment(0, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) // Attachment 0 is color.
        .set_depth_stencil_attachment(1, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL); // Attachment 1 is depth-stencil.

    let dependency0 = SubpassDependencyCI::new(vk::SUBPASS_EXTERNAL, 0)
        .stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .access_mask(vk::AccessFlags::MEMORY_READ, vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .flags(vk::DependencyFlags::BY_REGION);

    let dependency1 = SubpassDependencyCI::new(0, vk::SUBPASS_EXTERNAL)
        .stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, vk::PipelineStageFlags::BOTTOM_OF_PIPE)
        .access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE, vk::AccessFlags::MEMORY_READ)
        .flags(vk::DependencyFlags::BY_REGION);

    let render_pass = RenderPassCI::new()
        .add_attachment(color_attachment.value())
        .add_attachment(depth_attachment.value())
        .add_subpass(subpass_description.value())
        .add_dependency(dependency0.value())
        .add_dependency(dependency1.value())
        .build(device)?;

    Ok(render_pass)
}

fn prepare_pipelines(device: &VkDevice, model: &VkglTFModel, render_pass: vk::RenderPass, set_layout: vk::DescriptorSetLayout) -> VkResult<PipelineStaff> {

    use vkbase::ci::pipeline::*;

    let viewport_state = ViewportSCI::new()
        .add_viewport(vk::Viewport::default())
        .add_scissor(vk::Rect2D::default());

    let mut rasterization_state = RasterizationSCI::new()
        .polygon(vk::PolygonMode::FILL)
        .cull_face(vk::CullModeFlags::BACK, vk::FrontFace::CLOCKWISE);

    let blend_attachment = BlendAttachmentSCI::new().value();
    let blend_state = ColorBlendSCI::new()
        .add_attachment(blend_attachment);

    let depth_stencil_state = DepthStencilSCI::new()
        .depth_test(true, true, vk::CompareOp::LESS_OR_EQUAL);

    let mut dynamic_state = DynamicSCI::new()
        .add_dynamic(vk::DynamicState::VIEWPORT)
        .add_dynamic(vk::DynamicState::SCISSOR);

    if device.phy.enable_features().wide_lines == vk::TRUE {
        dynamic_state = dynamic_state.add_dynamic(vk::DynamicState::LINE_WIDTH)
    };

    let material_range = vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX,
        offset: 0,
        size: model.materials.material_size(),
    };

    // Pipeline Layout.
    let pipeline_layout = PipelineLayoutCI::new()
        .add_set_layout(set_layout)
        .add_push_constants(material_range)
        .build(device)?;

    // base pipeline.
    let mut pipeline_ci = GraphicsPipelineCI::new(render_pass, pipeline_layout);

    pipeline_ci.set_vertex_input(model.meshes.vertex_input.clone());
    pipeline_ci.set_viewport(viewport_state);
    pipeline_ci.set_rasterization(rasterization_state.clone());
    pipeline_ci.set_depth_stencil(depth_stencil_state);
    pipeline_ci.set_color_blend(blend_state);
    pipeline_ci.set_dynamic(dynamic_state);


    let mut shader_compiler = vkbase::utils::shaderc::VkShaderCompiler::new()?;

    let phong_pipeline = {

        let vert_codes = shader_compiler.compile_from_path(Path::new(PHONG_VERTEX_SHADER_SOURCE_PATH), shaderc::ShaderKind::Vertex, "[Vertex Shader]", "main")?;
        let frag_codes = shader_compiler.compile_from_path(Path::new(PHONG_FRAGMENT_SHADER_SOURCE_PATH), shaderc::ShaderKind::Fragment, "[Fragment Shader]", "main")?;

        let vert_module = ShaderModuleCI::from_glsl(vk::ShaderStageFlags::VERTEX, vert_codes)
            .build(device)?;
        let frag_module = ShaderModuleCI::from_glsl(vk::ShaderStageFlags::FRAGMENT, frag_codes)
            .build(device)?;

        pipeline_ci.set_shaders(vec![
            ShaderStageCI::new(vk::ShaderStageFlags::VERTEX, vert_module),
            ShaderStageCI::new(vk::ShaderStageFlags::FRAGMENT, frag_module),
        ]);

        // Using this pipeline as the base for the other pipelines (derivatives).
        // Pipeline derivatives can be used for pipelines that share most of their state
        // depending on the implementation this may result in better performance for pipeline switching and faster creation time.
        pipeline_ci.set_flags(vk::PipelineCreateFlags::ALLOW_DERIVATIVES);

        let pipeline = device.build(&pipeline_ci)?;

        device.discard(vert_module);
        device.discard(frag_module);

        pipeline
    };

    let toon_pipeline = {

        let vert_codes = shader_compiler.compile_from_path(Path::new(TOON_VERTEX_SHADER_SOURCE_PATH), shaderc::ShaderKind::Vertex, "[Vertex Shader]", "main")?;
        let frag_codes = shader_compiler.compile_from_path(Path::new(TOON_FRAGMENT_SHADER_SOURCE_PATH), shaderc::ShaderKind::Fragment, "[Fragment Shader]", "main")?;

        let vert_module = ShaderModuleCI::from_glsl(vk::ShaderStageFlags::VERTEX, vert_codes)
            .build(device)?;
        let frag_module = ShaderModuleCI::from_glsl(vk::ShaderStageFlags::FRAGMENT, frag_codes)
            .build(device)?;

        pipeline_ci.set_shaders(vec![
            ShaderStageCI::new(vk::ShaderStageFlags::VERTEX, vert_module),
            ShaderStageCI::new(vk::ShaderStageFlags::FRAGMENT, frag_module),
        ]);
        // Base pipeline will be our first created pipeline.
        pipeline_ci.set_base_pipeline(phong_pipeline);
        // All pipelines created after the base pipeline will be derivatives.
        pipeline_ci.set_flags(vk::PipelineCreateFlags::DERIVATIVE);

        let pipeline = device.build(&pipeline_ci)?;

        device.discard(vert_module);
        device.discard(frag_module);

        pipeline
    };

    let wireframe_pipeline = {

        let vert_codes = shader_compiler.compile_from_path(Path::new(WIREFRAME_VERTEX_SHADER_SOURCE_PATH), shaderc::ShaderKind::Vertex, "[Vertex Shader]", "main")?;
        let frag_codes = shader_compiler.compile_from_path(Path::new(WIREFRAME_FRAGMENT_SHADER_SOURCE_PATH), shaderc::ShaderKind::Fragment, "[Fragment Shader]", "main")?;

        let vert_module = ShaderModuleCI::from_glsl(vk::ShaderStageFlags::VERTEX, vert_codes)
            .build(device)?;
        let frag_module = ShaderModuleCI::from_glsl(vk::ShaderStageFlags::FRAGMENT, frag_codes)
            .build(device)?;

        pipeline_ci.set_shaders(vec![
            ShaderStageCI::new(vk::ShaderStageFlags::VERTEX, vert_module),
            ShaderStageCI::new(vk::ShaderStageFlags::FRAGMENT, frag_module),
        ]);

        // Non solid rendering is not a mandatory Vulkan feature.
        if device.phy.enable_features().fill_mode_non_solid == vk::TRUE {
            rasterization_state = rasterization_state.polygon(vk::PolygonMode::LINE);
            pipeline_ci.set_rasterization(rasterization_state);
        }

        let pipeline = device.build(&pipeline_ci)?;

        device.discard(vert_module);
        device.discard(frag_module);

        pipeline
    };


    let result = PipelineStaff {
        phong: phong_pipeline,
        toon : toon_pipeline,
        wireframe: wireframe_pipeline,

        layout: pipeline_layout,
    };
    Ok(result)
}
