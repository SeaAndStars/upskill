#pragma once

#include <stdexcept>
#include <string>

/// Vulkan 调用结果检查。
inline void vk_check(VkResult r, const char* msg) {
    if (r != VK_SUCCESS) {
        throw std::runtime_error(std::string(msg) + " (VkResult=" + std::to_string(static_cast<int>(r)) + ")");
    }
}
