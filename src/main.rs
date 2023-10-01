mod clipboard;
mod keyboard;
mod pointer;
mod widget;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use clipboard::WaylandClipboard;
use iced::{mouse::ScrollDelta, widget::slider, Color, Size, Theme};
use iced_runtime::Debug;
use iced_wgpu::{
    graphics::{Renderer, Viewport},
    wgpu::{self, Backends, SurfaceCapabilities},
};
use lazy_static::lazy_static;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{EventLoop, LoopHandle, LoopSignal},
        client::{
            globals::registry_queue_init,
            protocol::{
                wl_keyboard::{self, WlKeyboard},
                wl_output::WlOutput,
                wl_pointer::{self, AxisSource, WlPointer},
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
            Connection, Proxy, QueueHandle, WaylandSource,
        },
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
use tracing::{debug, trace};
use tracing_subscriber::EnvFilter;
use widget::{SnowcapMessage, SnowcapWidgetProgram, WidgetFn};

lazy_static! {
    static ref CONN: Connection = Connection::connect_to_env()
        .context("failed to connect to wayland")
        .unwrap();
    static ref INSTANCE: wgpu::Instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL | wgpu::Backends::VULKAN,
        ..Default::default()
    });
    static ref ADAPTER: wgpu::Adapter = futures::executor::block_on(async {
        wgpu::util::initialize_adapter_from_env_or_default(
            &INSTANCE,
            Backends::GL | Backends::VULKAN,
            None,
        )
        .await
        .unwrap()
    });
    static ref _DEVICE_AND_QUEUE: (wgpu::Device, wgpu::Queue) =
        futures::executor::block_on(async {
            let adapter_features = ADAPTER.features();
            let needed_limits = wgpu::Limits::default();
            ADAPTER
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        features: adapter_features & wgpu::Features::default(),
                        limits: needed_limits,
                    },
                    None,
                )
                .await
                .expect("Request device")
        });
    static ref DEVICE: &'static wgpu::Device = &_DEVICE_AND_QUEUE.0;
    static ref QUEUE: &'static wgpu::Queue = &_DEVICE_AND_QUEUE.1;
}

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
    compositor_state: CompositorState,
    layer_shell: LayerShell,

    widget: iced_runtime::program::State<SnowcapWidgetProgram>,
    layer: LayerSurface,
    width: u32,
    height: u32,
    viewport: Viewport,

    exit: bool,
    loop_handle: LoopHandle<'static, Self>,
    loop_signal: LoopSignal,

    surface: wgpu::Surface,
    renderer: Arc<Mutex<iced_wgpu::Renderer<Theme>>>,
    capabilities: Arc<SurfaceCapabilities>,

    clipboard: WaylandClipboard,

    keyboard: Option<wl_keyboard::WlKeyboard>,
    keyboard_focus: bool,
    keyboard_modifiers: Modifiers,

    pointer: Option<wl_pointer::WlPointer>,
    pointer_location: (f64, f64),

    initial_configure_sent: bool,
}

