#include "vulkan/swapchain.hpp"

#include <algorithm>
#include <stdexcept>

#include "vulkan/vk_check.hpp"

SwapchainContext::~SwapchainContext() {
    // destroy() 应由调用方在 device 销毁前显式调用
}

void SwapchainContext::create(VkInstance instance, VulkanDevice& dev, VkPhysicalDevice pd,
                              VkSurfaceKHR surface, uint32_t width, uint32_t height) {
    device_ = dev.device();
    vkGetDeviceProcAddr(device_, "vkDestroySwapchainKHR",
                        reinterpret_cast<PFN_vkVoidFunction*>(&destroy_swapchain_));
    vkGetDeviceProcAddr(device_, "vkAcquireNextImageKHR",
                        reinterpret_cast<PFN_vkVoidFunction*>(&acquire_next_image_));
    vkGetDeviceProcAddr(device_, "vkQueuePresentKHR",
                        reinterpret_cast<PFN_vkVoidFunction*>(&queue_present_));

    VkSurfaceCapabilitiesKHR caps{};
    vkGetPhysicalDeviceSurfaceCapabilitiesKHR(pd, surface, &caps);

    uint32_t fmt_count = 0;
    vkGetPhysicalDeviceSurfaceFormatsKHR(pd, surface, &fmt_count, nullptr);
    std::vector<VkSurfaceFormatKHR> formats(fmt_count);
    vkGetPhysicalDeviceSurfaceFormatsKHR(pd, surface, &fmt_count, formats.data());

    format_ = formats[0].format;
    VkColorSpaceKHR color_space = formats[0].colorSpace;
    for (const auto& f : formats) {
        if (f.format == VK_FORMAT_B8G8R8A8_SRGB &&
            f.colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR) {
            format_ = f.format;
            color_space = f.colorSpace;
            break;
        }
    }

    extent_.width =
        std::clamp(width, caps.minImageExtent.width, caps.maxImageExtent.width);
    extent_.height =
        std::clamp(height, caps.minImageExtent.height, caps.maxImageExtent.height);

    uint32_t image_count = caps.minImageCount + 1;
    if (caps.maxImageCount > 0 && image_count > caps.maxImageCount) {
        image_count = caps.maxImageCount;
    }

    VkSwapchainCreateInfoKHR sci{};
    sci.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    sci.surface = surface;
    sci.minImageCount = image_count;
    sci.imageFormat = format_;
    sci.imageColorSpace = color_space;
    sci.imageExtent = extent_;
    sci.imageArrayLayers = 1;
    sci.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    sci.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    sci.preTransform = caps.currentTransform;
    sci.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    sci.presentMode = VK_PRESENT_MODE_FIFO_KHR;
    sci.clipped = VK_TRUE;
    sci.oldSwapchain = VK_NULL_HANDLE;

    vk_check(vkCreateSwapchainKHR(device_, &sci, nullptr, &swapchain_), "create swapchain");

    uint32_t sc_count = 0;
    vkGetSwapchainImagesKHR(device_, swapchain_, &sc_count, nullptr);
    images_.resize(sc_count);
    vkGetSwapchainImagesKHR(device_, swapchain_, &sc_count, images_.data());

    image_views_.resize(images_.size());
    for (std::size_t i = 0; i < images_.size(); ++i) {
        VkImageViewCreateInfo vi{};
        vi.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
        vi.image = images_[i];
        vi.viewType = VK_IMAGE_VIEW_TYPE_2D;
        vi.format = format_;
        vi.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
        vi.subresourceRange.levelCount = 1;
        vi.subresourceRange.layerCount = 1;
        vk_check(vkCreateImageView(device_, &vi, nullptr, &image_views_[i]), "swap image view");
    }

    VkImageCreateInfo msaa_info{};
    msaa_info.sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO;
    msaa_info.imageType = VK_IMAGE_TYPE_2D;
    msaa_info.format = format_;
    msaa_info.extent = {extent_.width, extent_.height, 1};
    msaa_info.mipLevels = 1;
    msaa_info.arrayLayers = 1;
    msaa_info.samples = kMsaaSamples;
    msaa_info.tiling = VK_IMAGE_TILING_OPTIMAL;
    msaa_info.usage =
        VK_IMAGE_USAGE_TRANSIENT_ATTACHMENT_BIT | VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    msaa_info.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;

    VmaAllocator alloc = dev.allocator();
    VmaAllocationCreateInfo msaa_alloc_info{};
    msaa_alloc_info.usage = VMA_MEMORY_USAGE_GPU_ONLY;
    vk_check(vmaCreateImage(alloc, &msaa_info, &msaa_alloc_info, &msaa_image_, &msaa_alloc_,
                            nullptr),
             "msaa image");

    VkImageViewCreateInfo msaa_view_info{};
    msaa_view_info.sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO;
    msaa_view_info.image = msaa_image_;
    msaa_view_info.viewType = VK_IMAGE_VIEW_TYPE_2D;
    msaa_view_info.format = format_;
    msaa_view_info.subresourceRange.aspectMask = VK_IMAGE_ASPECT_COLOR_BIT;
    msaa_view_info.subresourceRange.levelCount = 1;
    msaa_view_info.subresourceRange.layerCount = 1;
    vk_check(vkCreateImageView(device_, &msaa_view_info, nullptr, &msaa_view_), "msaa view");

  {
        VkAttachmentDescription color{};
        color.format = format_;
        color.samples = kMsaaSamples;
        color.loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR;
        color.storeOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
        color.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        color.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
        color.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
        color.finalLayout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        VkAttachmentDescription resolve{};
        resolve.format = format_;
        resolve.samples = VK_SAMPLE_COUNT_1_BIT;
        resolve.loadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        resolve.storeOp = VK_ATTACHMENT_STORE_OP_STORE;
        resolve.stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE;
        resolve.stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE;
        resolve.initialLayout = VK_IMAGE_LAYOUT_UNDEFINED;
        resolve.finalLayout = VK_IMAGE_LAYOUT_PRESENT_SRC_KHR;

        VkAttachmentReference color_ref{};
        color_ref.attachment = 0;
        color_ref.layout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        VkAttachmentReference resolve_ref{};
        resolve_ref.attachment = 1;
        resolve_ref.layout = VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL;

        VkSubpassDescription subpass{};
        subpass.pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS;
        subpass.colorAttachmentCount = 1;
        subpass.pColorAttachments = &color_ref;
        subpass.pResolveAttachments = &resolve_ref;

        VkSubpassDependency dep{};
        dep.srcSubpass = VK_SUBPASS_EXTERNAL;
        dep.dstSubpass = 0;
        dep.srcStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
        dep.dstStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
        dep.dstAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT;

        VkAttachmentDescription attachments[] = {color, resolve};
        VkRenderPassCreateInfo rp_info{};
        rp_info.sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO;
        rp_info.attachmentCount = 2;
        rp_info.pAttachments = attachments;
        rp_info.subpassCount = 1;
        rp_info.pSubpasses = &subpass;
        rp_info.dependencyCount = 1;
        rp_info.pDependencies = &dep;
        vk_check(vkCreateRenderPass(device_, &rp_info, nullptr, &render_pass_), "render pass");
    }

    framebuffers_.resize(image_views_.size());
    for (std::size_t i = 0; i < image_views_.size(); ++i) {
        VkImageView attachments[] = {msaa_view_, image_views_[i]};
        VkFramebufferCreateInfo fb{};
        fb.sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO;
        fb.renderPass = render_pass_;
        fb.attachmentCount = 2;
        fb.pAttachments = attachments;
        fb.width = extent_.width;
        fb.height = extent_.height;
        fb.layers = 1;
        vk_check(vkCreateFramebuffer(device_, &fb, nullptr, &framebuffers_[i]), "framebuffer");
    }
    (void)instance;
}

