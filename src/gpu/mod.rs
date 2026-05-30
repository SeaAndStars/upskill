//! Vulkan 渲染应用（winit + ash + MSAA）。

mod device;
mod instance;
mod pipeline;
mod swapchain;

use std::sync::Arc;
use std::time::{Duration, Instant};

use ash::vk;
use gpu_allocator::vulkan::Allocation;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::frame_geom::{build_frame_geometry, FrameGeometry, Vertex};
use crate::parser::Question;

use device::VulkanDevice;
use instance::VulkanInstance;
use pipeline::Pipelines;
use swapchain::SwapchainContext;

/// 帧间隔（60 FPS）。
const FRAME_MS: u64 = 16;

/// 飞行中帧数。
const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// 清屏色。
const CLEAR_COLOR: vk::ClearColorValue = vk::ClearColorValue {
    float32: [0.063, 0.063, 0.094, 1.0],
};

/// Y 轴角速度。
const OMEGA_Y: f64 = 0.7;

/// Z 轴角速度。
const OMEGA_Z: f64 = 0.5;

/// 对外入口。
pub fn run(question: &Question) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("EventLoop: {e}"))?;
    let mut app = AppState::new(question.clone())?;
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("event loop: {e}"))?;
    Ok(())
}

/// winit 应用状态。
struct AppState {
    question: Question,
    window: Option<Arc<Window>>,
    vk: Option<VulkanApp>,
    angle_y: f64,
    angle_z: f64,
    last_frame: Instant,
    should_exit: bool,
}

