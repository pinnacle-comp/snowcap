mod api;
mod clipboard;
mod handlers;
mod layer;
mod state;
mod util;
mod wgpu;
mod widget;

use std::future::Future;

use anyhow::Context;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::EventLoop,
        calloop_wayland_source::WaylandSource,
        client::{globals::registry_queue_init, Connection},
    },
    registry::RegistryState,
    seat::SeatState,
    shell::wlr_layer::LayerShell,
};
use state::State;
use tracing_subscriber::EnvFilter;
use wgpu::setup_wgpu;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let conn = Connection::connect_to_env().context("failed to establish wayland connection")?;

    let (globals, event_queue) =
        registry_queue_init::<State>(&conn).context("failed to init registry queue")?;
    let queue_handle = event_queue.handle();

    let layer_shell_state = LayerShell::bind(&globals, &queue_handle).unwrap();

    let seat_state = SeatState::new(&globals, &queue_handle);

    let registry_state = RegistryState::new(&globals);

    let output_state = OutputState::new(&globals, &queue_handle);

    let compositor_state = CompositorState::bind(&globals, &queue_handle).unwrap();

    let mut event_loop = EventLoop::<State>::try_new().unwrap();
    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .unwrap();

    let mut state = State {
        loop_handle: event_loop.handle(),
        conn: conn.clone(),
        registry_state,
        seat_state,
        output_state,
        compositor_state,
        layer_shell_state,
        queue_handle,
        wgpu: setup_wgpu().unwrap(),
        layers: Vec::new(),
        keyboard_focus: None,
        keyboard_modifiers: smithay_client_toolkit::seat::keyboard::Modifiers::default(),
        keyboard: None,
        pointer: None,
    };

    state.start_grpc_server("/tmp").unwrap();

    event_loop.run(None, &mut state, |_state| {}).unwrap();

    Ok(())
}

fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
