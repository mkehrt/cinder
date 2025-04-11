// Followed from https://hoj-senna.github.io/ashen-aetna/

use ash::{vk, Entry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::linked();

    // AFAICT, app_info instance_create_info, &c. cannot be factored out into their own functions
    // because they do not take ownership of their builder arguments.  As such, they can't be
    // returned upstack in safe code, and in unsafe code doing so produces stack corruption!

    let engine_name = std::ffi::CString::new("Engine").unwrap();
    let app_name = std::ffi::CString::new("Application").unwrap();
    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .engine_name(&engine_name);

    let mut extension_names = Vec::new();

    // Magic to make OSX work.  See
    // - https://github.com/ash-rs/ash/blob/76baaafe2940491093d323a9c7f84fa80b92d1e0/ash-examples/src/lib.rs#L229 and following lines
    // - https://stackoverflow.com/questions/58732459/vk-error-incompatible-driver-with-mac-os-and-vulkan-moltenvk
    extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
    extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
    let flags = vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;

    let validation_layer_name = std::ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap();
    let layer_names = vec![validation_layer_name.as_ptr()];

    extension_names.push(vk::EXT_DEBUG_UTILS_NAME.as_ptr());

    let instance_create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names)
        .enabled_layer_names(&layer_names)
        .flags(flags);

    dbg!(&instance_create_info);

    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

    let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);
    let debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(vulkan_debug_utils_callback));

    let utils_messenger =
        unsafe { debug_utils.create_debug_utils_messenger(&debugcreateinfo, None)? };

    unsafe {
        debug_utils.destroy_debug_utils_messenger(utils_messenger, None);
        instance.destroy_instance(None)
    };

    Ok(())
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);
    vk::FALSE
}