impl AppState {
    fn new(question: Question) -> Result<Self, String> {
        Ok(Self {
            question,
            window: None,
            vk: None,
            angle_y: 0.0,
            angle_z: 0.0,
            last_frame: Instant::now(),
            should_exit: false,
        })
    }
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let w = self.question.width.max(1) as u32;
        let h = self.question.height.max(1) as u32;
        let attrs = WindowAttributes::default()
            .with_title(format!("upskill - id {}", self.question.id))
            .with_inner_size(winit::dpi::LogicalSize::new(w, h))
            .with_resizable(false);
        let window = Arc::new(event_loop.create_window(attrs).expect("create_window"));
        match VulkanApp::new(window.clone(), &self.question) {
            Ok(vk) => {
                self.vk = Some(vk);
                self.window = Some(window);
            }
            Err(e) => {
                eprintln!("Vulkan 初始化失败: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.should_exit = true;
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && event.logical_key == Key::Named(NamedKey::Escape)
                {
                    self.should_exit = true;
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.should_exit {
                    return;
                }
                let frame_start = Instant::now();
                let dt = frame_start.duration_since(self.last_frame).as_secs_f64();
                self.last_frame = frame_start;
                self.angle_y += OMEGA_Y * dt;
                self.angle_z += OMEGA_Z * dt;

                if let Some(vk) = self.vk.as_mut() {
                    if let Err(e) =
                        vk.draw_frame(&self.question, self.angle_y, self.angle_z)
                    {
                        eprintln!("渲染帧失败: {e}");
                        event_loop.exit();
                    }
                }

                let elapsed = frame_start.elapsed();
                if elapsed < Duration::from_millis(FRAME_MS) {
                    std::thread::sleep(Duration::from_millis(FRAME_MS) - elapsed);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

struct FrameSync {
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    fence: vk::Fence,
}

struct VulkanApp {
    inst: VulkanInstance,
    surface: vk::SurfaceKHR,
    dev: VulkanDevice,
    swap: SwapchainContext,
    pipelines: Pipelines,
    frame_sync: Vec<FrameSync>,
    frame_index: usize,
    command_buffers: Vec<vk::CommandBuffer>,
}

impl VulkanApp {
    fn new(window: Arc<Window>, question: &Question) -> Result<Self, String> {
        let (inst, surface) = VulkanInstance::new(&window)?;
        let (mut dev, pd) = VulkanDevice::new(&inst, surface)?;
        let w = question.width.max(1) as u32;
        let h = question.height.max(1) as u32;
        let swap = SwapchainContext::new(&inst, &mut dev, pd, surface, w, h)?;
        let pipelines = Pipelines::new(&dev.device, &swap)?;

        let mut frame_sync = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let sem_info = vk::SemaphoreCreateInfo::default();
            let image_available = unsafe { dev.device.create_semaphore(&sem_info, None) }
                .map_err(|e| format!("sem: {e}"))?;
            let render_finished = unsafe { dev.device.create_semaphore(&sem_info, None) }
                .map_err(|e| format!("sem: {e}"))?;
            let fence = unsafe {
                dev.device.create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
            }
            .map_err(|e| format!("fence: {e}"))?;
            frame_sync.push(FrameSync {
                image_available,
                render_finished,
                fence,
            });
        }

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(dev.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffers = unsafe { dev.device.allocate_command_buffers(&alloc_info) }
            .map_err(|e| format!("cmd buffers: {e}"))?;

        Ok(Self {
            inst,
            surface,
            dev,
            swap,
            pipelines,
            frame_sync,
            frame_index: 0,
            command_buffers,
        })
    }

    fn draw_frame(
        &mut self,
        question: &Question,
        angle_y: f64,
        angle_z: f64,
    ) -> Result<(), String> {
        let fi = self.frame_index;
        let sync = &self.frame_sync[fi];
        unsafe {
            self.dev
                .device
                .wait_for_fences(&[sync.fence], true, u64::MAX)
                .map_err(|e| format!("wait_fence: {e}"))?;
            self.dev
                .device
                .reset_fences(&[sync.fence])
                .map_err(|e| format!("reset_fence: {e}"))?;
        }

        let (image_index, _) = unsafe {
            self.swap.swapchain_loader.acquire_next_image(
                self.swap.swapchain,
                u64::MAX,
                sync.image_available,
                vk::Fence::null(),
            )
        }
        .map_err(|e| format!("acquire: {e}"))?;

        let geom = build_frame_geometry(question, angle_y, angle_z);
        let cmd = self.command_buffers[fi];

        unsafe {
            self.dev
                .device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .map_err(|e| format!("reset_cmd: {e}"))?;
            self.dev
                .device
                .begin_command_buffer(
                    cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .map_err(|e| format!("begin_cmd: {e}"))?;
        }

        record_frame(
            &mut self.dev,
            cmd,
            &self.swap,
            &self.pipelines,
            image_index as usize,
            &geom,
        )?;

        unsafe {
            self.dev
                .device
                .end_command_buffer(cmd)
                .map_err(|e| format!("end_cmd: {e}"))?;
        }

        let wait_sems = [sync.image_available];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_sems = [sync.render_finished];
        let cmd_bufs = [cmd];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_sems)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&cmd_bufs)
            .signal_semaphores(&signal_sems);

        unsafe {
            self.dev
                .device
                .queue_submit(self.dev.graphics_queue, &[submit_info], sync.fence)
                .map_err(|e| format!("submit: {e}"))?;
        }

        let swap_sems = [sync.render_finished];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&swap_sems)
            .swapchains(std::slice::from_ref(&self.swap.swapchain))
            .image_indices(std::slice::from_ref(&image_index));

        unsafe {
            self.swap
                .swapchain_loader
                .queue_present(self.dev.present_queue, &present_info)
                .map_err(|e| format!("present: {e}"))?;
        }

        self.frame_index = (fi + 1) % MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.dev.device.device_wait_idle().ok();
            for s in &self.frame_sync {
                self.dev.device.destroy_semaphore(s.image_available, None);
                self.dev.device.destroy_semaphore(s.render_finished, None);
                self.dev.device.destroy_fence(s.fence, None);
            }
            self.dev
                .device
                .free_command_buffers(self.dev.command_pool, &self.command_buffers);
            self.pipelines.destroy(&self.dev.device);
            self.swap
                .destroy(&mut self.dev.device, &mut self.dev.allocator);
            self.inst
                .surface_loader
                .destroy_surface(self.surface, None);
        }
    }
}

impl SwapchainContext {
    fn destroy(
        &mut self,
        device: &ash::Device,
        allocator: &mut gpu_allocator::vulkan::Allocator,
    ) {
        unsafe {
            for &fb in &self.framebuffers {
                device.destroy_framebuffer(fb, None);
            }
            device.destroy_render_pass(self.render_pass, None);
            device.destroy_image_view(self.msaa_view, None);
            device.destroy_image(self.msaa_image, None);
            if let Some(alloc) = self.msaa_allocation.take() {
                allocator.free(alloc).ok();
            }
            for &view in &self.image_views {
                device.destroy_image_view(view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }
}

/// 录制单帧：上传几何并绘制。
fn record_frame(
    dev: &mut VulkanDevice,
    cmd: vk::CommandBuffer,
    swap: &SwapchainContext,
    pipes: &Pipelines,
    image_index: usize,
    geom: &FrameGeometry,
) -> Result<(), String> {
    let mut line_mesh = MeshGpu::default();
    let mut point_mesh = MeshGpu::default();
    if !geom.line_verts.is_empty() {
        line_mesh.upload(dev, cmd, &geom.line_verts, &geom.line_indices)?;
    }
    if !geom.point_verts.is_empty() {
        point_mesh.upload(dev, cmd, &geom.point_verts, &geom.point_indices)?;
    }

    if !geom.line_verts.is_empty() || !geom.point_verts.is_empty() {
        let barrier = vk::MemoryBarrier::default().src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ);
        unsafe {
            dev.device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                std::slice::from_ref(&barrier),
                &[],
                &[],
            );
        }
    }

    let clear = vk::ClearValue {
        color: CLEAR_COLOR,
    };
    let rp_begin = vk::RenderPassBeginInfo::default()
        .render_pass(swap.render_pass)
        .framebuffer(swap.framebuffers[image_index])
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swap.extent,
        })
        .clear_values(std::slice::from_ref(&clear));

    unsafe {
        dev.device.cmd_begin_render_pass(cmd, &rp_begin, vk::SubpassContents::INLINE);
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
        dev.device.cmd_set_viewport(cmd, 0, std::slice::from_ref(&viewport));
        dev.device.cmd_set_scissor(cmd, 0, std::slice::from_ref(&scissor));

        dev.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipes.line);
        line_mesh.draw(dev, cmd);
        dev.device
            .cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipes.point);
        point_mesh.draw(dev, cmd);

        dev.device.cmd_end_render_pass(cmd);
    }

