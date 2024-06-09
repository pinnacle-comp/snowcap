use std::sync::Arc;

use anyhow::Context;
use iced_wgpu::{
    wgpu::{self, Backends},
    Backend,
};

use crate::block_on_tokio;

pub struct Wgpu {
    pub instance: Arc<wgpu::Instance>,
    pub adapter: Arc<wgpu::Adapter>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub renderer: iced_wgpu::Renderer,
}

pub fn setup_wgpu() -> anyhow::Result<Wgpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL | wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = block_on_tokio(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .context("no adapter")?;

    let (device, queue) = block_on_tokio(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(), // TODO:
            required_limits: wgpu::Limits::downlevel_defaults(),
        },
        None,
    ))?;

    let backend = Backend::new(
        &device,
        &queue,
        iced_wgpu::Settings {
            present_mode: wgpu::PresentMode::Mailbox,
            internal_backend: Backends::GL | Backends::VULKAN,
            ..Default::default()
        },
        wgpu::TextureFormat::Bgra8UnormSrgb,
    );

    let renderer = iced_wgpu::Renderer::new(backend, Default::default(), iced::Pixels(12.0));

    Ok(Wgpu {
        instance: Arc::new(instance),
        adapter: Arc::new(adapter),
        device: Arc::new(device),
        queue: Arc::new(queue),
        renderer,
    })
}
