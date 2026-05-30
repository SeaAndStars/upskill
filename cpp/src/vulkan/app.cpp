#include "vulkan/app.hpp"

#include <chrono>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

#include <GLFW/glfw3.h>

#include "frame_geom.hpp"
#include "vulkan/device.hpp"
#include "vulkan/instance.hpp"
#include "vulkan/pipeline.hpp"
#include "vulkan/swapchain.hpp"
#include "vulkan/vk_check.hpp"

namespace {

constexpr uint64_t kFrameMs = 16;
constexpr int kMaxFramesInFlight = 2;
constexpr double kOmegaY = 0.7;
constexpr double kOmegaZ = 0.5;

/// 单帧同步对象。
struct FrameSync {
    VkSemaphore image_available = VK_NULL_HANDLE;
    VkSemaphore render_finished = VK_NULL_HANDLE;
    VkFence fence = VK_NULL_HANDLE;
};

/// GPU 网格（顶点+索引缓冲）。
struct MeshGpu {
    VkBuffer vertex_buffer = VK_NULL_HANDLE;
    VkBuffer index_buffer = VK_NULL_HANDLE;
    uint32_t index_count = 0;
    VmaAllocation vertex_alloc = VK_NULL_HANDLE;
    VmaAllocation index_alloc = VK_NULL_HANDLE;
    VkBuffer staging_vertex = VK_NULL_HANDLE;
    VkBuffer staging_index = VK_NULL_HANDLE;
    VmaAllocation staging_vertex_alloc = VK_NULL_HANDLE;
    VmaAllocation staging_index_alloc = VK_NULL_HANDLE;

    void upload(VulkanDevice& dev, VkCommandBuffer cmd, const std::vector<Vertex>& verts,
                const std::vector<uint32_t>& indices) {
        if (verts.empty() || indices.empty()) {
            return;
        }
        index_count = static_cast<uint32_t>(indices.size());
        VkDeviceSize v_size = sizeof(Vertex) * verts.size();
        VkDeviceSize i_size = sizeof(uint32_t) * indices.size();

        dev.create_staging_buffer(v_size, staging_vertex, staging_vertex_alloc);
        std::memcpy(dev.map_allocation(staging_vertex_alloc), verts.data(),
                    static_cast<std::size_t>(v_size));
        vmaUnmapMemory(dev.allocator(), staging_vertex_alloc);

        dev.create_staging_buffer(i_size, staging_index, staging_index_alloc);
        std::memcpy(dev.map_allocation(staging_index_alloc), indices.data(),
                    static_cast<std::size_t>(i_size));
        vmaUnmapMemory(dev.allocator(), staging_index_alloc);

        dev.create_device_buffer(v_size, VK_BUFFER_USAGE_TRANSFER_DST_BIT | VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
                                 vertex_buffer, vertex_alloc);
        dev.create_device_buffer(i_size, VK_BUFFER_USAGE_TRANSFER_DST_BIT | VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
                                 index_buffer, index_alloc);

        VkBufferCopy c_v{};
        c_v.size = v_size;
        vkCmdCopyBuffer(cmd, staging_vertex, vertex_buffer, 1, &c_v);
        VkBufferCopy c_i{};
        c_i.size = i_size;
        vkCmdCopyBuffer(cmd, staging_index, index_buffer, 1, &c_i);

        VkBufferMemoryBarrier barriers[2]{};
        barriers[0].sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER;
        barriers[0].srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
        barriers[0].dstAccessMask = VK_ACCESS_VERTEX_ATTRIBUTE_READ_BIT;
        barriers[0].buffer = vertex_buffer;
        barriers[0].size = v_size;
        barriers[1].sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER;
        barriers[1].srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
        barriers[1].dstAccessMask = VK_ACCESS_INDEX_READ_BIT;
        barriers[1].buffer = index_buffer;
        barriers[1].size = i_size;
        vkCmdPipelineBarrier(cmd, VK_PIPELINE_STAGE_TRANSFER_BIT,
                             VK_PIPELINE_STAGE_VERTEX_INPUT_BIT, 0, 0, nullptr, 2, barriers, 0,
                             nullptr);
    }

    void draw(VulkanDevice& dev, VkCommandBuffer cmd) const {
        if (index_count == 0) {
            return;
        }
        VkDeviceSize offset = 0;
        vkCmdBindVertexBuffers(cmd, 0, 1, &vertex_buffer, &offset);
        vkCmdBindIndexBuffer(cmd, index_buffer, 0, VK_INDEX_TYPE_UINT32);
        vkCmdDrawIndexed(cmd, index_count, 1, 0, 0, 0);
    }

