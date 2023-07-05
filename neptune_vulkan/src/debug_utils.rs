use ash::vk;
use ash::vk::DebugUtilsObjectNameInfoEXT;
use std::ffi::{CStr, CString};

use log::{error, info, trace, warn};

#[allow(dead_code)]
pub struct DebugUtils {
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    debug_call_back: vk::DebugUtilsMessengerEXT,
}

impl DebugUtils {
    pub(crate) fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> ash::prelude::VkResult<Self> {
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let debug_call_back = unsafe {
            debug_utils_loader.create_debug_utils_messenger(
                &vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR, // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                                                                            // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback)),
                None,
            )?
        };

        Ok(Self {
            debug_utils_loader,
            debug_call_back,
        })
    }

    pub(crate) fn set_object_name<T: vk::Handle>(
        &self,
        device: vk::Device,
        object: T,
        object_name: &str,
    ) {
        let object_name = CString::new(object_name).expect("Failed to create CString");
        unsafe {
            self.debug_utils_loader
                .set_debug_utils_object_name(
                    device,
                    &DebugUtilsObjectNameInfoEXT::builder()
                        .object_type(T::TYPE)
                        .object_handle(object.as_raw())
                        .object_name(object_name.as_c_str())
                        .build(),
                )
                .expect("Failed to set object name");
        }
    }
}

impl Drop for DebugUtils {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_call_back, None);
        }
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use std::borrow::Cow;
    let callback_data = *p_callback_data;
    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE {
        trace!("{:?}", message);
    } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        info!("{:?}", message);
    } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("{:?}", message);
    } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("{:?}", message);
    }

    vk::FALSE
}
