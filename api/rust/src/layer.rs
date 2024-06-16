//! Support for layer surface widgets using `wlr-layer-shell`.

use std::num::NonZeroU32;

use snowcap_api_defs::snowcap::{
    input::v0alpha1::{input_service_client::InputServiceClient, KeyboardKeyRequest},
    layer::{
        self,
        v0alpha1::{layer_service_client::LayerServiceClient, CloseRequest, NewLayerRequest},
    },
};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use xkbcommon::xkb::Keysym;

use crate::{
    block_on_tokio,
    input::Modifiers,
    widget::{WidgetDef, WidgetId},
};

/// The Layer API.
#[derive(Clone, Debug)]
pub struct Layer {
    client: LayerServiceClient<Channel>,
    input_client: InputServiceClient<Channel>,
    join_handle_sender: UnboundedSender<JoinHandle<()>>,
}

// TODO: change to bitflag
/// An anchor for a layer surface.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Anchor {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl From<Anchor> for layer::v0alpha1::Anchor {
    fn from(value: Anchor) -> Self {
        match value {
            Anchor::Top => layer::v0alpha1::Anchor::Top,
            Anchor::Bottom => layer::v0alpha1::Anchor::Bottom,
            Anchor::Left => layer::v0alpha1::Anchor::Left,
            Anchor::Right => layer::v0alpha1::Anchor::Right,
            Anchor::TopLeft => layer::v0alpha1::Anchor::TopLeft,
            Anchor::TopRight => layer::v0alpha1::Anchor::TopRight,
            Anchor::BottomLeft => layer::v0alpha1::Anchor::BottomLeft,
            Anchor::BottomRight => layer::v0alpha1::Anchor::BottomRight,
        }
    }
}

/// Layer surface keyboard interactivity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum KeyboardInteractivity {
    /// This layer surface cannot get keyboard focus.
    None,
    /// This layer surface can get keyboard focus through the compositor's implementation.
    OnDemand,
    /// This layer surface will take exclusive keyboard focus.
    Exclusive,
}

impl From<KeyboardInteractivity> for layer::v0alpha1::KeyboardInteractivity {
    fn from(value: KeyboardInteractivity) -> Self {
        match value {
            KeyboardInteractivity::None => layer::v0alpha1::KeyboardInteractivity::None,
            KeyboardInteractivity::OnDemand => layer::v0alpha1::KeyboardInteractivity::OnDemand,
            KeyboardInteractivity::Exclusive => layer::v0alpha1::KeyboardInteractivity::Exclusive,
        }
    }
}

/// Layer surface behavior for exclusive zones.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ExclusiveZone {
    /// This layer surface requests an exclusive zone of the given size.
    Exclusive(NonZeroU32),
    /// The layer surface does not request an exclusive zone but wants to be
    /// positioned respecting any active exclusive zones.
    Respect,
    /// The layer surface does not request an exclusive zone and wants to be
    /// positioned ignoring any active exclusive zones.
    Ignore,
}

impl From<ExclusiveZone> for i32 {
    fn from(value: ExclusiveZone) -> Self {
        match value {
            ExclusiveZone::Exclusive(size) => size.get() as i32,
            ExclusiveZone::Respect => 0,
            ExclusiveZone::Ignore => -1,
        }
    }
}

/// The layer on which a layer surface will be drawn.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ZLayer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl From<ZLayer> for layer::v0alpha1::Layer {
    fn from(value: ZLayer) -> Self {
        match value {
            ZLayer::Background => Self::Background,
            ZLayer::Bottom => Self::Bottom,
            ZLayer::Top => Self::Top,
            ZLayer::Overlay => Self::Overlay,
        }
    }
}

impl Layer {
    pub(crate) fn new(channel: Channel, sender: UnboundedSender<JoinHandle<()>>) -> Self {
        Self {
            client: LayerServiceClient::new(channel.clone()),
            input_client: InputServiceClient::new(channel),
            join_handle_sender: sender,
        }
    }

    /// Create a new widget.
    pub fn new_widget(
        &self,
        widget: impl Into<WidgetDef>,
        width: u32,
        height: u32,
        anchor: Option<Anchor>,
        keyboard_interactivity: KeyboardInteractivity,
        exclusive_zone: ExclusiveZone,
        layer: ZLayer,
    ) -> LayerHandle {
        let mut client = self.client.clone();

        let response = block_on_tokio(client.new_layer(NewLayerRequest {
            widget_def: Some(widget.into().into()),
            width: Some(width),
            height: Some(height),
            anchor: anchor.map(|anchor| layer::v0alpha1::Anchor::from(anchor) as i32),
            keyboard_interactivity: Some(layer::v0alpha1::KeyboardInteractivity::from(
                keyboard_interactivity,
            ) as i32),
            exclusive_zone: Some(exclusive_zone.into()),
            layer: Some(layer::v0alpha1::Layer::from(layer) as i32),
        }))
        .unwrap();

        let id = response
            .into_inner()
            .layer_id
            .expect("id should not be null");

        LayerHandle {
            id: id.into(),
            client,
            input_client: self.input_client.clone(),
            join_handle_sender: self.join_handle_sender.clone(),
        }
    }
}

/// A handle to a layer surface widget.
#[derive(Debug, Clone)]
pub struct LayerHandle {
    id: WidgetId,
    client: LayerServiceClient<Channel>,
    input_client: InputServiceClient<Channel>,
    join_handle_sender: UnboundedSender<JoinHandle<()>>,
}

impl LayerHandle {
    /// Close this layer widget.
    pub fn close(&self) {
        let mut client = self.client.clone();
        block_on_tokio(client.close(CloseRequest {
            layer_id: Some(self.id.into_inner()),
        }))
        .unwrap();
    }

    /// Do something on key press.
    pub fn on_key_press(
        &self,
        mut on_press: impl FnMut(&LayerHandle, Keysym, Modifiers) + Send + 'static,
    ) {
        let mut client = self.input_client.clone();

        let mut stream = block_on_tokio(client.keyboard_key(KeyboardKeyRequest {
            id: Some(self.id.into_inner()),
        }))
        .unwrap()
        .into_inner();

        let handle = self.clone();

        self.join_handle_sender
            .send(tokio::spawn(async move {
                while let Some(Ok(response)) = stream.next().await {
                    let key = Keysym::new(response.key());
                    let mods = Modifiers::from(response.modifiers.unwrap_or_default());

                    on_press(&handle, key, mods);
                }
            }))
            .unwrap();
    }
}
