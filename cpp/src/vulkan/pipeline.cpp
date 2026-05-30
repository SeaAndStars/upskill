#include "vulkan/pipeline.hpp"

#include <cstddef>
#include <fstream>
#include <stdexcept>

#include "frame_geom.hpp"
#include "vulkan/vk_check.hpp"

namespace {

std::vector<uint32_t> read_spv_file(const char* path) {
    std::ifstream f(path, std::ios::binary | std::ios::ate);
    if (!f) {
        throw std::runtime_error(std::string("无法打开着色器: ") + path);
    }
    auto size = f.tellg();
    f.seekg(0);
    std::vector<uint32_t> code(static_cast<std::size_t>(size) / 4);
    f.read(reinterpret_cast<char*>(code.data()), size);
    return code;
}

VkShaderModule create_shader_module(VkDevice device, const std::vector<uint32_t>& code) {
    VkShaderModuleCreateInfo info{};
    info.sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO;
    info.codeSize = code.size() * sizeof(uint32_t);
    info.pCode = code.data();
    VkShaderModule mod = VK_NULL_HANDLE;
    vk_check(vkCreateShaderModule(device, &info, nullptr, &mod), "shader module");
    return mod;
}

VkPipeline build_graphics_pipeline(VkDevice device, VkRenderPass render_pass,
                                   VkPipelineLayout layout, VkExtent2D extent,
                                   VkShaderModule vert, VkShaderModule frag) {
    VkPipelineShaderStageCreateInfo stages[2]{};
    stages[0].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stages[0].stage = VK_SHADER_STAGE_VERTEX_BIT;
    stages[0].module = vert;
    stages[0].pName = "main";
    stages[1].sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO;
    stages[1].stage = VK_SHADER_STAGE_FRAGMENT_BIT;
    stages[1].module = frag;
    stages[1].pName = "main";

    VkVertexInputBindingDescription binding{};
    binding.binding = 0;
    binding.stride = sizeof(Vertex);
    binding.inputRate = VK_VERTEX_INPUT_RATE_VERTEX;

    VkVertexInputAttributeDescription attrs[3]{};
    attrs[0] = {0, 0, VK_FORMAT_R32G32_SFLOAT, offsetof(Vertex, pos)};
    attrs[1] = {1, 0, VK_FORMAT_R32G32_SFLOAT, offsetof(Vertex, uv)};
    attrs[2] = {2, 0, VK_FORMAT_R32_SFLOAT, offsetof(Vertex, r_ndc)};

    VkPipelineVertexInputStateCreateInfo vertex_input{};
    vertex_input.sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO;
    vertex_input.vertexBindingDescriptionCount = 1;
    vertex_input.pVertexBindingDescriptions = &binding;
    vertex_input.vertexAttributeDescriptionCount = 3;
    vertex_input.pVertexAttributeDescriptions = attrs;

    VkPipelineInputAssemblyStateCreateInfo input_asm{};
    input_asm.sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO;
    input_asm.topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST;

    VkViewport viewport{};
    viewport.width = static_cast<float>(extent.width);
    viewport.height = static_cast<float>(extent.height);
    viewport.maxDepth = 1.0f;
    VkRect2D scissor{{0, 0}, extent};
    VkPipelineViewportStateCreateInfo viewport_state{};
    viewport_state.sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO;
    viewport_state.viewportCount = 1;
    viewport_state.pViewports = &viewport;
    viewport_state.scissorCount = 1;
    viewport_state.pScissors = &scissor;

    VkPipelineRasterizationStateCreateInfo raster{};
    raster.sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO;
    raster.polygonMode = VK_POLYGON_MODE_FILL;
    raster.cullMode = VK_CULL_MODE_NONE;
    raster.frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE;
    raster.lineWidth = 1.0f;

    VkPipelineMultisampleStateCreateInfo ms{};
    ms.sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO;
    ms.rasterizationSamples = kMsaaSamples;

    VkPipelineColorBlendAttachmentState blend_att{};
    blend_att.colorWriteMask = VK_COLOR_COMPONENT_R_BIT | VK_COLOR_COMPONENT_G_BIT |
                               VK_COLOR_COMPONENT_B_BIT | VK_COLOR_COMPONENT_A_BIT;
    blend_att.blendEnable = VK_TRUE;
    blend_att.srcColorBlendFactor = VK_BLEND_FACTOR_ONE;
    blend_att.dstColorBlendFactor = VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA;
    blend_att.colorBlendOp = VK_BLEND_OP_ADD;
    blend_att.srcAlphaBlendFactor = VK_BLEND_FACTOR_ONE;
    blend_att.dstAlphaBlendFactor = VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA;
    blend_att.alphaBlendOp = VK_BLEND_OP_ADD;

    VkPipelineColorBlendStateCreateInfo blend{};
    blend.sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO;
    blend.attachmentCount = 1;
    blend.pAttachments = &blend_att;

    VkDynamicState dyn_states[] = {VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR};
    VkPipelineDynamicStateCreateInfo dyn{};
    dyn.sType = VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO;
    dyn.dynamicStateCount = 2;
    dyn.pDynamicStates = dyn_states;

    VkGraphicsPipelineCreateInfo gp{};
    gp.sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO;
    gp.stageCount = 2;
    gp.pStages = stages;
    gp.pVertexInputState = &vertex_input;
    gp.pInputAssemblyState = &input_asm;
    gp.pViewportState = &viewport_state;
    gp.pRasterizationState = &raster;
    gp.pMultisampleState = &ms;
    gp.pColorBlendState = &blend;
    gp.pDynamicState = &dyn;
    gp.layout = layout;
    gp.renderPass = render_pass;
    gp.subpass = 0;

    VkPipeline pipeline = VK_NULL_HANDLE;
    vk_check(vkCreateGraphicsPipelines(device, VK_NULL_HANDLE, 1, &gp, nullptr, &pipeline),
             "graphics pipeline");
    return pipeline;
}

}  // namespace

