use snowcap_api_defs::snowcap::layer::v0alpha1::{
    layer_service_client::LayerServiceClient, NewLayerRequest,
};
use tonic::transport::Channel;

use crate::{block_on_tokio, widget::WidgetDef};

pub struct Layer {
    client: LayerServiceClient<Channel>,
}

impl Layer {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            client: LayerServiceClient::new(channel),
        }
    }

    pub fn new_widget(&self, widget: WidgetDef, width: u32, height: u32) {
        let mut client = self.client.clone();

        block_on_tokio(client.new_layer(NewLayerRequest {
            widget_def: Some(widget.into_api()),
            width: Some(width),
            height: Some(height),
            anchor: None,
            keyboard_exclusivity: None,
        }))
        .unwrap();
    }
}
