
pub mod platforms;
pub mod utils;

mod error;
mod context;

// type alias ------------------------------------
/// unsigned integer type commonly used in vulkan(an alias type of uint32_t).
#[allow(non_camel_case_types)]
pub type vkuint = u32;
/// signed integer type used in vulkan(an alias type of int32_t).
#[allow(non_camel_case_types)]
pub type vksint = i32;
/// float type used in vulkan.
#[allow(non_camel_case_types)]
pub type vkfloat = ::std::os::raw::c_float;
/// unsigned long integer type used in vulkan.
#[allow(non_camel_case_types)]
pub type vklint = u64;
/// char type used in vulkan.
#[allow(non_camel_case_types)]
pub type vkchar = ::std::os::raw::c_char;
/// boolean type used in vulkan(an alias type of VkBool32).
#[allow(non_camel_case_types)]
pub type vkbool = ash::vk::Bool32;
#[allow(non_camel_case_types)]
// raw pointer type used in vulkan.
pub type vkptr = *mut ::std::os::raw::c_void;
/// the number of bytes, used to measure the size of memory block(buffer, image...).
#[allow(non_camel_case_types)]
pub type vkbytes = ash::vk::DeviceSize;
// -----------------------------------------------