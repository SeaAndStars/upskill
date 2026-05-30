#pragma once

#include <vector>

#include <vulkan/vulkan.h>

#include "vk_mem_alloc.h"
#include "vulkan/device.hpp"

/// MSAA 采样数。
constexpr VkSampleCountFlagBits kMsaaSamples = VK_SAMPLE_COUNT_4_BIT;

/// Swapchain、MSAA 与 RenderPass。
class SwapchainContext {
public:
    SwapchainContext() = default;
    ~SwapchainContext();

    SwapchainContext(const SwapchainContext&) = delete;
    SwapchainContext& operator=(const SwapchainContext&) = delete;

    void create(VkInstance instance, VulkanDevice& dev, VkPhysicalDevice pd, VkSurfaceKHR surface,
                uint32_t width, uint32_t height);

    void destroy(VulkanDevice& dev);

    VkSwapchainKHR swapchain() const { return swapchain_; }
    VkFormat format() const { return format_; }
    VkExtent2D extent() const { return extent_; }
    VkRenderPass render_pass() const { return render_pass_; }
    VkFramebuffer framebuffer(std::size_t i) const { return framebuffers_[i]; }
    std::size_t image_count() const { return framebuffers_.size(); }

    void acquire_next(VkDevice device, VkSemaphore sem, uint32_t& image_index);
    void present(VkDevice device, VkQueue queue, VkSemaphore wait_sem, uint32_t image_index);

private:
    VkDevice device_ = VK_NULL_HANDLE;
    PFN_vkDestroySwapchainKHR destroy_swapchain_ = nullptr;
    PFN_vkAcquireNextImageKHR acquire_next_image_ = nullptr;
    PFN_vkQueuePresentKHR queue_present_ = nullptr;

    VkSwapchainKHR swapchain_ = VK_NULL_HANDLE;
    VkFormat format_ = VK_FORMAT_UNDEFINED;
    VkExtent2D extent_{};
    std::vector<VkImage> images_;
    std::vector<VkImageView> image_views_;
    VkImage msaa_image_ = VK_NULL_HANDLE;
    VkImageView msaa_view_ = VK_NULL_HANDLE;
    VmaAllocation msaa_alloc_ = VK_NULL_HANDLE;
    VkRenderPass render_pass_ = VK_NULL_HANDLE;
    std::vector<VkFramebuffer> framebuffers_;
};
