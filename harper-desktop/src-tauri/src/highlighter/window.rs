use std::num::NonZeroU32;
use std::sync::Arc;

use egui_wgpu::wgpu::{self, PresentMode};
use egui_wgpu::winit::Painter;
use egui_wgpu::{RendererOptions, WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::monitor::MonitorHandle;
use winit::window::{Window as WinitWindow, WindowButtons, WindowId, WindowLevel};

use super::Error;
use super::render_state::RenderState;

/// A transparent click-through overlay window for one monitor.
///
/// `Window` owns the native winit window plus egui/wgpu integration required to render into it. It
/// deliberately does not own highlighter drawing decisions; those are supplied by `RenderState`
/// during each redraw.
pub struct Window {
    inner: Arc<WinitWindow>,
    egui_state: egui_winit::State,
    painter: Painter,
    viewport_id: egui::ViewportId,
}

impl Window {
    pub async fn new(
        event_loop: &ActiveEventLoop,
        monitor: MonitorHandle,
        context: egui::Context,
    ) -> Result<Self, Error> {
        let position = monitor.position();
        let size = monitor.size();
        let attributes = WinitWindow::default_attributes()
            .with_title(crate::branding::APP_NAME)
            // Stay hidden until we know the GPU can draw a see-through window. An opaque
            // full-screen overlay would black out the user's screen.
            .with_visible(false)
            .with_inner_size(size)
            .with_position(position)
            .with_resizable(false)
            .with_enabled_buttons(WindowButtons::empty())
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_active(false);
        // Keep the overlay out of the taskbar and Alt+Tab.
        #[cfg(target_os = "windows")]
        let attributes = {
            use winit::platform::windows::WindowAttributesExtWindows;
            attributes.with_skip_taskbar(true)
        };
        let window = Arc::new(event_loop.create_window(attributes)?);

        window.set_outer_position(PhysicalPosition::new(position.x, position.y));
        let _ = window.request_inner_size(PhysicalSize::new(size.width, size.height));
        window.set_cursor_hittest(false)?;
        let viewport_id = egui::ViewportId::from_hash_of(window.id());

        let egui_state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            event_loop,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );

        let setup = overlay_wgpu_setup(event_loop);
        if !supports_transparency(&setup, window.clone()).await {
            return Err(Error::NoTransparency);
        }

        let mut painter = Painter::new(
            context,
            WgpuConfiguration {
                present_mode: PresentMode::Fifo,
                wgpu_setup: WgpuSetup::CreateNew(setup),
                ..Default::default()
            },
            true,
            RendererOptions::default(),
        )
        .await;
        painter
            .set_window(viewport_id, Some(window.clone()))
            .await?;
        window.set_visible(true);
        window.request_redraw();

        Ok(Self {
            inner: window,
            egui_state,
            painter,
            viewport_id,
        })
    }

    pub fn id(&self) -> WindowId {
        self.inner.id()
    }

    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    /// Controls whether the transparent overlay can receive pointer events.
    ///
    /// Highlight windows are click-through by default so the underlying app remains usable. The
    /// window manager temporarily enables hit-testing when global cursor polling shows the pointer
    /// is over an interactive highlight or popup.
    pub fn set_cursor_hittest(&self, enabled: bool) -> Result<(), Error> {
        self.inner.set_cursor_hittest(enabled)?;

        Ok(())
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        let response = self.egui_state.on_window_event(&self.inner, event);

        if response.repaint {
            self.inner.request_redraw();
        }

        if let WindowEvent::Resized(size) = event
            && let (Some(width), Some(height)) =
                (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            self.painter
                .on_window_resized(self.viewport_id, width, height);
            self.inner.request_redraw();
        }
    }

    pub fn render(&mut self, render_state: &mut RenderState) {
        let context = self.egui_state.egui_ctx().clone();
        let input = self.egui_state.take_egui_input(&self.inner);
        let output = context.run_ui(input, |ui| {
            render_state.render(ui);
        });

        self.egui_state
            .handle_platform_output(&self.inner, output.platform_output);

        let clipped_primitives = context.tessellate(output.shapes, output.pixels_per_point);
        self.painter.paint_and_update_textures(
            self.viewport_id,
            output.pixels_per_point,
            [0.0, 0.0, 0.0, 0.0],
            &clipped_primitives,
            &output.textures_delta,
            Vec::new(),
        );
    }
}

/// Picks a GPU setup that can present a see-through window.
///
/// On Windows, wgpu's default DX12 swapchain is created straight from the window handle, and that
/// kind of swapchain is always opaque: the overlay would cover every monitor in black. A swapchain
/// made from a DirectComposition visual supports per-pixel transparency, and Vulkan or OpenGL
/// surfaces on Windows do not, so DX12 with a composition visual is the only option there. The
/// overlay is cheap to draw, so the integrated GPU is preferred to save battery.
fn overlay_wgpu_setup(event_loop: &ActiveEventLoop) -> WgpuSetupCreateNew {
    let mut setup = WgpuSetupCreateNew::from_display_handle(event_loop.owned_display_handle());
    setup.power_preference = wgpu::PowerPreference::LowPower;

    #[cfg(target_os = "windows")]
    {
        setup.instance_descriptor.backends = wgpu::Backends::DX12;
        setup
            .instance_descriptor
            .backend_options
            .dx12
            .presentation_system = wgpu::Dx12SwapchainKind::DxgiFromVisual;
    }

    setup
}

/// Checks, before anything is shown, that the window's surface can blend with the desktop.
///
/// Uses a throwaway wgpu instance so the real renderer is untouched; the probe surface is dropped
/// before the renderer creates its own.
async fn supports_transparency(setup: &WgpuSetupCreateNew, window: Arc<WinitWindow>) -> bool {
    let descriptor = &setup.instance_descriptor;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: descriptor.backends,
        flags: descriptor.flags,
        backend_options: descriptor.backend_options.clone(),
        memory_budget_thresholds: descriptor.memory_budget_thresholds,
        display: None,
    });

    let surface = match instance.create_surface(window) {
        Ok(surface) => surface,
        Err(error) => {
            eprintln!("Overlay disabled: could not create a GPU surface: {error}");
            return false;
        }
    };

    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: setup.power_preference,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            eprintln!("Overlay disabled: no GPU adapter can draw the overlay: {error}");
            return false;
        }
    };

    let alpha_modes = surface.get_capabilities(&adapter).alpha_modes;
    let transparent = alpha_modes.iter().any(|mode| {
        matches!(
            mode,
            wgpu::CompositeAlphaMode::PreMultiplied | wgpu::CompositeAlphaMode::PostMultiplied
        )
    });

    let info = adapter.get_info();
    tracing::info!(
        "Overlay GPU: {} ({:?}, {:?}); alpha modes {alpha_modes:?}",
        info.name,
        info.backend,
        info.device_type
    );

    if !transparent {
        eprintln!(
            "Overlay disabled: the {:?} adapter only offers {alpha_modes:?}, which would draw an opaque window",
            adapter.get_info().backend
        );
    }

    transparent
}
