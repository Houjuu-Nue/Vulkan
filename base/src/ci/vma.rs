
use ash::vk;

use crate::ci::VulkanCI;
use crate::context::VmaResourceDiscardable;
use crate::{VkResult, VkErrorKind};
use crate::{vkuint, vkptr};

// ----------------------------------------------------------------------------------------------
pub struct VmaBuffer {

    /// the handle of vk::Buffer.
    pub handle: vk::Buffer,
    /// allocation info managed by vma.
    pub allocation: vma::Allocation,
    /// the meta information about about this memory and allocation.
    pub info: vma::AllocationInfo,
}

impl From<(vk::Buffer, vma::Allocation, vma::AllocationInfo)> for VmaBuffer {

    fn from(content: (vk::Buffer, vma::Allocation, vma::AllocationInfo)) -> VmaBuffer {
        VmaBuffer {
            handle: content.0,
            allocation: content.1,
            info: content.2,
        }
    }
}

impl VmaResourceDiscardable for VmaBuffer {

    fn discard(&self, vma: &mut vma::Allocator) -> VkResult<()> {
        vma.destroy_buffer(self.handle, &self.allocation)
            .map_err(VkErrorKind::Vma)?;
        Ok(())
    }
}

pub struct VmaImage {

    /// the handle of vk::Image.
    pub handle: vk::Image,
    /// allocation info managed by vma.
    pub allocation: vma::Allocation,
    /// the meta information about about this memory and allocation.
    pub info: vma::AllocationInfo,
}


impl From<(vk::Image, vma::Allocation, vma::AllocationInfo)> for VmaImage {

    fn from(content: (vk::Image, vma::Allocation, vma::AllocationInfo)) -> VmaImage {
        VmaImage {
            handle: content.0,
            allocation: content.1,
            info: content.2,
        }
    }
}

impl VmaResourceDiscardable for VmaImage {

    fn discard(&self, vma: &mut vma::Allocator) -> VkResult<()> {
        vma.destroy_image(self.handle, &self.allocation)
            .map_err(VkErrorKind::Vma)?;
        Ok(())
    }
}
// ----------------------------------------------------------------------------------------------


// ----------------------------------------------------------------------------------------------
/// Wrapper class for vma::AllocationCreateInfo.
#[derive(Debug, Clone)]
pub struct VmaAllocationCI {
    ci: vma::AllocationCreateInfo,
}

impl VulkanCI for VmaAllocationCI {
    type CIType = vma::AllocationCreateInfo;

    fn default_ci() -> vma::AllocationCreateInfo {

        vma::AllocationCreateInfo {
            usage: vma::MemoryUsage::Unknown,
            flags: vma::AllocationCreateFlags::NONE,
            required_flags : vk::MemoryPropertyFlags::empty(),
            preferred_flags: vk::MemoryPropertyFlags::empty(),
            // set `memory_type_bits` means to accept all memory type index.
            memory_type_bits: 0,
            pool: None,
            user_data: None,
        }
    }
}

impl VmaAllocationCI {

    pub fn new(usage: vma::MemoryUsage, required_flags: vk::MemoryPropertyFlags) -> VmaAllocationCI {

        VmaAllocationCI {
            ci: vma::AllocationCreateInfo {
                usage, required_flags,
                ..VmaAllocationCI::default_ci()
            }
        }
    }

    #[inline(always)]
    pub fn flags(mut self, flags: vma::AllocationCreateFlags) -> VmaAllocationCI {
        self.ci.flags = flags; self
    }

    #[inline(always)]
    pub fn preferred_flags(mut self, flags: vk::MemoryPropertyFlags) -> VmaAllocationCI {
        self.ci.preferred_flags = flags; self
    }

    #[inline(always)]
    pub fn accept_memory_types(mut self, acceptable_type_bits: vkuint) -> VmaAllocationCI {
        self.ci.memory_type_bits = acceptable_type_bits; self
    }

    #[inline(always)]
    pub fn with_pool(mut self, pool: vma::AllocatorPool) -> VmaAllocationCI {
        self.ci.pool = Some(pool); self
    }

    #[inline(always)]
    pub fn with_user_data(mut self, data_ptr: vkptr) -> VmaAllocationCI {
        self.ci.user_data = Some(data_ptr); self
    }

    #[inline(always)]
    pub fn value(&self) -> vma::AllocationCreateInfo {
        self.ci.clone()
    }
}

impl AsRef<vma::AllocationCreateInfo> for VmaAllocationCI {

    fn as_ref(&self) -> &vma::AllocationCreateInfo {
        &self.ci
    }
}
// ----------------------------------------------------------------------------------------------