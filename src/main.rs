mod api;
mod clipboard;
mod handlers;
mod input;
mod layer;
mod runtime;
mod server;
mod state;
mod util;
mod wgpu;
mod widget;

use std::future::Future;

use server::socket_dir;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use state::State;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("snowcap=info"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .init();

    let mut event_loop = EventLoop::<State>::try_new().unwrap();

    let mut state = State::new(event_loop.handle()).unwrap();

    state.start_grpc_server(socket_dir()).unwrap();

    event_loop
        .run(None, &mut state, |state| {
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
        })
        .unwrap();

    Ok(())
}

fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