    line_mesh.destroy(dev);
    point_mesh.destroy(dev);
    Ok(())
}

#[derive(Default)]
struct MeshGpu {
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: u32,
    vertex_alloc: Option<Allocation>,
    index_alloc: Option<Allocation>,
    staging_vertex: Option<(vk::Buffer, Allocation)>,
    staging_index: Option<(vk::Buffer, Allocation)>,
}

impl MeshGpu {
    fn upload(
        &mut self,
        dev: &mut VulkanDevice,
        cmd: vk::CommandBuffer,
        verts: &[Vertex],
        indices: &[u32],
    ) -> Result<(), String> {
        let v_size = (verts.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize;
        let i_size = (indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize;
        self.index_count = indices.len() as u32;

        let (sv, mut sa) = dev.create_staging_buffer(v_size)?;
        if let Some(mapped) = sa.mapped_slice_mut() {
            mapped.copy_from_slice(bytemuck::cast_slice(verts));
        } else {
            return Err("staging 不可映射".into());
        }
        let (iv, mut ia) = dev.create_staging_buffer(i_size)?;
        if let Some(mapped) = ia.mapped_slice_mut() {
            mapped.copy_from_slice(bytemuck::cast_slice(indices));
        } else {
            return Err("index staging 不可映射".into());
        }

        let (vb, va) = dev.create_device_buffer(
            v_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let (ib, iba) = dev.create_device_buffer(
            i_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        )?;

        let regions_v = [vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: v_size,
        }];
        let regions_i = [vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: i_size,
        }];
        unsafe {
            dev.device.cmd_copy_buffer(cmd, sv, vb, &regions_v);
            dev.device.cmd_copy_buffer(cmd, iv, ib, &regions_i);
        }

        self.vertex_buffer = vb;
        self.index_buffer = ib;
        self.vertex_alloc = Some(va);
        self.index_alloc = Some(iba);
        self.staging_vertex = Some((sv, sa));
        self.staging_index = Some((iv, ia));
        Ok(())
    }

    fn draw(&self, dev: &VulkanDevice, cmd: vk::CommandBuffer) {
        if self.index_count == 0 {
            return;
        }
        unsafe {
            dev.device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer], &[0]);
            dev.device
                .cmd_bind_index_buffer(cmd, self.index_buffer, 0, vk::IndexType::UINT32);
            dev.device.cmd_draw_indexed(cmd, self.index_count, 1, 0, 0, 0);
        }
    }

    fn destroy(&mut self, dev: &mut VulkanDevice) {
        unsafe {
            if self.vertex_buffer != vk::Buffer::null() {
                dev.device.destroy_buffer(self.vertex_buffer, None);
            }
            if self.index_buffer != vk::Buffer::null() {
                dev.device.destroy_buffer(self.index_buffer, None);
            }
            if let Some((b, a)) = self.staging_vertex.take() {
                dev.device.destroy_buffer(b, None);
                dev.allocator.free(a).ok();
            }
            if let Some((b, a)) = self.staging_index.take() {
                dev.device.destroy_buffer(b, None);
                dev.allocator.free(a).ok();
            }
            if let Some(a) = self.vertex_alloc.take() {
                dev.allocator.free(a).ok();
            }
            if let Some(a) = self.index_alloc.take() {
                dev.allocator.free(a).ok();
            }
        }
    }
}
