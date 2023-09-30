mod clipboard;
mod keyboard;
mod pointer;
mod program;

use anyhow::Context;
use clipboard::WaylandClipboard;
use iced::{mouse::ScrollDelta, Color, Size, Theme};
use iced_runtime::Debug;
use iced_wgpu::{
    graphics::{Renderer, Viewport},
    wgpu::{self, Backends},
};
use program::Prog;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::client::{
        globals::registry_queue_init,
        protocol::{
            wl_keyboard::WlKeyboard,
            wl_output::WlOutput,
            wl_pointer::{self, AxisSource, WlPointer},
            wl_seat::WlSeat,
            wl_surface::WlSurface,
        },
        Connection, Proxy, QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{self, Anchor, LayerShell, LayerShellHandler, LayerSurface},
        WaylandSurface,
    },
};
use tracing_subscriber::EnvFilter;

struct RawWaylandHandle(RawDisplayHandle, RawWindowHandle);

unsafe impl HasRawDisplayHandle for RawWaylandHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.0
    }
}

unsafe impl HasRawWindowHandle for RawWaylandHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.1
    }
}

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,

    layer: LayerSurface,
    width: u32,
    height: u32,
    viewport: Viewport,

    exit: bool,

    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    renderer: iced_wgpu::Renderer<Theme>,

    program: iced_runtime::program::State<Prog>,

    clipboard: WaylandClipboard,

    keyboard_focus: bool,
    keyboard_modifiers: Modifiers,

    pointer: Option<wl_pointer::WlPointer>,
    pointer_location: (f64, f64),

    initial_configure_sent: bool,
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let conn = Connection::connect_to_env().context("failed to connect to wayland")?;
    let (globals, mut event_queue) =
        registry_queue_init::<State>(&conn).context("failed to init registry queue")?;

    let queue_handle = event_queue.handle();

    let compositor =
        CompositorState::bind(&globals, &queue_handle).context("wl_compositor not availible")?;
    let layer_shell =
        LayerShell::bind(&globals, &queue_handle).context("layer shell not availible")?;

    let surface = compositor.create_surface(&queue_handle);
    let layer = layer_shell.create_layer_surface(
        &queue_handle,
        surface,
        wlr_layer::Layer::Overlay,
        Some("snowcap_layer"),
        None,
    );

    layer.set_keyboard_interactivity(wlr_layer::KeyboardInteractivity::OnDemand);

    let width = 256;
    let height = 128;

    layer.set_size(width, height);
    layer.set_anchor(Anchor::TOP);

    layer.commit();

    let wgpu_instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL | wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let handle = {
        let mut handle = WaylandDisplayHandle::empty();
        handle.display = conn.backend().display_ptr() as *mut _;
        let display_handle = RawDisplayHandle::Wayland(handle);

        let mut handle = WaylandWindowHandle::empty();
        handle.surface = layer.wl_surface().id().as_ptr() as *mut _;
        let window_handle = RawWindowHandle::Wayland(handle);

        RawWaylandHandle(display_handle, window_handle)
    };

    let wgpu_surface = unsafe {
        wgpu_instance
            .create_surface(&handle)
            .context("failed to create wgpu surface")?
    };

    let adapter =
        futures::executor::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&wgpu_surface),
            ..Default::default()
        }))
        .context("failed to find suitable adapter")?;

    let (format, (device, queue)) = futures::executor::block_on(async {
        let adapter = wgpu::util::initialize_adapter_from_env_or_default(
            &wgpu_instance,
            Backends::GL | Backends::VULKAN,
            Some(&wgpu_surface),
        )
        .await
        .expect("Create adapter");

        let adapter_features = adapter.features();

        let needed_limits = wgpu::Limits::default();

        let capabilities = wgpu_surface.get_capabilities(&adapter);

        (
            capabilities
                .formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .or_else(|| capabilities.formats.first().copied())
                .expect("Get preferred format"),
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: adapter_features & wgpu::Features::default(),
                        limits: needed_limits,
                    },
                    None,
                )
                .await
                .expect("Request device"),
        )
    });

    let backend = iced_wgpu::Backend::new(
        &device,
        &queue,
        iced_wgpu::Settings {
            present_mode: wgpu::PresentMode::Mailbox,
            internal_backend: Backends::GL | Backends::VULKAN,
            ..Default::default()
        },
        format,
    );

    let mut renderer: Renderer<iced_wgpu::Backend, Theme> = Renderer::new(backend);

    let prog = iced_runtime::program::State::new(
        Prog,
        Size::new(width as f32, height as f32),
        &mut renderer,
        &mut Debug::new(),
    );

    let mut state = State {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &queue_handle),
        output_state: OutputState::new(&globals, &queue_handle),
        layer,
        width,
        height,
        viewport: Viewport::with_physical_size(Size::new(width, height), 1.0),
        exit: false,
        adapter,
        device,
        queue,
        surface: wgpu_surface,
        renderer,
        program: prog,
        clipboard: unsafe { WaylandClipboard::new(conn.backend().display_ptr() as *mut _) },
        keyboard_focus: false,
        keyboard_modifiers: Modifiers::default(),
        pointer: None,
        pointer_location: (0.0, 0.0),
        initial_configure_sent: false,
    };

    loop {
        // tracing::debug!("TOP OF LOOP");
        event_queue.blocking_dispatch(&mut state).unwrap();

        state.update_widgets();

        if state.exit {
            break;
        }
    }

    drop(state.surface);
    drop(state.layer);

    Ok(())
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        new_factor: i32,
    ) {
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, time: u32) {
        self.update_widgets();
        self.draw(qh);
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {}

    fn update_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {}

    fn output_destroyed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {}
}