    void destroy(VulkanDevice& dev) {
        VkDevice d = dev.device();
        if (vertex_buffer) {
            vmaDestroyBuffer(dev.allocator(), vertex_buffer, vertex_alloc);
            vertex_buffer = VK_NULL_HANDLE;
        }
        if (index_buffer) {
            vmaDestroyBuffer(dev.allocator(), index_buffer, index_alloc);
            index_buffer = VK_NULL_HANDLE;
        }
        if (staging_vertex) {
            vmaDestroyBuffer(dev.allocator(), staging_vertex, staging_vertex_alloc);
            staging_vertex = VK_NULL_HANDLE;
        }
        if (staging_index) {
            vmaDestroyBuffer(dev.allocator(), staging_index, staging_index_alloc);
            staging_index = VK_NULL_HANDLE;
        }
        (void)d;
        index_count = 0;
    }
};

void record_frame(VulkanDevice& dev, VkCommandBuffer cmd, SwapchainContext& swap,
                  Pipelines& pipes, uint32_t image_index, const FrameGeometry& geom) {
    MeshGpu line_mesh;
    MeshGpu point_mesh;

    VkCommandBufferBeginInfo begin_info{};
    begin_info.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
    begin_info.flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT;
    vk_check(vkBeginCommandBuffer(cmd, &begin_info), "vkBeginCommandBuffer");

    if (!geom.line_verts.empty()) {
        line_mesh.upload(dev, cmd, geom.line_verts, geom.line_indices);
    }
    if (!geom.point_verts.empty()) {
        point_mesh.upload(dev, cmd, geom.point_verts, geom.point_indices);
    }

    VkClearValue clear{};
    clear.color = {{0.063f, 0.063f, 0.094f, 1.0f}};

    VkRenderPassBeginInfo rp{};
    rp.sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO;
    rp.renderPass = swap.render_pass();
    rp.framebuffer = swap.framebuffer(image_index);
    rp.renderArea.offset = {0, 0};
    rp.renderArea.extent = swap.extent();
    rp.clearValueCount = 1;
    rp.pClearValues = &clear;

    vkCmdBeginRenderPass(cmd, &rp, VK_SUBPASS_CONTENTS_INLINE);

    VkViewport vp{};
    vp.width = static_cast<float>(swap.extent().width);
    vp.height = static_cast<float>(swap.extent().height);
    vp.maxDepth = 1.0f;
    VkRect2D sc{{0, 0}, swap.extent()};
    vkCmdSetViewport(cmd, 0, 1, &vp);
    vkCmdSetScissor(cmd, 0, 1, &sc);

    vkCmdBindPipeline(cmd, VK_PIPELINE_BIND_POINT_GRAPHICS, pipes.line());
    line_mesh.draw(dev, cmd);
    vkCmdBindPipeline(cmd, VK_PIPELINE_BIND_POINT_GRAPHICS, pipes.point());
    point_mesh.draw(dev, cmd);

    vkCmdEndRenderPass(cmd);
    vkEndCommandBuffer(cmd);

    line_mesh.destroy(dev);
    point_mesh.destroy(dev);
}

struct VulkanApp {
    VulkanInstance inst;
    VulkanDevice dev;
    SwapchainContext swap;
    Pipelines pipes;
    std::vector<FrameSync> sync;
    std::vector<VkCommandBuffer> cmd_bufs;
    int frame_index = 0;

    void init(GLFWwindow* window, const Question& q) {
        inst.create(window);
        VkPhysicalDevice pd = VK_NULL_HANDLE;
        dev.create(inst.instance(), inst.surface(), pd);
        swap.create(inst.instance(), dev, pd, inst.surface(),
                    static_cast<uint32_t>(q.width > 0 ? q.width : 1),
                    static_cast<uint32_t>(q.height > 0 ? q.height : 1));
        pipes.create(dev.device(), swap);

        sync.resize(kMaxFramesInFlight);
        for (auto& s : sync) {
            VkSemaphoreCreateInfo sem{};
            sem.sType = VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO;
            vkCreateSemaphore(dev.device(), &sem, nullptr, &s.image_available);
            vkCreateSemaphore(dev.device(), &sem, nullptr, &s.render_finished);
            VkFenceCreateInfo fence{};
            fence.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
            fence.flags = VK_FENCE_CREATE_SIGNALED_BIT;
            vkCreateFence(dev.device(), &fence, nullptr, &s.fence);
        }

        cmd_bufs.resize(kMaxFramesInFlight);
        VkCommandBufferAllocateInfo alloc{};
        alloc.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        alloc.commandPool = dev.command_pool();
        alloc.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        alloc.commandBufferCount = kMaxFramesInFlight;
        vkAllocateCommandBuffers(dev.device(), &alloc, cmd_bufs.data());
    }

