use std::sync::{Arc, Mutex};

use anyhow::Context;
use iced::{mouse::ScrollDelta, Color, Size, Theme};
use iced_runtime::{Command, Debug, Program};
use iced_wgpu::{
    core::Element,
    graphics::{Renderer, Viewport},
    wgpu::{self, Backends},
};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::EventLoop,
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

use crate::{
    clipboard::WaylandClipboard, RawWaylandHandle, ADAPTER, CONN, DEVICE, INSTANCE, QUEUE, RENDERER,
};

pub struct SnowcapWidgetProgram {
    pub widgets: WidgetFn,
}

pub type WidgetFn = Box<
    dyn Fn(&SnowcapWidgetProgram) -> Element<'static, SnowcapMessage, iced_wgpu::Renderer<Theme>>,
>;

pub struct SnowcapWidget {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,

    pub widget: iced_runtime::program::State<SnowcapWidgetProgram>,
    pub layer: LayerSurface,
    pub width: u32,
    pub height: u32,
    pub viewport: Viewport,
    pub capabilities: wgpu::SurfaceCapabilities,

    pub surface: wgpu::Surface,

    pub clipboard: WaylandClipboard,

    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub keyboard_focus: bool,
    pub keyboard_modifiers: Modifiers,

    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_location: (f64, f64),

    pub initial_configure_sent: bool,
}

impl SnowcapWidget {
    pub fn new(
        (width, height): (u32, u32),
        anchor: Anchor,
        widgets: WidgetFn,
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        debug!("top of State::new");
        debug!("init registry");
        let (globals, event_queue) =
            registry_queue_init::<Self>(&CONN).context("failed to init registry queue")?;

        let queue_handle = event_queue.handle();

        debug!("create loop");
        let event_loop = EventLoop::<Self>::try_new()?;
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

        debug!("get capabilities"); // PERF: SLOW
        let capabilities = wgpu_surface.get_capabilities(&ADAPTER);
        let renderer = RENDERER.get_or_init(|| {
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
            Arc::new(Mutex::new(renderer))
        });

        let mut ren = renderer.lock().unwrap();

        let state = iced_runtime::program::State::new(
            SnowcapWidgetProgram { widgets },
            Size::new(width as f32, height as f32),
            &mut ren,
            &mut Debug::new(),
        );

        debug!("create state");
        let state = SnowcapWidget {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &queue_handle),
            output_state: OutputState::new(&globals, &queue_handle),

            widget: state,
            layer,
            width,
            height,
            viewport: Viewport::with_physical_size(Size::new(width, height), 1.0),
            capabilities,

            surface: wgpu_surface,

            clipboard: unsafe { WaylandClipboard::new(CONN.backend().display_ptr() as *mut _) },

            keyboard: None,
            keyboard_focus: false,
            keyboard_modifiers: Modifiers::default(),

            pointer: None,
            pointer_location: (0.0, 0.0),

            initial_configure_sent: false,
        };

        drop(ren);

        Ok((state, event_loop))
    }

    pub fn configure_wgpu_surface(&self) {
        let capabilities = &self.capabilities;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: capabilities.formats[0],
            width: self.width,
            height: self.height,
            present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![capabilities.formats[0]],
        };

        self.surface.configure(&DEVICE, &surface_config);
    }
}

impl CompositorHandler for SnowcapWidget {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        tracing::trace!("CompositorHandler::frame");
        self.update_widgets();
        self.draw(qh, surface);
    }
}

impl OutputHandler for SnowcapWidget {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {}

    fn update_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {}

    fn output_destroyed(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: WlOutput) {}
}

impl LayerShellHandler for SnowcapWidget {
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
        debug!("update_widgets");
        self.update_widgets();

        if !self.initial_configure_sent {
            self.initial_configure_sent = true;
            self.draw(qh, layer.wl_surface());
        }
    }
}

impl SeatHandler for SnowcapWidget {
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

impl KeyboardHandler for SnowcapWidget {
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

impl PointerHandler for SnowcapWidget {
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

delegate_compositor!(SnowcapWidget);
delegate_output!(SnowcapWidget);
delegate_seat!(SnowcapWidget);
delegate_keyboard!(SnowcapWidget);
delegate_pointer!(SnowcapWidget);
delegate_layer!(SnowcapWidget);
delegate_registry!(SnowcapWidget);

impl ProvidesRegistryState for SnowcapWidget {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState, SeatState);
}

impl SnowcapWidget {
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

                let renderer = RENDERER.get().unwrap();
                let mut renderer = renderer.lock().unwrap();
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
        let renderer = RENDERER.get().unwrap();
        let mut renderer = renderer.lock().unwrap();
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

#[derive(Debug, Clone)]
pub enum SnowcapMessage {
    Nothing,
}

impl Program for SnowcapWidgetProgram {
    type Renderer = iced_wgpu::Renderer<Theme>;

    type Message = SnowcapMessage;

    fn update(&mut self, message: Self::Message) -> iced_runtime::Command<Self::Message> {
        match message {
            SnowcapMessage::Nothing => (),
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        (self.widgets)(self)
    }
}
