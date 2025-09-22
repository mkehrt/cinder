// Followed from https://hoj-senna.github.io/ashen-aetna/

use anyhow::anyhow;
use ash::khr::swapchain;
use ash::vk;
use ash::{Entry, Instance};
use raw_window_handle::{DisplayHandle, WindowHandle};
use std::ffi::CStr;

static ENGINE_NAME: &CStr = c"Engine";
static APP_NAME: &CStr = c"Application";

static VALIDATION_LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

pub struct Vulkan {
    entry: Entry,
    instance: Instance,
    debug_utils: ash::ext::debug_utils::Instance,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    surface_instance: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    logical_device: ash::Device,
    queues: Queues,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
}

struct Queues {
    graphics_queue: vk::Queue,
    transfer_queue: vk::Queue,
}

struct QueueFamilyIndices {
    graphics: u32,
    transfer: u32,
}

impl Vulkan {
    pub fn new(
        display_handle: &DisplayHandle,
        window_handle: &WindowHandle,
    ) -> Result<Self, anyhow::Error> {
        let entry = Entry::linked();
        let instance: Instance = Self::create_instance(display_handle, &entry)?;
        let (debug_utils, debug_utils_messenger) =
            Self::create_debug_utils_and_messenger(&entry, &instance)?;

        let surface_instance = ash::khr::surface::Instance::new(&entry, &instance);
        let surface = Self::create_surface(&entry, &instance, &display_handle, &window_handle)?;

        let physical_device: vk::PhysicalDevice = Self::create_physical_device(&instance)?;
        let queue_family_indices = Self::get_queue_family_indices(
            &instance,
            &physical_device,
            &surface_instance,
            &surface,
        )?;

        let logical_device =
            Self::create_logcal_device(&instance, physical_device, &queue_family_indices)?;
        let queues = Self::get_queues(&logical_device, &queue_family_indices);

        let swapchain_image_views = Self::create_swapchain_image_views(
            &instance,
            &physical_device,
            &logical_device,
            &surface_instance,
            &surface,
            &queue_family_indices,
        )?;

        let render_pass = Self::create_render_pass(
            &logical_device,
            &physical_device,
            &surface_instance,
            &surface,
        )?;

        let framebuffers = Self::create_framebuffers(
            render_pass,
            &physical_device,
            &logical_device,
            &surface_instance,
            &surface,
            &swapchain_image_views,
        )?;

        Ok(Self {
            entry,
            instance,
            debug_utils,
            debug_utils_messenger,
            surface_instance,
            surface,
            logical_device,
            queues,
            swapchain_image_views,
            render_pass,
            framebuffers,
        })
    }

    fn create_instance(
        display_handle: &DisplayHandle,
        entry: &Entry,
    ) -> Result<Instance, anyhow::Error> {
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

        let raw_display_handle = display_handle.as_raw();
        let window_required_extensions =
            ash_window::enumerate_required_extensions(raw_display_handle)?;
        extension_names.extend(window_required_extensions);

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names)
            .flags(flags);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        Ok(instance)
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

