// Followed from https://hoj-senna.github.io/ashen-aetna/

use ash::{vk, Entry, Instance};
use std::ffi::CStr;

static ENGINE_NAME: &CStr = c"Engine";
static APP_NAME: &CStr = c"Application";

static VALIDATION_LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

fn main() -> Result<(), vk::Result> {
    let entry = Entry::linked();

    let instance: Instance = create_instance(&entry)?;
    let (debug_utils, debug_utils_messenger) = create_debug_utils_and_messenger(&entry, &instance)?;
    let physical_device: vk::PhysicalDevice = create_physical_device(&instance)?;
    let queue_family_indices = get_queue_family_indices(&instance, &physical_device);

    unsafe {
        debug_utils.destroy_debug_utils_messenger(debug_utils_messenger, None);
        instance.destroy_instance(None)
    }

    Ok(())
}

fn create_instance(entry: &Entry) -> Result<Instance, vk::Result> {
    let app_info = vk::ApplicationInfo::default()
        .application_name(&APP_NAME)
        .engine_name(&ENGINE_NAME);

    let mut extension_names = Vec::new();

    // Magic to make OSX work.  See
    // - https://github.com/ash-rs/ash/blob/76baaafe2940491093d323a9c7f84fa80b92d1e0/ash-examples/src/lib.rs#L229 and following lines
    // - https://stackoverflow.com/questions/58732459/vk-error-incompatible-driver-with-mac-os-and-vulkan-moltenvk
    extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
    extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
    let flags = vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;

    let layer_names = vec![VALIDATION_LAYER_NAME.as_ptr()];

    extension_names.push(vk::EXT_DEBUG_UTILS_NAME.as_ptr());

    let instance_create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names)
        .enabled_layer_names(&layer_names)
        .flags(flags);

    let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

    Ok(instance)
}

fn create_debug_utils_and_messenger(
    entry: &Entry,
    instance: &Instance,
) -> Result<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT), vk::Result> {
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

    let debug_utils_messenger =
        unsafe { debug_utils.create_debug_utils_messenger(&debugcreateinfo, None)? };
    Ok((debug_utils, debug_utils_messenger))
}

fn create_physical_device(instance: &Instance) -> Result<vk::PhysicalDevice, vk::Result> {
    let phys_devs = unsafe { instance.enumerate_physical_devices()? };
    let physical_device = phys_devs[0];

    Ok(physical_device)
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

struct QueueFamilyIndices {
    graphics: u32,
    transfer: u32,
}

fn get_queue_family_indices(
    instance: &Instance,
    physical_device: &vk::PhysicalDevice,
) -> QueueFamilyIndices {
    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };
    dbg!(&queue_family_properties);
    let mut found_graphics_queue_family_indices: Vec<u32> = Vec::new();
    let mut found_transfer_queue_family_indices: Vec<u32> = Vec::new();
    for (index, queue_family_property) in queue_family_properties.iter().enumerate() {
        if queue_family_property.queue_count > 0
            && queue_family_property
                .queue_flags
                .contains(vk::QueueFlags::GRAPHICS)
        {
            found_graphics_queue_family_indices.push(index as u32);
        }
        if queue_family_property.queue_count > 0
            && queue_family_property
                .queue_flags
                .contains(vk::QueueFlags::TRANSFER)
        {
            found_transfer_queue_family_indices.push(index as u32);
        }
    }
    let queue_family_indices = QueueFamilyIndices {
        graphics: found_graphics_queue_family_indices[0],
        transfer: found_transfer_queue_family_indices[0],
    };

    queue_family_indices
}