impl LayerShellHandler for State {
    fn closed(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &wlr_layer::LayerSurface,
    ) {
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &wlr_layer::LayerSurface,
        configure: wlr_layer::LayerSurfaceConfigure,
        serial: u32,
    ) {
        let (new_width, new_height) = configure.new_size;
        if new_width != 0 {
            self.width = new_width;
        };
        if new_height != 0 {
            self.height = new_height;
        }

        let cap = self.surface.get_capabilities(&self.adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: cap.formats[0],
            width: self.width,
            height: self.height,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![cap.formats[0]],
        };

        self.surface.configure(&self.device, &surface_config);

        self.update_widgets();

        if !self.initial_configure_sent {
            self.initial_configure_sent = true;
            self.draw(qh);
        }
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: WlSeat) {}

    fn new_capability(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            let pointer = self.seat_state.get_pointer(qh, &seat).unwrap();
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: WlSeat) {}
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[u32],
    ) {
        if self.layer.wl_surface() != surface {
            return;
        }

        self.keyboard_focus = true;
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
    ) {
        if self.layer.wl_surface() != surface {
            return;
        }

        self.keyboard_focus = false;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        tracing::debug!("start of press_key");
        if !self.keyboard_focus {
            return;
        }

        let Some(keycode) = crate::keyboard::raw_key_to_keycode(event.raw_code) else {
            return;
        };

        let mut modifiers = iced_runtime::keyboard::Modifiers::default();

        let Modifiers {
            ctrl,
            alt,
            shift,
            caps_lock: _,
            logo,
            num_lock: _,
        } = &self.keyboard_modifiers;

        if *ctrl {
            modifiers |= iced_runtime::keyboard::Modifiers::CTRL;
        }
        if *alt {
            modifiers |= iced_runtime::keyboard::Modifiers::ALT;
        }
        if *shift {
            modifiers |= iced_runtime::keyboard::Modifiers::SHIFT;
        }
        if *logo {
            modifiers |= iced_runtime::keyboard::Modifiers::LOGO;
        }

        let event = iced::Event::Keyboard(iced_runtime::keyboard::Event::KeyPressed {
            key_code: keycode,
            modifiers,
        });

        self.program.queue_event(event);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if !self.keyboard_focus {
            return;
        }

        let Some(keycode) = crate::keyboard::raw_key_to_keycode(event.raw_code) else {
            return;
        };

        let mut modifiers = iced_runtime::keyboard::Modifiers::default();

        let Modifiers {
            ctrl,
            alt,
            shift,
            caps_lock: _,
            logo,
            num_lock: _,
        } = &self.keyboard_modifiers;

        if *ctrl {
            modifiers |= iced_runtime::keyboard::Modifiers::CTRL;
        }
        if *alt {
            modifiers |= iced_runtime::keyboard::Modifiers::ALT;
        }
        if *shift {
            modifiers |= iced_runtime::keyboard::Modifiers::SHIFT;
        }
        if *logo {
            modifiers |= iced_runtime::keyboard::Modifiers::LOGO;
        }

        let event = iced::Event::Keyboard(iced_runtime::keyboard::Event::KeyReleased {
            key_code: keycode,
            modifiers,
        });

        self.program.queue_event(event);
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        self.keyboard_modifiers = modifiers;
    }
}

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }

            let iced_event = match event.kind {
                PointerEventKind::Enter { .. } => {
                    iced::Event::Mouse(iced::mouse::Event::CursorEntered)
                }
                PointerEventKind::Leave { .. } => {
                    iced::Event::Mouse(iced::mouse::Event::CursorLeft)
                }
                PointerEventKind::Motion { .. } => {
                    self.pointer_location = event.position;
                    iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                        position: iced::Point {
                            x: event.position.0 as f32,
                            y: event.position.1 as f32,
                        },
                    })
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(button) = crate::pointer::button_to_iced_button(button) {
                        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button))
                    } else {
                        continue;
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(button) = crate::pointer::button_to_iced_button(button) {
                        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button))
                    } else {
                        continue;
                    }
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    source,
                    time: _,
                } => {
                    let delta = match source.unwrap() {
                        AxisSource::Wheel => ScrollDelta::Lines {
                            x: horizontal.discrete as f32,
                            y: vertical.discrete as f32,
                        },
                        AxisSource::Finger => ScrollDelta::Pixels {
                            x: horizontal.absolute as f32,
                            y: vertical.absolute as f32,
                        },
                        AxisSource::Continuous => ScrollDelta::Pixels {
                            x: horizontal.absolute as f32,
                            y: vertical.absolute as f32,
                        },
                        AxisSource::WheelTilt => ScrollDelta::Lines {
                            x: horizontal.discrete as f32,
                            y: vertical.discrete as f32,
                        },
                        _ => continue,
                    };
                    iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta })
                }
            };

            self.program.queue_event(iced_event);
        }
    }
}

