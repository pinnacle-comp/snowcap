use std::{collections::HashMap, num::NonZeroU32, ptr::NonNull};

use iced::Size;
use iced_wgpu::{graphics::Viewport, wgpu::SurfaceTargetUnsafe};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    reexports::client::{Proxy, QueueHandle},
    shell::{
        wlr_layer::{self, Anchor, LayerSurface},
        WaylandSurface,
    },
};

use crate::{clipboard::WaylandClipboard, state::State, widget::SnowcapWidgetProgram};

pub struct SnowcapLayer {
    // SAFETY: Drop order: surface needs to be dropped before the layer
    surface: iced_wgpu::wgpu::Surface<'static>,
    pub layer: LayerSurface,

    width: u32,
    height: u32,
    pub viewport: Viewport,

    pub widgets: iced_runtime::program::State<SnowcapWidgetProgram>,
    pub clipboard: WaylandClipboard,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExclusiveZone {
    /// This layer surface wants an exclusive zone of the given size.
    Exclusive(NonZeroU32),
    /// This layer surface does not have an exclusive zone but wants to be placed respecting any.
    Respect,
    /// This layer surface does not have an exclusive zone and wants to be placed ignoring any.
    Ignore,
}

impl SnowcapLayer {
    pub fn new(
        state: &mut State,
        width: u32,
        height: u32,
        anchor: Anchor,
        exclusive_zone: ExclusiveZone,
        keyboard_interactivity: wlr_layer::KeyboardInteractivity,
        program: SnowcapWidgetProgram,
    ) -> Self {
        let surface = state.compositor_state.create_surface(&state.queue_handle);
        let layer = state.layer_shell_state.create_layer_surface(
            &state.queue_handle,
            surface,
            wlr_layer::Layer::Top,
            Some("snowcap"),
            None,
        );

        layer.set_size(width, height);
        layer.set_anchor(anchor);
        layer.set_keyboard_interactivity(keyboard_interactivity);
        layer.set_exclusive_zone(match exclusive_zone {
            ExclusiveZone::Exclusive(size) => size.get() as i32,
            ExclusiveZone::Respect => 0,
            ExclusiveZone::Ignore => -1,
        });

        layer.commit();

        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(state.conn.backend().display_ptr() as *mut _).unwrap(),
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(layer.wl_surface().id().as_ptr() as *mut _).unwrap(),
        ));

        let wgpu_surface = unsafe {
            state
                .wgpu
                .instance
                .create_surface_unsafe(SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle,
                })
                .unwrap()
        };

        let capabilities = wgpu_surface.get_capabilities(&state.wgpu.adapter);
        let surface_config = iced_wgpu::wgpu::SurfaceConfiguration {
            usage: iced_wgpu::wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: capabilities.formats[0],
            width,
            height,
            present_mode: iced_wgpu::wgpu::PresentMode::Mailbox,
            desired_maximum_frame_latency: 1,
            alpha_mode: iced_wgpu::wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![capabilities.formats[0]],
        };

        wgpu_surface.configure(&state.wgpu.device, &surface_config);

        let widgets = iced_runtime::program::State::new(
            program,
            [width as f32, height as f32].into(),
            &mut state.wgpu.renderer,
            &mut iced_runtime::Debug::new(),
        );

        let clipboard =
            unsafe { WaylandClipboard::new(state.conn.backend().display_ptr() as *mut _) };

        Self {
            surface: wgpu_surface,
            layer,
            width,
            height,
            viewport: Viewport::with_physical_size(Size::new(width, height), 1.0),
            widgets,
            clipboard,
        }
    }

    pub fn draw(
        &self,
        device: &iced_wgpu::wgpu::Device,
        queue: &iced_wgpu::wgpu::Queue,
        renderer: &mut iced_wgpu::Renderer,
        qh: &QueueHandle<State>,
    ) {
        let Ok(frame) = self.surface.get_current_texture() else {
            return;
        };

        let mut encoder =
            device.create_command_encoder(&iced_wgpu::wgpu::CommandEncoderDescriptor::default());

        let view = frame
            .texture
            .create_view(&iced_wgpu::wgpu::TextureViewDescriptor::default());

        {
            renderer.with_primitives(|backend, primitives| {
                backend.present::<String>(
                    device,
                    queue,
                    &mut encoder,
                    Some(iced::Color::new(0.3, 0.3, 0.3, 1.0)),
                    iced_wgpu::wgpu::TextureFormat::Bgra8UnormSrgb,
                    &view,
                    primitives,
                    &self.viewport,
                    &[],
                );
            });
        }

        queue.submit(Some(encoder.finish()));

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);

        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());

        // Does a commit
        frame.present();
    }
}
