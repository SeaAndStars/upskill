//! Swapchain、MSAA 图像与 RenderPass。

use ash::vk;

use super::device::VulkanDevice;
use super::instance::VulkanInstance;

/// MSAA 采样数。
pub const MSAA_SAMPLES: vk::SampleCountFlags = vk::SampleCountFlags::TYPE_4;

/// Swapchain 与帧缓冲相关资源。
pub struct SwapchainContext {
    /// Swapchain 扩展。
    pub swapchain_loader: ash::khr::swapchain::Device,
    /// Swapchain 句柄。
    pub swapchain: vk::SwapchainKHR,
    /// 表面格式。
    pub format: vk::SurfaceFormatKHR,
    ///  extent。
    pub extent: vk::Extent2D,
    /// 交换链图像。
    pub images: Vec<vk::Image>,
    /// 图像视图。
    pub image_views: Vec<vk::ImageView>,
    /// MSAA 颜色图。
    pub msaa_image: vk::Image,
    /// MSAA 视图。
    pub msaa_view: vk::ImageView,
    /// MSAA 分配（由 device 管理释放需在外部 free）。
    pub msaa_allocation: Option<gpu_allocator::vulkan::Allocation>, // take() on destroy
    /// RenderPass。
    pub render_pass: vk::RenderPass,
    /// 每 swapchain 图像的 framebuffer。
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl SwapchainContext {
    /// 创建 swapchain、MSAA 与 render pass。
    pub fn new(
        inst: &VulkanInstance,
        dev: &mut VulkanDevice,
        pd: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let caps = unsafe {
            inst.surface_loader
                .get_physical_device_surface_capabilities(pd, surface)
                .map_err(|e| format!("surface capabilities: {e}"))?
        };
        let formats = unsafe {
            inst.surface_loader
                .get_physical_device_surface_formats(pd, surface)
                .map_err(|e| format!("surface formats: {e}"))?
        };
        let format = formats
            .iter()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .copied()
            .unwrap_or(formats[0]);

        let extent = vk::Extent2D {
            width: width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        };

        let image_count = (caps.min_image_count + 1).min(if caps.max_image_count > 0 {
            caps.max_image_count
        } else {
            u32::MAX
        });

        let queue_indices = [dev.families.graphics, dev.families.present];
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null())
            .queue_family_indices(&queue_indices);

        let swapchain_loader = ash::khr::swapchain::Device::new(&inst.instance, &dev.device);
        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None) }
            .map_err(|e| format!("create_swapchain: {e}"))?;

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }
            .map_err(|e| format!("get_swapchain_images: {e}"))?;

        let image_views: Result<Vec<_>, _> = images
            .iter()
            .map(|img| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(*img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                unsafe { dev.device.create_image_view(&view_info, None) }
                    .map_err(|e| format!("image_view: {e}"))
            })
            .collect();
        let image_views = image_views?;

        let (msaa_image, msaa_view, msaa_allocation) =
            create_msaa_image(dev, pd, extent, format.format)?;

        let render_pass = create_render_pass(&dev.device, format.format)?;

        let framebuffers: Result<Vec<_>, _> = image_views
            .iter()
            .map(|view| {
                let attachments = [msaa_view, *view];
                let fb_info = vk::FramebufferCreateInfo::default()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(extent.width)
                    .height(extent.height)
                    .layers(1);
                unsafe { dev.device.create_framebuffer(&fb_info, None) }
                    .map_err(|e| format!("framebuffer: {e}"))
            })
            .collect();
        let framebuffers = framebuffers?;

        Ok(Self {
            swapchain_loader,
            swapchain,
            format,
            extent,
            images,
            image_views,
            msaa_image,
            msaa_view,
            msaa_allocation: Some(msaa_allocation),
            render_pass,
            framebuffers,
        })
    }

    /// 获取 framebuffer 数量。
    pub fn frame_count(&self) -> usize {
        self.framebuffers.len()
    }
}


/// 创建 MSAA 颜色附件。
fn create_msaa_image(
    dev: &mut VulkanDevice,
    pd: vk::PhysicalDevice,
    extent: vk::Extent2D,
    format: vk::Format,
) -> Result<(vk::Image, vk::ImageView, gpu_allocator::vulkan::Allocation), String> {
    let _ = pd;
    let img_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(MSAA_SAMPLES)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    let image = unsafe { dev.device.create_image(&img_info, None) }
        .map_err(|e| format!("msaa image: {e}"))?;
    let req = unsafe { dev.device.get_image_memory_requirements(image) };
    let allocation = dev
        .allocator
        .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
            name: "msaa",
            requirements: req,
            location: gpu_allocator::MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
        })
        .map_err(|e| format!("msaa alloc: {e}"))?;
    unsafe {
        dev.device
            .bind_image_memory(image, allocation.memory(), allocation.offset())
            .map_err(|e| format!("bind msaa: {e}"))?;
    }
    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    let view = unsafe { dev.device.create_image_view(&view_info, None) }
        .map_err(|e| format!("msaa view: {e}"))?;
    Ok((image, view, allocation))
}

/// 创建带 resolve 的 RenderPass。
fn create_render_pass(device: &ash::Device, format: vk::Format) -> Result<vk::RenderPass, String> {
    let color = vk::AttachmentDescription::default()
        .format(format)
        .samples(MSAA_SAMPLES)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let resolve = vk::AttachmentDescription::default()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    let resolve_ref = vk::AttachmentReference::default()
        .attachment(1)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(std::slice::from_ref(&color_ref))
        .resolve_attachments(std::slice::from_ref(&resolve_ref));

    let dependency = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    let attachments = [color, resolve];
    let rp_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(std::slice::from_ref(&subpass))
        .dependencies(std::slice::from_ref(&dependency));

    unsafe { device.create_render_pass(&rp_info, None) }.map_err(|e| format!("render_pass: {e}"))
}