impl State {
    pub fn new(
        (width, height): (u32, u32),
        anchor: Anchor,
        widgets: WidgetFn,
        renderer: Option<Arc<Mutex<Renderer<iced_wgpu::Backend, Theme>>>>,
        capabilities: Option<Arc<SurfaceCapabilities>>,
    ) -> anyhow::Result<(
        Self,
        EventLoop<'static, Self>,
        Arc<Mutex<Renderer<iced_wgpu::Backend, Theme>>>,
    )> {
        debug!("top of State::new");
        debug!("init registry");
        let (globals, event_queue) =
            registry_queue_init::<State>(&CONN).context("failed to init registry queue")?;

        let queue_handle = event_queue.handle();

        debug!("create loop");
        let event_loop = EventLoop::<State>::try_new()?;
        let loop_handle = event_loop.handle();
        debug!("create wayland source");
        WaylandSource::new(event_queue)?
            .insert(loop_handle.clone())
            .expect("failed to insert wayland source into event loop");

        debug!("bind globals");
        let compositor = CompositorState::bind(&globals, &queue_handle)
            .context("wl_compositor not availible")?;
        let layer_shell =
            LayerShell::bind(&globals, &queue_handle).context("layer shell not availible")?;

        debug!("create layer surface");
        let surface = compositor.create_surface(&queue_handle);
        let layer = layer_shell.create_layer_surface(
            &queue_handle,
            surface,
            wlr_layer::Layer::Overlay,
            Some("snowcap_layer"),
            None,
        );

        layer.set_keyboard_interactivity(wlr_layer::KeyboardInteractivity::OnDemand);

        layer.set_size(width, height);
        layer.set_anchor(anchor);

        layer.commit();

        debug!("create wayland handle");
        let wayland_handle = {
            let mut handle = WaylandDisplayHandle::empty();
            handle.display = CONN.backend().display_ptr() as *mut _;
            let display_handle = RawDisplayHandle::Wayland(handle);

            let mut handle = WaylandWindowHandle::empty();
            handle.surface = layer.wl_surface().id().as_ptr() as *mut _;
            let window_handle = RawWindowHandle::Wayland(handle);

            RawWaylandHandle(display_handle, window_handle)
        };

        debug!("create wgpu surface");
        let wgpu_surface = unsafe { INSTANCE.create_surface(&wayland_handle).unwrap() };

        let (renderer, capabilities) =
            if let (Some(renderer), Some(capabilities)) = (renderer, capabilities) {
                debug!("create iced state");
                (renderer, capabilities)
            } else {
                debug!("get capabilities"); // PERF: SLOW
                let capabilities = wgpu_surface.get_capabilities(&ADAPTER);
                debug!("get texture format");
                let format = capabilities
                    .formats
                    .iter()
                    .copied()
                    .find(wgpu::TextureFormat::is_srgb)
                    .or_else(|| capabilities.formats.first().copied())
                    .expect("Get preferred format");

                tracing::debug!("---------FORMAT IS {format:?}");

                // TODO: speed up
                debug!("create iced backend"); // PERF: SLOW
                let backend = iced_wgpu::Backend::new(
                    &DEVICE,
                    &QUEUE,
                    iced_wgpu::Settings {
                        present_mode: wgpu::PresentMode::Mailbox,
                        internal_backend: Backends::GL | Backends::VULKAN,
                        ..Default::default()
                    },
                    format,
                );

                debug!("create iced renderer");
                let renderer: Renderer<iced_wgpu::Backend, Theme> = Renderer::new(backend);
                (Arc::new(Mutex::new(renderer)), Arc::new(capabilities))
            };

        let mut ren = renderer.lock().unwrap();

        let state = iced_runtime::program::State::new(
            SnowcapWidgetProgram { widgets },
            Size::new(width as f32, height as f32),
            &mut ren,
            &mut Debug::new(),
        );

        debug!("create state");
        let state = State {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &queue_handle),
            output_state: OutputState::new(&globals, &queue_handle),
            compositor_state: compositor,
            layer_shell,

            widget: state,
            layer,
            width,
            height,
            viewport: Viewport::with_physical_size(Size::new(width, height), 1.0),

            renderer: renderer.clone(),
            surface: wgpu_surface,
            capabilities,

            exit: false,
            loop_handle,
            loop_signal: event_loop.get_signal(),

            clipboard: unsafe { WaylandClipboard::new(CONN.backend().display_ptr() as *mut _) },

            keyboard: None,
            keyboard_focus: false,
            keyboard_modifiers: Modifiers::default(),

            pointer: None,
            pointer_location: (0.0, 0.0),

            initial_configure_sent: false,
        };

        drop(ren);

