
use ash::vk;
use ash::version::DeviceV1_0;

use crate::context::VkDevice;
use crate::context::VkObjectCreatable;
use crate::ci::{VulkanCI, VkObjectBuildableCI};
use crate::error::{VkResult, VkError};

use std::ptr;

// ----------------------------------------------------------------------------------------------
/// Wrapper class for vk::SemaphoreCreateInfo.
#[derive(Debug, Clone)]
pub struct SemaphoreCI {
    ci: vk::SemaphoreCreateInfo,
}

impl VulkanCI for SemaphoreCI {
    type CIType = vk::SemaphoreCreateInfo;

    fn default_ci() -> Self::CIType {

        vk::SemaphoreCreateInfo {
            s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
            p_next: ptr::null(),
            flags : vk::SemaphoreCreateFlags::empty(),
        }
    }
}

impl VkObjectBuildableCI for SemaphoreCI {
    type ObjectType = vk::Semaphore;

    fn build(&self, device: &VkDevice) -> VkResult<Self::ObjectType> {

        let semaphore = unsafe {
            device.logic.handle.create_semaphore(&self.ci, None)
                .map_err(|_| VkError::create("Semaphore"))?
        };
        Ok(semaphore)
    }
}

impl SemaphoreCI {

    pub fn new() -> SemaphoreCI {

        SemaphoreCI {
            ci: SemaphoreCI::default_ci(),
        }
    }

    pub fn flags(mut self, flags: vk::SemaphoreCreateFlags) {
        self.ci.flags = flags;
    }
}

impl VkObjectCreatable for vk::Semaphore {

    fn discard(self, device: &VkDevice) {
        unsafe {
            device.logic.handle.destroy_semaphore(self, None);
        }
    }
}
// ----------------------------------------------------------------------------------------------

// ----------------------------------------------------------------------------------------------
/// Wrapper class for vk::SemaphoreCreateInfo.
#[derive(Debug, Clone)]
pub struct FenceCI {
    ci: vk::FenceCreateInfo,
}

impl VulkanCI for FenceCI {
    type CIType = vk::FenceCreateInfo;

    fn default_ci() -> Self::CIType {

        vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            p_next: ptr::null(),
            flags : vk::FenceCreateFlags::empty(),
        }
    }
}

impl VkObjectBuildableCI for FenceCI {
    type ObjectType = vk::Fence;

    fn build(&self, device: &VkDevice) -> VkResult<Self::ObjectType> {

        let fence = unsafe {
            device.logic.handle.create_fence(&self.ci, None)
                .or(Err(VkError::create("Fence")))?
        };
        Ok(fence)
    }
}

impl FenceCI {

    pub fn new(is_signed: bool) -> FenceCI {

        let mut fence = FenceCI { ci: FenceCI::default_ci() };

        if is_signed {
            fence.ci.flags = vk::FenceCreateFlags::SIGNALED;
        }

        fence
    }
}

impl VkObjectCreatable for vk::Fence {

    fn discard(self, device: &VkDevice) {
        unsafe {
            device.logic.handle.destroy_fence(self, None);
        }
    }
}

impl VkObjectCreatable for &Vec<vk::Fence> {

    fn discard(self, device: &VkDevice) {

        for fence in self {
            device.discard(*fence);
        }
    }
}
// ----------------------------------------------------------------------------------------------
