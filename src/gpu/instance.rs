//! Vulkan 实例与 Surface。

use ash::khr::surface::Instance as SurfaceLoader;
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

/// Vulkan 实例。
pub struct VulkanInstance {
    /// ash 入口。
    pub entry: ash::Entry,
    /// Vulkan 实例。
    pub instance: ash::Instance,
    /// Surface 加载器。
    pub surface_loader: SurfaceLoader,
}

impl VulkanInstance {
    /// 创建实例并绑定窗口 Surface。
    pub fn new(window: &Window) -> Result<(Self, vk::SurfaceKHR), String> {
        let entry = unsafe { ash::Entry::load() }.map_err(|e| format!("加载 Vulkan: {e}"))?;

        let display = window
            .display_handle()
            .map_err(|e| format!("display_handle: {e}"))?
            .as_raw();
        let win = window
            .window_handle()
            .map_err(|e| format!("window_handle: {e}"))?
            .as_raw();

        let mut extensions = ash_window::enumerate_required_extensions(display)
            .map_err(|e| format!("Surface 扩展: {e}"))?
            .to_vec();

        #[cfg(target_os = "macos")]
        {
            extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
        }

        let app_name = std::ffi::CString::new("upskill").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_2);

        let mut create_info =
            vk::InstanceCreateInfo::default().application_info(&app_info).enabled_extension_names(&extensions);

        #[cfg(target_os = "macos")]
        {
            create_info.flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
        }

        let instance = unsafe { entry.create_instance(&create_info, None) }
            .map_err(|e| format!("create_instance: {e}"))?;

        let surface_loader = SurfaceLoader::new(&entry, &instance);
        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display, win, None)
                .map_err(|e| format!("create_surface: {e}"))?
        };

        Ok((
            Self {
                entry,
                instance,
                surface_loader,
            },
            surface,
        ))
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