delegate_compositor!(State);
delegate_output!(State);
delegate_seat!(State);
delegate_keyboard!(State);
delegate_pointer!(State);
delegate_layer!(State);
delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

impl State {
    pub fn draw(&mut self, queue_handle: &QueueHandle<Self>) {
        // tracing::debug!("DRAWING");
        match self.surface.get_current_texture() {
            Ok(frame) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // let program = self.program.program();

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                self.renderer.with_primitives(|backend, primitives| {
                    backend.present::<String>(
                        &self.device,
                        &self.queue,
                        &mut encoder,
                        Some(iced::Color::new(0.6, 0.6, 0.6, 1.0)),
                        &view,
                        primitives,
                        &self.viewport,
                        &[],
                    );
                });

                self.queue.submit(Some(encoder.finish()));
                frame.present();

                self.layer
                    .wl_surface()
                    .damage_buffer(0, 0, self.width as i32, self.height as i32);

                self.layer
                    .wl_surface()
                    .frame(queue_handle, self.layer.wl_surface().clone());

                self.layer.commit();
            }
            Err(_) => todo!(),
        }
    }

    pub fn update_widgets(&mut self) {
        let _ = self.program.update(
            self.viewport.logical_size(),
            iced::mouse::Cursor::Available(iced::Point {
                x: self.pointer_location.0 as f32,
                y: self.pointer_location.1 as f32,
            }),
            &mut self.renderer,
            &Theme::Dark,
            &iced_wgpu::core::renderer::Style {
                text_color: Color::WHITE,
            },
            &mut self.clipboard,
            &mut Debug::new(),
        );
    }
}