    fn create_debug_utils_and_messenger(
        entry: &Entry,
        instance: &Instance,
    ) -> Result<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT), anyhow::Error> {
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
            .pfn_user_callback(Some(Self::vulkan_debug_utils_callback));

        let debug_utils_messenger =
            unsafe { debug_utils.create_debug_utils_messenger(&debugcreateinfo, None)? };
        Ok((debug_utils, debug_utils_messenger))
    }

    fn create_physical_device(instance: &Instance) -> Result<vk::PhysicalDevice, vk::Result> {
        let phys_devs = unsafe { instance.enumerate_physical_devices()? };
        let physical_device = phys_devs[0];

        Ok(physical_device)
    }

    fn get_queue_family_indices(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        surface_instance: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR,
    ) -> Result<QueueFamilyIndices, anyhow::Error> {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };
        dbg!(&queue_family_properties);
        let mut found_graphics_queue_family_indices: Vec<u32> = Vec::new();
        let mut found_transfer_queue_family_indices: Vec<u32> = Vec::new();
        for (index, queue_family_property) in queue_family_properties.iter().enumerate() {
            let surface_support = unsafe {
                surface_instance.get_physical_device_surface_support(
                    *physical_device,
                    index as u32,
                    *surface,
                )?
            };
            if queue_family_property.queue_count > 0
                && queue_family_property
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
                && surface_support
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
        let graphics_queue_family_index = found_graphics_queue_family_indices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No graphics queue family index found."))?;
        let transfer_queue_family_index = found_transfer_queue_family_indices
            .into_iter()
            // https://hoj-senna.github.io/ashen-aetna/text/005_Queues.html claims that the
            // graphics and transfer queue families should be different, but I only have one queue
            // family on my Mac.
            //.find(|index| *index != graphics_queue_family_index)
            .next()
            .ok_or_else(|| anyhow!("No valid transfer queue family index found."))?;
        let queue_family_indices = QueueFamilyIndices {
            // TODO handle errors and convert to anyhow
            graphics: graphics_queue_family_index,
            transfer: transfer_queue_family_index,
        };

        Ok(queue_family_indices)
    }

    fn create_logcal_device(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        queue_family_indices: &QueueFamilyIndices,
    ) -> Result<ash::Device, anyhow::Error> {
        let priorities = [1.0f32];

        let graphics_queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_indices.graphics)
            .queue_priorities(&priorities);
        let transfer_queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_indices.transfer)
            .queue_priorities(&priorities);
        let queue_infos = vec![graphics_queue_info, transfer_queue_info];

        let extension_names = vec![
            ash::khr::swapchain::NAME.as_ptr(),
            ash::khr::portability_subset::NAME.as_ptr(),
        ];

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&extension_names);

        let logical_device =
            unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        Ok(logical_device)
    }

    fn get_queues(
        logical_device: &ash::Device,
        queue_family_indices: &QueueFamilyIndices,
    ) -> Queues {
        let graphics_queue =
            unsafe { logical_device.get_device_queue(queue_family_indices.graphics, 0) };
        let transfer_queue =
            unsafe { logical_device.get_device_queue(queue_family_indices.transfer, 0) };

        dbg!(&graphics_queue);
        dbg!(&transfer_queue);

        Queues {
            graphics_queue,
            transfer_queue,
        }
    }

    fn create_surface(
        entry: &Entry,
        instance: &Instance,
        display_handle: &DisplayHandle,
        window_handle: &WindowHandle,
    ) -> Result<vk::SurfaceKHR, anyhow::Error> {
        let raw_display_handle = display_handle.as_raw();
        let raw_window_handle = window_handle.as_raw();

        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )?
        };

        Ok(surface)
    }

    fn get_surface_format(
        surface_instance: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR,
        physical_device: &vk::PhysicalDevice,
    ) -> Result<vk::SurfaceFormatKHR, anyhow::Error> {
        let surface_formats_result = unsafe {
            surface_instance.get_physical_device_surface_formats(*physical_device, *surface)
        };
        let surface_formats =
            surface_formats_result.map_err(|err| anyhow!("No surface formats found: {}", err))?;
        let surface_format = surface_formats
            .first()
            .ok_or_else(|| anyhow!("No surface format found"))?;
        Ok(*surface_format)
    }

    fn create_swapchain_image_views(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        logical_device: &ash::Device,
        surface_instance: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR,
        queue_family_indices: &QueueFamilyIndices,
    ) -> Result<Vec<vk::ImageView>, anyhow::Error> {
        let surface_capabilities = unsafe {
            surface_instance.get_physical_device_surface_capabilities(*physical_device, *surface)
        }?;
        let surface_present_modes = unsafe {
            surface_instance.get_physical_device_surface_present_modes(*physical_device, *surface)
        }?;
        let surface_present_mode = surface_present_modes
            .first()
            .ok_or_else(|| anyhow!("No surface present mode found"))?;

        let surface_format = Self::get_surface_format(surface_instance, surface, physical_device)?;

        let queue_families = [queue_family_indices.graphics];
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(*surface)
            .min_image_count(
                3.max(surface_capabilities.min_image_count)
                    .min(surface_capabilities.max_image_count),
            )
            .present_mode(*surface_present_mode)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_families)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO);
        let swapchain_loader = swapchain::Device::new(instance, logical_device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        // https://hoj-senna.github.io/ashen-aetna/text/007_Swapchain.html keeps
        // the swapchain and loader to destroy later, but in the current version
        // of the API, get_swapchain_images takes ownership of the swapchain,
        // so I'm not sure what to do here.

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let mut swapchain_image_views = Vec::with_capacity(swapchain_images.len());
        for image in &swapchain_images {
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);
            let image_view_create_info = vk::ImageViewCreateInfo::default()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::B8G8R8A8_UNORM)
                .subresource_range(subresource_range);
            let image_view =
                unsafe { logical_device.create_image_view(&image_view_create_info, None) }?;
            swapchain_image_views.push(image_view);
        }
        Ok(swapchain_image_views)
    }

    fn create_attachments(
        physical_device: &vk::PhysicalDevice,
        surface_instance: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR,
    ) -> Result<Vec<vk::AttachmentDescription>, anyhow::Error> {
        let surface_format = Self::get_surface_format(surface_instance, surface, physical_device)?;
        let format = surface_format.format;
        let attachment = vk::AttachmentDescription::default()
            .format(format)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .samples(vk::SampleCountFlags::TYPE_1);

        let attachments = vec![attachment];
        Ok(attachments)
    }

    fn create_render_pass(
        logical_device: &ash::Device,
        physical_device: &vk::PhysicalDevice,
        surface_instance: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR,
    ) -> Result<vk::RenderPass, anyhow::Error> {
        let attachments = Self::create_attachments(physical_device, surface_instance, surface)?;
        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        let attachment_refs = vec![color_attachment_ref];
        let subpass = vk::SubpassDescription::default()
            .color_attachments(&attachment_refs)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);
        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_subpass(0)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            );
        let dependencies = vec![dependency];
        let subpasses = vec![subpass];
        let render_pass_create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        let render_pass =
            unsafe { logical_device.create_render_pass(&render_pass_create_info, None)? };
        Ok(render_pass)
    }

    fn create_framebuffers(
        render_pass: vk::RenderPass,
        physical_device: &vk::PhysicalDevice,
        logical_device: &ash::Device,
        surface_instance: &ash::khr::surface::Instance,
        surface: &vk::SurfaceKHR,
        image_views: &Vec<vk::ImageView>,
    ) -> Result<Vec<vk::Framebuffer>, vk::Result> {
        let surface_capabilities = unsafe {
            surface_instance.get_physical_device_surface_capabilities(*physical_device, *surface)
        }?;
        let extent = surface_capabilities.current_extent;
        let mut framebuffers = Vec::new();
        for image_view in image_views {
            let image_view_array = [*image_view];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&image_view_array)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            let framebuffer =
                unsafe { logical_device.create_framebuffer(&framebuffer_info, None) }?;
            framebuffers.push(framebuffer);
        }
        Ok(framebuffers)
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            let image_views = std::mem::take(&mut self.swapchain_image_views);
            for image_view in image_views {
                self.logical_device.destroy_image_view(image_view, None);
            }
            self.surface_instance.destroy_surface(self.surface, None);
            self.logical_device.destroy_device(None);
            self.logical_device
                .destroy_render_pass(self.render_pass, None);
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}
