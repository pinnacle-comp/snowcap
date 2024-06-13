pub mod api;
pub mod clipboard;
pub mod handlers;
pub mod input;
pub mod layer;
pub mod runtime;
pub mod server;
pub mod state;
pub mod util;
pub mod wgpu;
pub mod widget;

use std::time::Duration;

use futures::Future;
use server::socket_dir;
use smithay_client_toolkit::{
    reexports::calloop::{self, EventLoop},
    shell::WaylandSurface,
};
use state::State;

pub fn start(kill_ping: Option<calloop::ping::PingSource>) {
    let mut event_loop = EventLoop::<State>::try_new().unwrap();

    let mut state = State::new(event_loop.handle(), event_loop.get_signal()).unwrap();

    state.start_grpc_server(socket_dir()).unwrap();

    if let Some(kill_ping) = kill_ping {
        let loop_signal = event_loop.get_signal();

        event_loop
            .handle()
            .insert_source(kill_ping, move |_, _, _| {
                loop_signal.stop();
                loop_signal.wakeup();
            })
            .unwrap();
    }

    event_loop
        .run(Duration::from_secs(1), &mut state, |state| {
            let keyboard_focus_is_dead =
                state
                    .keyboard_focus
                    .as_ref()
                    .is_some_and(|focus| match focus {
                        handlers::keyboard::KeyboardFocus::Layer(layer) => {
                            !state.layers.iter().any(|sn_layer| &sn_layer.layer == layer)
                        }
                    });
            if keyboard_focus_is_dead {
                state.keyboard_focus = None;
            }

            for layer in state.layers.iter_mut() {
                if !layer.widgets.is_queue_empty() {
                    layer
                        .layer
                        .wl_surface()
                        .frame(&state.queue_handle, layer.layer.wl_surface().clone());
                }
            }
        })
        .unwrap();
}

fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
