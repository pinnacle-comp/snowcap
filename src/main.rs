mod api;
mod clipboard;
mod keyboard;
mod pointer;
mod widget;

use std::{
    cell::{OnceCell, RefCell},
    os::unix::net::UnixStream,
    path::Path,
    rc::Rc,
    time::Duration,
};

use api::{
    msg::{Msg, WidgetDefinition},
    SnowcapSocketSource,
};
use iced::Theme;
use iced_wgpu::wgpu::{self, Backends};
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use smithay_client_toolkit::{
    reexports::{
        calloop::{self, EventLoop, LoopHandle},
        client::Connection,
    },
    shell::wlr_layer::Anchor,
};
use tracing_subscriber::EnvFilter;
use widget::SnowcapWidget;

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

pub struct State {
    widgets: Vec<(SnowcapWidget, EventLoop<'static, SnowcapWidget>)>,

    stream: Option<UnixStream>,

    /// The Wayland connection.
    conn: Connection,

    // wgpu stuff
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    renderer: OnceCell<Rc<RefCell<iced_wgpu::Renderer<Theme>>>>,

    loop_handle: LoopHandle<'static, Self>,
}

impl State {
    pub fn new() -> anyhow::Result<(Self, EventLoop<'static, Self>)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL | wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let adapter = futures::executor::block_on(async {
            wgpu::util::initialize_adapter_from_env_or_default(
                &instance,
                Backends::GL | Backends::VULKAN,
                None,
            )
            .await
            .unwrap()
        });

        let (device, queue) = futures::executor::block_on(async {
            let adapter_features = adapter.features();
            let needed_limits = wgpu::Limits::default();
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
                .expect("Request device")
        });

        let event_loop = EventLoop::<State>::try_new()?;

        let (sender, channel) = calloop::channel::channel::<Msg>();

        let source = SnowcapSocketSource::new(sender, Path::new("/tmp"))?;

        event_loop
            .handle()
            .insert_source(source, |stream, _, state| {
                tracing::debug!("got new stream");
                if let Some(stream) = state.stream.replace(stream) {
                    if let Err(err) = stream.shutdown(std::net::Shutdown::Both) {
                        tracing::error!("Error shutting down stream: {err}");
                    }
                }
            })?;

        event_loop
            .handle()
            .insert_source(channel, |msg, _, state| {
                use calloop::channel::Event;
                match msg {
                    Event::Msg(msg) => state.handle_msg(msg),
                    Event::Closed => todo!(),
                }
            })
            .unwrap();

        Ok((
            State {
                widgets: vec![],
                stream: None,
                conn: Connection::connect_to_env()?,
                instance,
                adapter,
                device: Rc::new(device),
                queue: Rc::new(queue),
                renderer: OnceCell::new(),
                loop_handle: event_loop.handle(),
            },
            event_loop,
        ))
    }

    pub fn new_widget(
        &mut self,
        (width, height): (u32, u32),
        anchor: Anchor,
        widget_def: WidgetDefinition,
    ) {
        if let Ok(widget) = SnowcapWidget::new(
            &self.conn,
            &self.instance,
            &self.adapter,
            self.device.clone(),
            self.queue.clone(),
            &self.renderer,
            (width, height),
            anchor,
            widget_def,
            &self.loop_handle,
        ) {
            self.widgets.push(widget);
        }
    }

    pub fn configure_wgpu_surfaces(&self) {
        for (widget, _) in self.widgets.iter() {
            widget.configure_wgpu_surface(&self.device);
        }
    }

    pub fn dispatch_loops(&mut self) -> anyhow::Result<()> {
        for (widget, event_loop) in self.widgets.iter_mut() {
            event_loop.dispatch(Duration::ZERO, widget)?;
        }
        Ok(())
    }

    pub fn handle_msg(&mut self, msg: Msg) {
        match msg {
            Msg::NewWidget {
                widget,
                width,
                height,
                anchor,
            } => {
                let anchor = match anchor {
                    api::msg::Anchor::Top => Anchor::TOP,
                    api::msg::Anchor::Bottom => Anchor::BOTTOM,
                    api::msg::Anchor::Left => Anchor::LEFT,
                    api::msg::Anchor::Right => Anchor::RIGHT,
                    api::msg::Anchor::TopRight => Anchor::TOP | Anchor::RIGHT,
                    api::msg::Anchor::TopLeft => Anchor::TOP | Anchor::LEFT,
                    api::msg::Anchor::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
                    api::msg::Anchor::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
                };
                self.new_widget((width, height), anchor, widget);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let (mut state, mut state_loop) = State::new()?;

    loop {
        state_loop.dispatch(Duration::ZERO, &mut state)?;
        state.dispatch_loops()?;

        if false {
            break;
        }
    }

    Ok(())
}
