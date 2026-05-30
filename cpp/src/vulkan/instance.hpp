#pragma once

#include <vulkan/vulkan.h>

struct GLFWwindow;

/// Vulkan 实例与 Surface 创建。
class VulkanInstance {
public:
    VulkanInstance() = default;
    ~VulkanInstance();

    VulkanInstance(const VulkanInstance&) = delete;
    VulkanInstance& operator=(const VulkanInstance&) = delete;

    /// 创建实例并绑定 GLFW 窗口 Surface。
    void create(GLFWwindow* window);

    VkInstance instance() const { return instance_; }
    VkSurfaceKHR surface() const { return surface_; }

private:
    VkInstance instance_ = VK_NULL_HANDLE;
    VkSurfaceKHR surface_ = VK_NULL_HANDLE;
    bool surface_created_ = false;
};
