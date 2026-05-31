#include "vulkan/instance.hpp"

#include <cstring>
#include <vector>

#define GLFW_INCLUDE_VULKAN
#include <GLFW/glfw3.h>

#include "vulkan/vk_check.hpp"

VulkanInstance::~VulkanInstance() {
    if (instance_ != VK_NULL_HANDLE) {
        if (surface_created_) {
            vkDestroySurfaceKHR(instance_, surface_, nullptr);
        }
        vkDestroyInstance(instance_, nullptr);
    }
}

void VulkanInstance::create(GLFWwindow* window) {
    uint32_t ext_count = 0;
    const char** glfw_ext = glfwGetRequiredInstanceExtensions(&ext_count);
    std::vector<const char*> extensions(glfw_ext, glfw_ext + ext_count);

#if defined(__APPLE__)
    extensions.push_back(VK_KHR_PORTABILITY_ENUMERATION_EXTENSION_NAME);
#endif

    VkApplicationInfo app_info{};
    app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    app_info.pApplicationName = "upskill_cpp";
    app_info.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.pEngineName = "upskill_cpp";
    app_info.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.apiVersion = VK_API_VERSION_1_2;

    VkInstanceCreateInfo create_info{};
    create_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    create_info.pApplicationInfo = &app_info;
    create_info.enabledExtensionCount = static_cast<uint32_t>(extensions.size());
    create_info.ppEnabledExtensionNames = extensions.data();
#if defined(__APPLE__)
    create_info.flags |= VK_INSTANCE_CREATE_ENUMERATE_PORTABILITY_BIT_KHR;
#endif

    vk_check(vkCreateInstance(&create_info, nullptr, &instance_), "vkCreateInstance");

    vk_check(glfwCreateWindowSurface(instance_, window, nullptr, &surface_),
             "glfwCreateWindowSurface");
    surface_created_ = true;
}
