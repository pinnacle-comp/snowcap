use std::path::Path;

use anyhow::Context;
use smithay_client_toolkit::{reexports::calloop, shell::wlr_layer};
use snowcap_api_defs::snowcap::layer::{
    self,
    v0alpha1::{
        layer_service_server::{self, LayerServiceServer},
        NewLayerRequest, NewLayerResponse,
    },
};
use tonic::{Request, Response, Status};
use tracing::{error, warn};

use crate::{
    layer::SnowcapLayer,
    state::State,
    widget::{widget_def_to_fn, WidgetFn},
};

async fn run_unary_no_response<F>(
    fn_sender: &StateFnSender,
    with_state: F,
) -> Result<Response<()>, Status>
where
    F: FnOnce(&mut State) + Send + 'static,
{
    fn_sender
        .send(Box::new(with_state))
        .map_err(|_| Status::internal("failed to execute request"))?;

    Ok(Response::new(()))
}

async fn run_unary<F, T>(fn_sender: &StateFnSender, with_state: F) -> Result<Response<T>, Status>
where
    F: FnOnce(&mut State) -> T + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel::<T>();

    let f = Box::new(|state: &mut State| {
        // TODO: find a way to handle this error
        if sender.send(with_state(state)).is_err() {
            warn!("failed to send result of API call to config; receiver already dropped");
        }
    });

    fn_sender
        .send(f)
        .map_err(|_| Status::internal("failed to execute request"))?;

    receiver.await.map(Response::new).map_err(|err| {
        Status::internal(format!(
            "failed to transfer response for transport to client: {err}"
        ))
    })
}

impl State {
    pub fn start_grpc_server(&mut self, socket_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let socket_dir = socket_dir.as_ref();
        std::fs::create_dir_all(socket_dir)?;

        let socket_name = format!("snowcap-grpc-{}.sock", std::process::id());

        let socket_path = socket_dir.join(socket_name);

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

        let refl_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(snowcap_api_defs::FILE_DESCRIPTOR_SET)
            .build()?;

        let uds = tokio::net::UnixListener::bind(&socket_path)?;
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

        std::env::set_var("SNOWCAP_GRPC_SOCKET", &socket_path);

        let grpc_server = tonic::transport::Server::builder()
            .add_service(refl_service)
            .add_service(LayerServiceServer::new(layer_service));

        let todo = tokio::spawn(async move {
            if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                error!("gRPC server error: {err}");
            }
        });

        Ok(())
    }
}

type StateFnSender = calloop::channel::Sender<Box<dyn FnOnce(&mut State) + Send>>;

struct SnowcapService {
    sender: StateFnSender,
}

impl SnowcapService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

struct LayerService {
    sender: StateFnSender,
}

impl LayerService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

#[tonic::async_trait]
impl layer_service_server::LayerService for LayerService {
    async fn new_layer(
        &self,
        request: Request<NewLayerRequest>,
    ) -> Result<Response<NewLayerResponse>, Status> {
        let request = request.into_inner();

        let anchor = request.anchor();

        let Some(widget_def) = request.widget_def else {
            return Err(Status::invalid_argument("no widget def"));
        };

        let width = request.width.unwrap_or(600);
        let height = request.height.unwrap_or(480);

        let anchor = match anchor {
            layer::v0alpha1::Anchor::Unspecified => wlr_layer::Anchor::empty(),
            layer::v0alpha1::Anchor::Top => wlr_layer::Anchor::TOP,
            layer::v0alpha1::Anchor::Bottom => wlr_layer::Anchor::BOTTOM,
            layer::v0alpha1::Anchor::Left => wlr_layer::Anchor::LEFT,
            layer::v0alpha1::Anchor::Right => wlr_layer::Anchor::RIGHT,
            layer::v0alpha1::Anchor::TopLeft => wlr_layer::Anchor::TOP | wlr_layer::Anchor::LEFT,
            layer::v0alpha1::Anchor::TopRight => wlr_layer::Anchor::TOP | wlr_layer::Anchor::RIGHT,
            layer::v0alpha1::Anchor::BottomLeft => {
                wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::LEFT
            }
            layer::v0alpha1::Anchor::BottomRight => {
                wlr_layer::Anchor::BOTTOM | wlr_layer::Anchor::RIGHT
            }
        };

        run_unary(&self.sender, move |state| {
            let Some((f, states)) = widget_def_to_fn(widget_def) else {
                return NewLayerResponse {}; // TODO: error
            };

            let layer = SnowcapLayer::new(
                state,
                width,
                height,
                anchor,
                crate::widget::SnowcapWidgetProgram {
                    widgets: f,
                    widget_state: states,
                },
            );

            state.layers.push(layer);

            NewLayerResponse {}
        })
        .await
    }
}
