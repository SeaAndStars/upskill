#pragma once

#include <vulkan/vulkan.h>

#include "vk_mem_alloc.h"

struct QueueFamilies {
    uint32_t graphics = 0;  ///< 图形队列族。
    uint32_t present = 0;   ///< 呈现队列族。
};

/// 逻辑设备、队列、VMA 与命令池。
class VulkanDevice {
public:
    VulkanDevice() = default;
    ~VulkanDevice();

    VulkanDevice(const VulkanDevice&) = delete;
    VulkanDevice& operator=(const VulkanDevice&) = delete;

    /// 选择物理设备并创建逻辑设备。
    void create(VkInstance instance, VkSurfaceKHR surface, VkPhysicalDevice& out_pd);

    VkDevice device() const { return device_; }
    VkQueue graphics_queue() const { return graphics_queue_; }
    VkQueue present_queue() const { return present_queue_; }
    VkCommandPool command_pool() const { return command_pool_; }
    VmaAllocator allocator() const { return allocator_; }
    const QueueFamilies& families() const { return families_; }

    /// 设备本地顶点/索引缓冲。
    void create_device_buffer(VkDeviceSize size, VkBufferUsageFlags usage, VkBuffer& buffer,
                              VmaAllocation& alloc);

    /// 可映射 staging 缓冲。
    void create_staging_buffer(VkDeviceSize size, VkBuffer& buffer, VmaAllocation& alloc);

    void* map_allocation(VmaAllocation alloc) const;

private:
    VkPhysicalDevice pd_ = VK_NULL_HANDLE;
    VkDevice device_ = VK_NULL_HANDLE;
    VkQueue graphics_queue_ = VK_NULL_HANDLE;
    VkQueue present_queue_ = VK_NULL_HANDLE;
    VkCommandPool command_pool_ = VK_NULL_HANDLE;
    VmaAllocator allocator_ = VK_NULL_HANDLE;
    QueueFamilies families_;
};
