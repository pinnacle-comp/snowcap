use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::Context;
use iced::{
    mouse::ScrollDelta,
    widget::{Button, Column, Slider, Text},
    Color, Size, Theme,
};
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

use crate::{api::msg::WidgetDefinition, clipboard::WaylandClipboard, RawWaylandHandle};

pub struct SnowcapWidgetProgram {
    pub widgets: WidgetFn,
    pub widget_state: HashMap<u32, WidgetStates>,
}

pub type WidgetFn = Box<
    dyn Fn(&SnowcapWidgetProgram) -> Element<'static, SnowcapMessage, iced_wgpu::Renderer<Theme>>,
>;

pub struct SnowcapWidget {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,

    // surface must be dropped before layer
    pub surface: wgpu::Surface,
    pub dirty: bool,

    pub widget: iced_runtime::program::State<SnowcapWidgetProgram>,
    pub layer: LayerSurface,
    pub width: u32,
    pub height: u32,
    pub viewport: Viewport,
    pub capabilities: wgpu::SurfaceCapabilities,

    pub clipboard: WaylandClipboard,

    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub keyboard_focus: bool,
    pub keyboard_modifiers: Modifiers,

    pub pointer: Option<wl_pointer::WlPointer>,
    pub pointer_location: (f64, f64),

    pub initial_configure_sent: bool,

    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
    pub renderer: Rc<RefCell<iced_wgpu::Renderer<Theme>>>,
}

