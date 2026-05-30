//! 图形管线：线段与圆点。

use ash::vk;

use super::swapchain::SwapchainContext;

/// 着色器 SPIR-V（build.rs 生成）。
const VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/color.vert.spv"));
const LINE_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/line.frag.spv"));
const POINT_FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/point.frag.spv"));

/// 双管线封装。
pub struct Pipelines {
    /// 线段管线。
    pub line: vk::Pipeline,
    /// 点管线。
    pub point: vk::Pipeline,
    /// 管线布局。
    pub layout: vk::PipelineLayout,
    /// 顶点着色器模块。
    pub vert_shader: vk::ShaderModule,
    /// 线片段着色器。
    pub line_frag: vk::ShaderModule,
    /// 点片段着色器。
    pub point_frag: vk::ShaderModule,
}

impl Pipelines {
    /// 创建 line/point 管线。
    pub fn new(device: &ash::Device, swap: &SwapchainContext) -> Result<Self, String> {
        let vert_shader = create_shader_module(device, VERT_SPV)?;
        let line_frag = create_shader_module(device, LINE_FRAG_SPV)?;
        let point_frag = create_shader_module(device, POINT_FRAG_SPV)?;

        let layout = unsafe {
            device
                .create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
                .map_err(|e| format!("pipeline_layout: {e}"))?
        };

        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<crate::frame_geom::Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attrs = [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(8),
            vk::VertexInputAttributeDescription::default()
                .location(2)
                .binding(0)
                .format(vk::Format::R32_SFLOAT)
                .offset(16),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding))
            .vertex_attribute_descriptions(&attrs);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: swap.extent.width as f32,
            height: swap.extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swap.extent,
        };
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        let raster = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(super::swapchain::MSAA_SAMPLES)
            .sample_shading_enable(false);

        let color_blend_attach = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD);

        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attach));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let line_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader)
                .name(c"vs_main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(line_frag)
                .name(c"main"),
        ];
        let point_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_shader)
                .name(c"vs_main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(point_frag)
                .name(c"main"),
        ];

        let mut pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&line_stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&raster)
            .multisample_state(&multisample)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic)
            .layout(layout)
            .render_pass(swap.render_pass)
            .subpass(0);

        let line = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|e| format!("line pipeline: {:?}", e.1))?[0]
        };

        pipeline_info = pipeline_info.stages(&point_stages);
        let point = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|e| format!("point pipeline: {:?}", e.1))?[0]
        };

        Ok(Self {
            line,
            point,
            layout,
            vert_shader,
            line_frag,
            point_frag,
        })
    }
}

impl Pipelines {
    /// 销毁管线与着色器模块。
    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.line, None);
            device.destroy_pipeline(self.point, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_shader_module(self.vert_shader, None);
            device.destroy_shader_module(self.line_frag, None);
            device.destroy_shader_module(self.point_frag, None);
        }
    }
}

/// 从 SPIR-V 字节创建着色器模块。
fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule, String> {
    let words: &[u32] = bytemuck::cast_slice(code);
    let info = vk::ShaderModuleCreateInfo::default().code(words);
    unsafe { device.create_shader_module(&info, None) }.map_err(|e| format!("shader_module: {e}"))
}