void SwapchainContext::destroy(VulkanDevice& dev) {
    VkDevice d = dev.device();
    for (auto fb : framebuffers_) {
        vkDestroyFramebuffer(d, fb, nullptr);
    }
    framebuffers_.clear();
    if (render_pass_) {
        vkDestroyRenderPass(d, render_pass_, nullptr);
        render_pass_ = VK_NULL_HANDLE;
    }
    if (msaa_view_) {
        vkDestroyImageView(d, msaa_view_, nullptr);
        msaa_view_ = VK_NULL_HANDLE;
    }
    if (msaa_image_) {
        vmaDestroyImage(dev.allocator(), msaa_image_, msaa_alloc_);
        msaa_image_ = VK_NULL_HANDLE;
        msaa_alloc_ = VK_NULL_HANDLE;
    }
    for (auto view : image_views_) {
        vkDestroyImageView(d, view, nullptr);
    }
    image_views_.clear();
    if (swapchain_ && destroy_swapchain_) {
        destroy_swapchain_(d, swapchain_, nullptr);
        swapchain_ = VK_NULL_HANDLE;
    }
}

void SwapchainContext::acquire_next(VkDevice device, VkSemaphore sem, uint32_t& image_index) {
    vk_check(acquire_next_image_(device, swapchain_, UINT64_MAX, sem, VK_NULL_HANDLE, &image_index),
             "acquire next image");
}

void SwapchainContext::present(VkDevice device, VkQueue queue, VkSemaphore wait_sem,
                               uint32_t image_index) {
    VkPresentInfoKHR pi{};
    pi.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    pi.waitSemaphoreCount = 1;
    pi.pWaitSemaphores = &wait_sem;
    pi.swapchainCount = 1;
    pi.pSwapchains = &swapchain_;
    pi.pImageIndices = &image_index;
    vk_check(queue_present_(queue, &pi), "queue present");
}