        Ok((state, event_loop, renderer))
    }
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let (mut state1, mut event_loop1, renderer) = State::new(
        (256, 128),
        Anchor::TOP,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
        None,
        None,
    )?;
    let (mut state5, mut event_loop5, _) = State::new(
        (256, 128),
        Anchor::BOTTOM,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
        Some(renderer.clone()),
        Some(state1.capabilities.clone()),
    )?;
    let (mut state4, mut event_loop4, _) = State::new(
        (256, 128),
        Anchor::BOTTOM | Anchor::RIGHT,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
        Some(renderer.clone()),
        Some(state1.capabilities.clone()),
    )?;
    let (mut state3, mut event_loop3, _) = State::new(
        (256, 128),
        Anchor::BOTTOM | Anchor::LEFT,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
        Some(renderer.clone()),
        Some(state1.capabilities.clone()),
    )?;

    let (mut state2, mut event_loop2, _) = State::new(
        (128, 256),
        Anchor::TOP | Anchor::LEFT,
        Box::new(|_| {
            iced::widget::column![
                slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1),
                slider(0.0..=1.0, 0.2, |_| { SnowcapMessage::Nothing }).step(0.1),
                slider(0.0..=1.0, 0.6, |_| { SnowcapMessage::Nothing }).step(0.1),
            ]
            .into()
        }),
        Some(renderer.clone()),
        Some(state1.capabilities.clone()),
    )?;

    loop {
        event_loop1.dispatch(Duration::ZERO, &mut state1)?;
        event_loop2.dispatch(Duration::ZERO, &mut state2)?;
        event_loop3.dispatch(Duration::ZERO, &mut state3)?;
        event_loop4.dispatch(Duration::ZERO, &mut state4)?;
        event_loop5.dispatch(Duration::ZERO, &mut state5)?;

        if state1.exit || state2.exit {
            break;
        }
    }

    drop(state1.surface);
    drop(state1.layer);

    Ok(())
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, time: u32) {
        tracing::trace!("CompositorHandler::frame");
        self.update_widgets();
        self.draw(qh, surface);
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
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &wlr_layer::LayerSurface,
    ) {
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &wlr_layer::LayerSurface,
        configure: wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        debug!("configure");
        let (new_width, new_height) = configure.new_size;
        if new_width != 0 {
            self.width = new_width;
        };
        if new_height != 0 {
            self.height = new_height;
        }

        debug!("get capabilities");
        let cap = &self.capabilities;
        debug!("create surface_config");
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: cap.formats[0],
            width: self.width,
            height: self.height,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![cap.formats[0]],
        };

        debug!("configure surface"); // PERF: SLOW
        self.surface.configure(&DEVICE, &surface_config);

        debug!("update_widgets");
        self.update_widgets();

        if !self.initial_configure_sent {
            self.initial_configure_sent = true;
            self.draw(qh, layer.wl_surface());
        }
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).unwrap();
            self.keyboard = Some(keyboard);
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).unwrap();
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {}
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
        debug!("start of press_key");
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

        self.widget.queue_event(event);
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

        self.widget.queue_event(event);
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
        trace!("pointer_frame");
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

            self.widget.queue_event(iced_event);
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
    pub fn draw(&mut self, queue_handle: &QueueHandle<Self>, surface: &WlSurface) {
        tracing::trace!("State::draw");
        if self.layer.wl_surface() != surface {
            return;
        }
        match self.surface.get_current_texture() {
            Ok(frame) => {
                let mut encoder =
                    DEVICE.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // let program = self.program.program();

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut renderer = self.renderer.lock().unwrap();
                renderer.with_primitives(|backend, primitives| {
                    backend.present::<String>(
                        &DEVICE,
                        &QUEUE,
                        &mut encoder,
                        Some(iced::Color::new(0.6, 0.6, 0.6, 1.0)),
                        &view,
                        primitives,
                        &self.viewport,
                        &[],
                    );
                });

                QUEUE.submit(Some(encoder.finish()));
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
        tracing::trace!("State::update_widgets");
        let mut renderer = self.renderer.lock().unwrap();
        let _ = self.widget.update(
            self.viewport.logical_size(),
            iced::mouse::Cursor::Available(iced::Point {
                x: self.pointer_location.0 as f32,
                y: self.pointer_location.1 as f32,
            }),
            &mut renderer,
            &Theme::Dark,
            &iced_wgpu::core::renderer::Style {
                text_color: Color::WHITE,
            },
            &mut self.clipboard,
            &mut Debug::new(),
        );
    }
}
