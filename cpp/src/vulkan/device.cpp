#include "vulkan/device.hpp"

#include <set>
#include <stdexcept>
#include <vector>

#define VMA_IMPLEMENTATION
#include "vk_mem_alloc.h"

#include "vulkan/vk_check.hpp"

VulkanDevice::~VulkanDevice() {
    if (device_ != VK_NULL_HANDLE) {
        vkDeviceWaitIdle(device_);
        if (command_pool_ != VK_NULL_HANDLE) {
            vkDestroyCommandPool(device_, command_pool_, nullptr);
        }
        if (allocator_ != VK_NULL_HANDLE) {
            vmaDestroyAllocator(allocator_);
        }
        vkDestroyDevice(device_, nullptr);
    }
}

namespace {

QueueFamilies find_queue_families(VkPhysicalDevice pd, VkSurfaceKHR surface) {
    uint32_t count = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(pd, &count, nullptr);
    std::vector<VkQueueFamilyProperties> props(count);
    vkGetPhysicalDeviceQueueFamilyProperties(pd, &count, props.data());

    QueueFamilies fam;
    bool has_graphics = false;
    bool has_present = false;
    for (uint32_t i = 0; i < count; ++i) {
        if (props[i].queueFlags & VK_QUEUE_GRAPHICS_BIT) {
            fam.graphics = i;
            has_graphics = true;
        }
        VkBool32 present_support = VK_FALSE;
        vkGetPhysicalDeviceSurfaceSupportKHR(pd, i, surface, &present_support);
        if (present_support) {
            fam.present = i;
            has_present = true;
        }
    }
    if (!has_graphics || !has_present) {
        throw std::runtime_error("无支持图形与呈现的队列族");
    }
    return fam;
}

}  // namespace

void VulkanDevice::create(VkInstance instance, VkSurfaceKHR surface, VkPhysicalDevice& out_pd) {
    uint32_t pd_count = 0;
    vkEnumeratePhysicalDevices(instance, &pd_count, nullptr);
    if (pd_count == 0) {
        throw std::runtime_error("未找到 Vulkan 物理设备");
    }
    std::vector<VkPhysicalDevice> pds(pd_count);
    vkEnumeratePhysicalDevices(instance, &pd_count, pds.data());

    pd_ = VK_NULL_HANDLE;
    families_ = {};
    for (VkPhysicalDevice pd : pds) {
        try {
            families_ = find_queue_families(pd, surface);
            pd_ = pd;
            break;
        } catch (...) {
        }
    }
    if (pd_ == VK_NULL_HANDLE) {
        throw std::runtime_error("无支持图形与呈现的 Vulkan 设备");
    }
    out_pd = pd_;

    float priority = 1.0f;
    std::set<uint32_t> unique = {families_.graphics, families_.present};
    std::vector<VkDeviceQueueCreateInfo> queue_infos;
    for (uint32_t qf : unique) {
        VkDeviceQueueCreateInfo qi{};
        qi.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
        qi.queueFamilyIndex = qf;
        qi.queueCount = 1;
        qi.pQueuePriorities = &priority;
        queue_infos.push_back(qi);
    }

    const char* dev_ext[] = {VK_KHR_SWAPCHAIN_EXTENSION_NAME};
    VkDeviceCreateInfo dev_info{};
    dev_info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    dev_info.queueCreateInfoCount = static_cast<uint32_t>(queue_infos.size());
    dev_info.pQueueCreateInfos = queue_infos.data();
    dev_info.enabledExtensionCount = 1;
    dev_info.ppEnabledExtensionNames = dev_ext;

    vk_check(vkCreateDevice(pd_, &dev_info, nullptr, &device_), "vkCreateDevice");

    vkGetDeviceQueue(device_, families_.graphics, 0, &graphics_queue_);
    vkGetDeviceQueue(device_, families_.present, 0, &present_queue_);

    VkCommandPoolCreateInfo pool_info{};
    pool_info.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
    pool_info.flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT;
    pool_info.queueFamilyIndex = families_.graphics;
    vk_check(vkCreateCommandPool(device_, &pool_info, nullptr, &command_pool_), "command pool");

    VmaAllocatorCreateInfo alloc_info{};
    alloc_info.physicalDevice = pd_;
    alloc_info.device = device_;
    alloc_info.instance = instance;
    vk_check(vmaCreateAllocator(&alloc_info, &allocator_), "vmaCreateAllocator");
}

void VulkanDevice::create_device_buffer(VkDeviceSize size, VkBufferUsageFlags usage, VkBuffer& buffer,
                                        VmaAllocation& alloc) {
    VkBufferCreateInfo buf_info{};
    buf_info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    buf_info.size = size;
    buf_info.usage = usage;
    buf_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;

    VmaAllocationCreateInfo alloc_create{};
    alloc_create.usage = VMA_MEMORY_USAGE_AUTO_PREFER_DEVICE;

    vk_check(vmaCreateBuffer(allocator_, &buf_info, &alloc_create, &buffer, &alloc, nullptr),
             "vmaCreateBuffer device");
}

void VulkanDevice::create_staging_buffer(VkDeviceSize size, VkBuffer& buffer, VmaAllocation& alloc) {
    VkBufferCreateInfo buf_info{};
    buf_info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
    buf_info.size = size;
    buf_info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
    buf_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;

    VmaAllocationCreateInfo alloc_create{};
    alloc_create.usage = VMA_MEMORY_USAGE_AUTO;
    alloc_create.flags = VMA_ALLOCATION_CREATE_HOST_ACCESS_SEQUENTIAL_WRITE_BIT;

    vk_check(vmaCreateBuffer(allocator_, &buf_info, &alloc_create, &buffer, &alloc, nullptr),
             "vmaCreateBuffer staging");
}

void* VulkanDevice::map_allocation(VmaAllocation alloc) const {
    void* data = nullptr;
    vk_check(vmaMapMemory(allocator_, alloc, &data), "vmaMapMemory");
    return data;
}
