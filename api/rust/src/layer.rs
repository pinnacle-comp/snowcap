use std::num::NonZeroU32;

use snowcap_api_defs::snowcap::layer::{
    self,
    v0alpha1::{layer_service_client::LayerServiceClient, NewLayerRequest},
};
use tonic::transport::Channel;

use crate::{block_on_tokio, util::IntoApi, widget::WidgetDef};

pub struct Layer {
    client: LayerServiceClient<Channel>,
}

// TODO: change to bitflag
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

pub enum KeyboardInteractivity {
    None,
    OnDemand,
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

pub enum ExclusiveZone {
    Exclusive(NonZeroU32),
    Respect,
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

impl Layer {
    pub(crate) fn new(channel: Channel) -> Self {
        Self {
            client: LayerServiceClient::new(channel),
        }
    }

    pub fn new_widget(
        &self,
        widget: WidgetDef,
        width: u32,
        height: u32,
        anchor: Option<Anchor>,
        keyboard_interactivity: KeyboardInteractivity,
        exclusive_zone: ExclusiveZone,
    ) {
        let mut client = self.client.clone();

        block_on_tokio(client.new_layer(NewLayerRequest {
            widget_def: Some(widget.into_api()),
            width: Some(width),
            height: Some(height),
            anchor: anchor.map(|anchor| layer::v0alpha1::Anchor::from(anchor) as i32),
            keyboard_interactivity: Some(layer::v0alpha1::KeyboardInteractivity::from(
                keyboard_interactivity,
            ) as i32),
            exclusive_zone: Some(exclusive_zone.into()),
        }))
        .unwrap();
    }
}
