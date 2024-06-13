use std::path::{Path, PathBuf};

use anyhow::Context;
use smithay_client_toolkit::reexports::calloop;
use snowcap_api_defs::snowcap::{
    input::v0alpha1::input_service_server::InputServiceServer,
    layer::v0alpha1::layer_service_server::LayerServiceServer,
};
use tracing::error;

use crate::{
    api::{input::InputService, LayerService, SnowcapService},
    state::State,
};

pub fn socket_dir() -> PathBuf {
    xdg::BaseDirectories::with_prefix("snowcap")
        .and_then(|xdg| xdg.get_runtime_directory().cloned())
        .unwrap_or(PathBuf::from("/tmp"))
}

fn socket_name() -> String {
    let wayland_suffix = std::env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
    format!("snowcap-grpc-{wayland_suffix}.sock")
}

impl State {
    pub fn start_grpc_server(&mut self, socket_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let socket_dir = socket_dir.as_ref();
        std::fs::create_dir_all(socket_dir)?;

        let socket_path = socket_dir.join(socket_name());

        if let Ok(true) = socket_path.try_exists() {
            std::fs::remove_file(&socket_path)
                .context(format!("failed to remove old socket at {socket_path:?}"))?;
        }

        let proto_dir = xdg::BaseDirectories::with_prefix("snowcap")?.get_data_file("protobuf");

        std::env::set_var("SNOWCAP_PROTO_DIR", proto_dir);

        let (grpc_sender, grpc_recv) =
            calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

        self.loop_handle
            .insert_source(grpc_recv, |msg, _, state| match msg {
                calloop::channel::Event::Msg(f) => f(state),
                calloop::channel::Event::Closed => error!("grpc receiver was closed"),
            })
            .unwrap();

        let snowcap_service = SnowcapService::new(grpc_sender.clone());
        let layer_service = LayerService::new(grpc_sender.clone());
        let input_service = InputService::new(grpc_sender.clone());

        let refl_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(snowcap_api_defs::FILE_DESCRIPTOR_SET)
            .build()?;

        let uds = tokio::net::UnixListener::bind(&socket_path)?;
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

        std::env::set_var("SNOWCAP_GRPC_SOCKET", &socket_path);

        let grpc_server = tonic::transport::Server::builder()
            .add_service(refl_service)
            .add_service(LayerServiceServer::new(layer_service))
            .add_service(InputServiceServer::new(input_service));

        let todo = tokio::spawn(async move {
            if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                error!("gRPC server error: {err}");
            }
        });

        Ok(())
    }
}
