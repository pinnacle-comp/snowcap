mod clipboard;
mod keyboard;
mod pointer;
mod widget;

use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use anyhow::Context;
use iced::{widget::slider, Theme};
use iced_wgpu::wgpu::{self, Backends};
use lazy_static::lazy_static;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use smithay_client_toolkit::{reexports::client::Connection, shell::wlr_layer::Anchor};
use tracing_subscriber::EnvFilter;
use widget::{SnowcapMessage, SnowcapWidget};

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

static RENDERER: OnceLock<Arc<Mutex<iced_wgpu::Renderer<Theme>>>> = OnceLock::new();

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

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let (mut state1, mut event_loop1) = SnowcapWidget::new(
        (256, 128),
        Anchor::TOP,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
    )
    .unwrap();

    let (mut state5, mut event_loop5) = SnowcapWidget::new(
        (256, 128),
        Anchor::BOTTOM,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
    )?;
    let (mut state4, mut event_loop4) = SnowcapWidget::new(
        (256, 128),
        Anchor::BOTTOM | Anchor::RIGHT,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
    )?;
    let (mut state3, mut event_loop3) = SnowcapWidget::new(
        (256, 128),
        Anchor::BOTTOM | Anchor::LEFT,
        Box::new(|_| {
            iced::widget::column![slider(0.0..=1.0, 0.5, |_| { SnowcapMessage::Nothing }).step(0.1)]
                .into()
        }),
    )?;

    let (mut state2, mut event_loop2) = SnowcapWidget::new(
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
    )?;

    state1.configure_wgpu_surface();
    state2.configure_wgpu_surface();
    state3.configure_wgpu_surface();
    state4.configure_wgpu_surface();
    state5.configure_wgpu_surface();

    loop {
        event_loop1.dispatch(Duration::ZERO, &mut state1)?;
        event_loop2.dispatch(Duration::ZERO, &mut state2)?;
        event_loop3.dispatch(Duration::ZERO, &mut state3)?;
        event_loop4.dispatch(Duration::ZERO, &mut state4)?;
        event_loop5.dispatch(Duration::ZERO, &mut state5)?;

        if false {
            break;
        }
    }

    drop(state1.surface);
    drop(state1.layer);

    Ok(())
}
