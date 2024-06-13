pub mod input;
pub mod layer;
pub mod snowcap;
pub mod util;
pub mod widget;

pub use xkbcommon;

use std::path::PathBuf;

use futures::{stream::FuturesUnordered, Future};
use layer::Layer;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
    task::JoinHandle,
};
use tokio_stream::StreamExt;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

fn socket_dir() -> PathBuf {
    xdg::BaseDirectories::with_prefix("snowcap")
        .and_then(|xdg| xdg.get_runtime_directory().cloned())
        .unwrap_or(PathBuf::from("/tmp"))
}

fn socket_name() -> String {
    let wayland_suffix = std::env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
    format!("snowcap-grpc-{wayland_suffix}.sock")
}

pub async fn connect(
) -> Result<(Layer, UnboundedReceiver<JoinHandle<()>>), Box<dyn std::error::Error>> {
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            tokio::net::UnixStream::connect(socket_dir().join(socket_name()))
        }))
        .await?;

    let (fut_sender, fut_recv) = unbounded_channel::<JoinHandle<()>>();

    let layer = Layer::new(channel.clone(), fut_sender.clone());

    Ok((layer, fut_recv))
}

pub async fn listen(mut recv: UnboundedReceiver<JoinHandle<()>>) {
    let mut set = FuturesUnordered::new();

    loop {
        tokio::select! {
            handle = recv.recv() => {
                if let Some(handle) = handle {
                    set.push(handle);
                }
            }
                _ = set.next() => ()
        }
    }
}

pub(crate) fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(future)
    })
}