Pipelines::~Pipelines() {
    // destroy 由外部在 device 存活时调用
}

void Pipelines::create(VkDevice device, const SwapchainContext& swap) {
    std::string base = SPV_DIR;
    vert_ = create_shader_module(device, read_spv_file((base + "/color.vert.spv").c_str()));
    line_frag_ = create_shader_module(device, read_spv_file((base + "/line.frag.spv").c_str()));
    point_frag_ = create_shader_module(device, read_spv_file((base + "/point.frag.spv").c_str()));

    VkPipelineLayoutCreateInfo layout_info{};
    layout_info.sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO;
    vk_check(vkCreatePipelineLayout(device, &layout_info, nullptr, &layout_), "pipeline layout");

    line_ = build_graphics_pipeline(device, swap.render_pass(), layout_, swap.extent(), vert_,
                                    line_frag_);
    point_ = build_graphics_pipeline(device, swap.render_pass(), layout_, swap.extent(), vert_,
                                     point_frag_);
}

void Pipelines::destroy(VkDevice device) {
    if (line_) {
        vkDestroyPipeline(device, line_, nullptr);
        line_ = VK_NULL_HANDLE;
    }
    if (point_) {
        vkDestroyPipeline(device, point_, nullptr);
        point_ = VK_NULL_HANDLE;
    }
    if (layout_) {
        vkDestroyPipelineLayout(device, layout_, nullptr);
        layout_ = VK_NULL_HANDLE;
    }
    if (vert_) {
        vkDestroyShaderModule(device, vert_, nullptr);
        vert_ = VK_NULL_HANDLE;
    }
    if (line_frag_) {
        vkDestroyShaderModule(device, line_frag_, nullptr);
        line_frag_ = VK_NULL_HANDLE;
    }
    if (point_frag_) {
        vkDestroyShaderModule(device, point_frag_, nullptr);
        point_frag_ = VK_NULL_HANDLE;
    }
}