    void draw_frame(const Question& q, double angle_y, double angle_z) {
        auto& s = sync[static_cast<std::size_t>(frame_index)];
        vkWaitForFences(dev.device(), 1, &s.fence, VK_TRUE, UINT64_MAX);
        vkResetFences(dev.device(), 1, &s.fence);

        uint32_t image_index = 0;
        swap.acquire_next(dev.device(), s.image_available, image_index);

        FrameGeometry geom = build_frame_geometry(q, angle_y, angle_z);
        VkCommandBuffer cmd = cmd_bufs[static_cast<std::size_t>(frame_index)];
        vkResetCommandBuffer(cmd, 0);
        record_frame(dev, cmd, swap, pipes, image_index, geom);

        VkPipelineStageFlags wait_stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
        VkSubmitInfo submit{};
        submit.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
        submit.waitSemaphoreCount = 1;
        submit.pWaitSemaphores = &s.image_available;
        submit.pWaitDstStageMask = &wait_stage;
        submit.commandBufferCount = 1;
        submit.pCommandBuffers = &cmd;
        submit.signalSemaphoreCount = 1;
        submit.pSignalSemaphores = &s.render_finished;
        vkQueueSubmit(dev.graphics_queue(), 1, &submit, s.fence);

        swap.present(dev.device(), dev.present_queue(), s.render_finished, image_index);
        frame_index = (frame_index + 1) % kMaxFramesInFlight;
    }

    void shutdown() {
        vkDeviceWaitIdle(dev.device());
        pipes.destroy(dev.device());
        swap.destroy(dev);
        for (auto& s : sync) {
            vkDestroySemaphore(dev.device(), s.image_available, nullptr);
            vkDestroySemaphore(dev.device(), s.render_finished, nullptr);
            vkDestroyFence(dev.device(), s.fence, nullptr);
        }
        if (!cmd_bufs.empty()) {
            vkFreeCommandBuffers(dev.device(), dev.command_pool(),
                                 static_cast<uint32_t>(cmd_bufs.size()), cmd_bufs.data());
        }
    }
};

void sleep_frame(std::chrono::steady_clock::time_point frame_start) {
    auto elapsed = std::chrono::steady_clock::now() - frame_start;
    auto target = std::chrono::milliseconds(kFrameMs);
    if (elapsed < target) {
        std::this_thread::sleep_for(target - elapsed);
    }
}

}  // namespace

void run_vulkan_app(const Question& question) {
    if (!glfwInit()) {
        throw std::runtime_error("glfwInit 失败");
    }
    glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);
    glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

    int w = static_cast<int>(question.width > 0 ? question.width : 1);
    int h = static_cast<int>(question.height > 0 ? question.height : 1);
    std::string title = "upskill_cpp - id " + question.id;
    GLFWwindow* window = glfwCreateWindow(w, h, title.c_str(), nullptr, nullptr);
    if (!window) {
        glfwTerminate();
        throw std::runtime_error("glfwCreateWindow 失败");
    }

    VulkanApp app;
    app.init(window, question);

    double angle_y = 0;
    double angle_z = 0;
    auto last = std::chrono::steady_clock::now();

    while (!glfwWindowShouldClose(window)) {
        auto frame_start = std::chrono::steady_clock::now();
        glfwPollEvents();
        if (glfwGetKey(window, GLFW_KEY_ESCAPE) == GLFW_PRESS) {
            glfwSetWindowShouldClose(window, GLFW_TRUE);
        }

        double dt =
            std::chrono::duration<double>(frame_start - last).count();
        last = frame_start;
        angle_y += kOmegaY * dt;
        angle_z += kOmegaZ * dt;

        app.draw_frame(question, angle_y, angle_z);
        sleep_frame(frame_start);
    }

    app.shutdown();
    glfwDestroyWindow(window);
    glfwTerminate();
}
