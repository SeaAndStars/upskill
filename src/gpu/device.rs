//! 物理/逻辑设备、队列与分配器。

use ash::khr::surface::Instance as SurfaceLoader;
use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_allocator::MemoryLocation;

use super::instance::VulkanInstance;

/// 队列 族索引。
pub struct QueueFamilies {
    /// 图形队列族。
    pub graphics: u32,
    /// 呈现队列族。
    pub present: u32,
}

/// 逻辑设备与队列。
pub struct VulkanDevice {
    /// 逻辑设备。
    pub device: ash::Device,
    /// 图形队列。
    pub graphics_queue: vk::Queue,
    /// 呈现队列。
    pub present_queue: vk::Queue,
    /// 队列族。
    pub families: QueueFamilies,
    /// GPU 内存分配器。
    pub allocator: Allocator,
    /// 图形队列命令池。
    pub command_pool: vk::CommandPool,
}

impl VulkanDevice {
    /// 选择设备并创建逻辑设备。
    pub fn new(
        inst: &VulkanInstance,
        surface: vk::SurfaceKHR,
    ) -> Result<(Self, vk::PhysicalDevice), String> {
        let pds = unsafe { inst.instance.enumerate_physical_devices() }
            .map_err(|e| format!("enumerate_physical_devices: {e}"))?;
        if pds.is_empty() {
            return Err("未找到 Vulkan 物理设备".into());
        }

        let mut chosen_pd = None;
        let mut chosen_families = None;

        for pd in pds {
            if let Some(fam) = find_queue_families(&inst.instance, &inst.surface_loader, pd, surface) {
                chosen_pd = Some(pd);
                chosen_families = Some(fam);
                break;
            }
        }

        let pd = chosen_pd.ok_or("无支持图形与呈现的 Vulkan 设备")?;
        let families = chosen_families.unwrap();

        let queue_priorities = [1.0f32];
        let queue_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(families.graphics)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(families.present)
                .queue_priorities(&queue_priorities),
        ];

        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];
        let device_create = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extensions);

        let device = unsafe { inst.instance.create_device(pd, &device_create, None) }
            .map_err(|e| format!("create_device: {e}"))?;

        let graphics_queue = unsafe { device.get_device_queue(families.graphics, 0) };
        let present_queue = unsafe { device.get_device_queue(families.present, 0) };

        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(families.graphics);
        let command_pool = unsafe { device.create_command_pool(&command_pool_info, None) }
            .map_err(|e| format!("create_command_pool: {e}"))?;

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: inst.instance.clone(),
            device: device.clone(),
            physical_device: pd,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .map_err(|e| format!("Allocator: {e}"))?;

        Ok((
            Self {
                device,
                graphics_queue,
                present_queue,
                families,
                allocator,
                command_pool,
            },
            pd,
        ))
    }

    /// 分配设备本地缓冲。
    pub fn create_device_buffer(
        &mut self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
    ) -> Result<(vk::Buffer, gpu_allocator::vulkan::Allocation), String> {
        let info = vk::BufferCreateInfo::default().size(size).usage(usage).sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { self.device.create_buffer(&info, None) }
            .map_err(|e| format!("create_buffer: {e}"))?;
        let req = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let allocation = self
            .allocator
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "buffer",
                requirements: req,
                location: MemoryLocation::GpuOnly,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("allocate: {e}"))?;
        unsafe {
            self.device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| format!("bind_buffer_memory: {e}"))?;
        }
        Ok((buffer, allocation))
    }

    /// 分配可映射 staging 缓冲。
    pub fn create_staging_buffer(
        &mut self,
        size: vk::DeviceSize,
    ) -> Result<(vk::Buffer, gpu_allocator::vulkan::Allocation), String> {
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { self.device.create_buffer(&info, None) }
            .map_err(|e| format!("create_buffer: {e}"))?;
        let req = unsafe { self.device.get_buffer_memory_requirements(buffer) };
        let allocation = self
            .allocator
            .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: "staging",
                requirements: req,
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| format!("allocate staging: {e}"))?;
        unsafe {
            self.device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| format!("bind staging: {e}"))?;
        }
        Ok((buffer, allocation))
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
        }
    }
}

/// 查找图形与呈现队列族。
fn find_queue_families(
    instance: &ash::Instance,
    surface_loader: &SurfaceLoader,
    pd: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> Option<QueueFamilies> {
    let families = unsafe { instance.get_physical_device_queue_family_properties(pd) };
    let mut graphics = None;
    let mut present = None;
    for (i, fam) in families.iter().enumerate() {
        if fam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            graphics = Some(i as u32);
        }
        let supported = unsafe {
            surface_loader
                .get_physical_device_surface_support(pd, i as u32, surface)
                .ok()?
        };
        if supported {
            present = Some(i as u32);
        }
    }
    Some(QueueFamilies {
        graphics: graphics?,
        present: present?,
    })
}
