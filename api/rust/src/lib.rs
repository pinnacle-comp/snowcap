pub mod layer;
pub mod snowcap;
pub mod widget;

use futures::{future::BoxFuture, Future};
use layer::Layer;
use tokio::sync::mpsc::unbounded_channel;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

pub async fn connect() -> Result<Layer, Box<dyn std::error::Error>> {
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            tokio::net::UnixStream::connect(
                std::env::var("SNOWCAP_GRPC_SOCKET")
                    .expect("SNOWCAP_GRPC_SOCKET was not set; is Snowcap running?"),
            )
        }))
        .await?;

    let (fut_sender, fut_recv) = unbounded_channel::<BoxFuture<'static, ()>>();

    let layer = Layer::new(channel.clone());

    Ok(layer)
}

pub(crate) fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(future)
    })
}