impl SnowcapWidget {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        conn: &Connection,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        renderer: &OnceCell<Rc<RefCell<iced_wgpu::Renderer<Theme>>>>,
        (width, height): (u32, u32),
        anchor: Anchor,
        widget_def: WidgetDefinition,
    ) -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        debug!("top of State::new");
        debug!("init registry");
        let (globals, event_queue) =
            registry_queue_init::<Self>(conn).context("failed to init registry queue")?;

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
            handle.display = conn.backend().display_ptr() as *mut _;
            let display_handle = RawDisplayHandle::Wayland(handle);

            let mut handle = WaylandWindowHandle::empty();
            handle.surface = layer.wl_surface().id().as_ptr() as *mut _;
            let window_handle = RawWindowHandle::Wayland(handle);

            RawWaylandHandle(display_handle, window_handle)
        };

        debug!("create wgpu surface");
        let wgpu_surface = unsafe { instance.create_surface(&wayland_handle).unwrap() };

        debug!("get capabilities"); // PERF: SLOW
        let capabilities = wgpu_surface.get_capabilities(adapter);
        let renderer = renderer.get_or_init(|| {
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
                &device,
                &queue,
                iced_wgpu::Settings {
                    present_mode: wgpu::PresentMode::Mailbox,
                    internal_backend: Backends::GL | Backends::VULKAN,
                    ..Default::default()
                },
                format,
            );

            debug!("create iced renderer");
            let renderer: Renderer<iced_wgpu::Backend, Theme> = Renderer::new(backend);
            Rc::new(RefCell::new(renderer))
        });

        let WidgetDefinitionReturn { widget, states } = widget_def.into_widget();

        let state = {
            let mut ren = renderer.borrow_mut();

            iced_runtime::program::State::new(
                SnowcapWidgetProgram {
                    widgets: widget,
                    widget_state: states,
                },
                Size::new(width as f32, height as f32),
                &mut ren,
                &mut Debug::new(), // TODO:
            )
        };

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

            dirty: true,

            surface: wgpu_surface,

            clipboard: unsafe { WaylandClipboard::new(conn.backend().display_ptr() as *mut _) },

            keyboard: None,
            keyboard_focus: false,
            keyboard_modifiers: Modifiers::default(),

            pointer: None,
            pointer_location: (0.0, 0.0),

            initial_configure_sent: false,

            device: device.clone(),
            queue,
            renderer: renderer.clone(),
        };

        state.configure_wgpu_surface(&device);

        Ok((state, event_loop))
    }

    pub fn configure_wgpu_surface(&self, device: &wgpu::Device) {
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

        self.surface.configure(device, &surface_config);
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
        // if !self.dirty {
        //     return;
        // }
        match self.surface.get_current_texture() {
            Ok(frame) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                {
                    let mut renderer = self.renderer.borrow_mut();
                    renderer.with_primitives(|backend, primitives| {
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
                }

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
        self.dirty = false;
    }

    pub fn update_widgets(&mut self) {
        tracing::trace!("State::update_widgets");
        let mut renderer = self.renderer.borrow_mut();
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
    Update(u32, WidgetStates),
}

impl Program for SnowcapWidgetProgram {
    type Renderer = iced_wgpu::Renderer<Theme>;

    type Message = SnowcapMessage;

    fn update(&mut self, message: Self::Message) -> iced_runtime::Command<Self::Message> {
        match message {
            SnowcapMessage::Nothing => (),
            SnowcapMessage::Update(index, state) => {
                self.widget_state.insert(index, state);
            }
        }

        Command::none()

        // Command::perform(async { println!("hello from future") }, |_| {
        //     SnowcapMessage::Nothing
        // })
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        (self.widgets)(self)
    }
}

pub struct WidgetDefinitionReturn {
    widget: WidgetFn,
    states: HashMap<u32, WidgetStates>,
}

static WIDGET_ID: AtomicU32 = AtomicU32::new(0);

impl WidgetDefinition {
    pub fn into_widget(self) -> WidgetDefinitionReturn {
        let mut states = HashMap::<u32, WidgetStates>::new();

        WIDGET_ID.store(0, Ordering::Relaxed);

        let widget = self.into_widget_inner(&mut states);

        WidgetDefinitionReturn { widget, states }
    }

    pub fn into_widget_inner(self, states: &mut HashMap<u32, WidgetStates>) -> WidgetFn {
        let index = WIDGET_ID.fetch_add(1, Ordering::Relaxed);

        match self {
            WidgetDefinition::Slider {
                range_start,
                range_end,
                // value,
                on_change,
                on_release,
                width,
                height,
                step,
            } => {
                let widget_state = WidgetStates::Slider(range_start);

                states.insert(index, widget_state);

                let f: WidgetFn = Box::new(move |prog| {
                    let slider = Slider::new(
                        range_start..=range_end,
                        prog.widget_state.get(&index).unwrap().assume_slider(),
                        move |val| SnowcapMessage::Update(index, WidgetStates::Slider(val)),
                    )
                    .width(width)
                    .height(height)
                    .step(step);

                    slider.into()
                });

                f
            }
            WidgetDefinition::Column {
                spacing,
                padding,
                width,
                height,
                max_width,
                alignment,
                children,
            } => {
                let children_widget_fns = children
                    .into_iter()
                    .map(move |def| def.into_widget_inner(states))
                    .collect::<Vec<_>>();

                let f: WidgetFn = Box::new(move |prog| {
                    let mut column = Column::new()
                        .spacing(spacing)
                        .padding(padding)
                        .width(width)
                        .height(height)
                        .max_width(max_width)
                        .align_items(alignment);

                    for child in children_widget_fns.iter() {
                        column = column.push(child(prog));
                    }

                    column.into()
                });

                f
            }
            WidgetDefinition::Button {
                width,
                height,
                padding,
                child,
            } => {
                let child = child.into_widget_inner(states);

                let f: WidgetFn = Box::new(move |prog| {
                    let button = Button::new(child(prog))
                        .width(width)
                        .height(height)
                        .padding(padding)
                        .on_press(SnowcapMessage::Nothing);

                    button.into()
                });

                f
            }
            WidgetDefinition::Text { text } => {
                let f: WidgetFn = Box::new(move |_prog| {
                    let text = Text::new(text.clone()); // PERF: find a way to not clone everytime
                                                        // |     this function is called

                    text.into()
                });

                f
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum WidgetStates {
    Slider(f32),
}

impl WidgetStates {
    pub fn assume_slider(&self) -> f32 {
        match self {
            WidgetStates::Slider(f) => *f,
            _ => panic!(),
        }
    }
}
