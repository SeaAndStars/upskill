#pragma once

#include <vector>

#include <vulkan/vulkan.h>

#include "vulkan/swapchain.hpp"

/// 线段与圆点双管线。
class Pipelines {
public:
    Pipelines() = default;
    ~Pipelines();

    Pipelines(const Pipelines&) = delete;
    Pipelines& operator=(const Pipelines&) = delete;

    void create(VkDevice device, const SwapchainContext& swap);
    void destroy(VkDevice device);

    VkPipeline line() const { return line_; }
    VkPipeline point() const { return point_; }
    VkPipelineLayout layout() const { return layout_; }

private:
    VkShaderModule load_spv(VkDevice device, const char* path);

    VkPipeline line_ = VK_NULL_HANDLE;
    VkPipeline point_ = VK_NULL_HANDLE;
    VkPipelineLayout layout_ = VK_NULL_HANDLE;
    VkShaderModule vert_ = VK_NULL_HANDLE;
    VkShaderModule line_frag_ = VK_NULL_HANDLE;
    VkShaderModule point_frag_ = VK_NULL_HANDLE;
};
