use snowcap_api_defs::snowcap::widget;

use crate::util::IntoApi;

pub enum WidgetDef {
    Text(Text),
    Row(Row),
    Scrollable(Box<Scrollable>),
}

impl IntoApi for WidgetDef {
    type ApiType = widget::v0alpha1::WidgetDef;
    fn into_api(self) -> widget::v0alpha1::WidgetDef {
        widget::v0alpha1::WidgetDef {
            widget: Some(match self {
                WidgetDef::Text(text) => {
                    widget::v0alpha1::widget_def::Widget::Text(text.into_api())
                }
                WidgetDef::Row(row) => widget::v0alpha1::widget_def::Widget::Row(row.into_api()),
                WidgetDef::Scrollable(scrollable) => {
                    widget::v0alpha1::widget_def::Widget::Scrollable(Box::new(
                        scrollable.into_api(),
                    ))
                }
            }),
        }
    }
}

impl From<Text> for WidgetDef {
    fn from(value: Text) -> Self {
        Self::Text(value)
    }
}

#[derive(Default)]
pub struct Text {
    pub text: String,
    pub size: Option<f32>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub horizontal_alignment: Option<Alignment>,
    pub vertical_alignment: Option<Alignment>,
}

impl Text {
    pub fn new(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            ..Default::default()
        }
    }

    pub fn with_size(self, size: f32) -> Self {
        Self {
            size: Some(size),
            ..self
        }
    }

    pub fn with_width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn with_height(self, height: Length) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn with_horizontal_alignment(self, alignment: Alignment) -> Self {
        Self {
            horizontal_alignment: Some(alignment),
            ..self
        }
    }

    pub fn with_vertical_alignment(self, alignment: Alignment) -> Self {
        Self {
            vertical_alignment: Some(alignment),
            ..self
        }
    }
}

impl IntoApi for Text {
    type ApiType = widget::v0alpha1::Text;

    fn into_api(self) -> Self::ApiType {
        let mut text = widget::v0alpha1::Text {
            text: Some(self.text),
            pixels: self.size,
            width: self.width.map(IntoApi::into_api),
            height: self.height.map(IntoApi::into_api),
            horizontal_alignment: None,
            vertical_alignment: None,
        };
        if let Some(horizontal_alignment) = self.horizontal_alignment {
            text.set_horizontal_alignment(horizontal_alignment.into_api());
        }
        if let Some(vertical_alignment) = self.vertical_alignment {
            text.set_vertical_alignment(vertical_alignment.into_api());
        }
        text
    }
}

#[derive(Default)]
pub struct Row {
    pub spacing: Option<f32>,
    pub padding: Option<Padding>,
    pub item_alignment: Option<Alignment>,
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub clip: Option<bool>,
    pub children: Vec<WidgetDef>,
}

impl Row {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_children(children: impl IntoIterator<Item = WidgetDef>) -> Self {
        Self {
            children: children.into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn with_spacing(self, spacing: f32) -> Self {
        Self {
            spacing: Some(spacing),
            ..self
        }
    }

    pub fn with_item_alignment(self, item_alignment: Alignment) -> Self {
        Self {
            item_alignment: Some(item_alignment),
            ..self
        }
    }

    pub fn with_padding(self, padding: Padding) -> Self {
        Self {
            padding: Some(padding),
            ..self
        }
    }

    pub fn with_width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn with_height(self, height: Length) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn with_clip(self, clip: bool) -> Self {
        Self {
            clip: Some(clip),
            ..self
        }
    }

    pub fn push(self, child: WidgetDef) -> Self {
        let mut children = self.children;
        children.push(child);
        Self { children, ..self }
    }
}

impl IntoApi for Row {
    type ApiType = widget::v0alpha1::Row;

    fn into_api(self) -> Self::ApiType {
        widget::v0alpha1::Row {
            spacing: self.spacing,
            padding: self.padding.map(IntoApi::into_api),
            item_alignment: self.item_alignment.map(|it| it.into_api() as i32),
            width: self.width.map(IntoApi::into_api),
            height: self.height.map(IntoApi::into_api),
            clip: self.clip,
            children: self.children.into_iter().map(IntoApi::into_api).collect(),
        }
    }
}

pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl IntoApi for Padding {
    type ApiType = widget::v0alpha1::Padding;

    fn into_api(self) -> Self::ApiType {
        widget::v0alpha1::Padding {
            top: Some(self.top),
            right: Some(self.right),
            bottom: Some(self.bottom),
            left: Some(self.left),
        }
    }
}

pub enum Alignment {
    Start,
    Center,
    End,
}

impl IntoApi for Alignment {
    type ApiType = widget::v0alpha1::Alignment;

    fn into_api(self) -> Self::ApiType {
        match self {
            Alignment::Start => widget::v0alpha1::Alignment::Start,
            Alignment::Center => widget::v0alpha1::Alignment::Center,
            Alignment::End => widget::v0alpha1::Alignment::End,
        }
    }
}

pub enum Length {
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed(f32),
}

impl IntoApi for Length {
    type ApiType = widget::v0alpha1::Length;

    fn into_api(self) -> Self::ApiType {
        widget::v0alpha1::Length {
            strategy: Some(match self {
                Length::Fill => widget::v0alpha1::length::Strategy::Fill(()),
                Length::FillPortion(portion) => {
                    widget::v0alpha1::length::Strategy::FillPortion(portion as u32)
                }
                Length::Shrink => widget::v0alpha1::length::Strategy::Shrink(()),
                Length::Fixed(size) => widget::v0alpha1::length::Strategy::Fixed(size),
            }),
        }
    }
}

pub enum ScrollableDirection {
    Vertical(ScrollableProperties),
    Horizontal(ScrollableProperties),
    Both {
        vertical: ScrollableProperties,
        horizontal: ScrollableProperties,
    },
}

impl IntoApi for ScrollableDirection {
    type ApiType = widget::v0alpha1::ScrollableDirection;

    fn into_api(self) -> Self::ApiType {
        match self {
            ScrollableDirection::Vertical(props) => widget::v0alpha1::ScrollableDirection {
                vertical: Some(props.into_api()),
                horizontal: None,
            },
            ScrollableDirection::Horizontal(props) => widget::v0alpha1::ScrollableDirection {
                vertical: None,
                horizontal: Some(props.into_api()),
            },
            ScrollableDirection::Both {
                vertical,
                horizontal,
            } => widget::v0alpha1::ScrollableDirection {
                vertical: Some(vertical.into_api()),
                horizontal: Some(horizontal.into_api()),
            },
        }
    }
}

pub enum ScrollableAlignment {
    Start,
    End,
}

impl IntoApi for ScrollableAlignment {
    type ApiType = widget::v0alpha1::ScrollableAlignment;

    fn into_api(self) -> Self::ApiType {
        match self {
            ScrollableAlignment::Start => widget::v0alpha1::ScrollableAlignment::Start,
            ScrollableAlignment::End => widget::v0alpha1::ScrollableAlignment::End,
        }
    }
}

#[derive(Default)]
pub struct ScrollableProperties {
    pub width: Option<f32>,
    pub margin: Option<f32>,
    pub scroller_width: Option<f32>,
    pub alignment: Option<ScrollableAlignment>,
}

impl IntoApi for ScrollableProperties {
    type ApiType = widget::v0alpha1::ScrollableProperties;

    fn into_api(self) -> Self::ApiType {
        widget::v0alpha1::ScrollableProperties {
            width: self.width,
            margin: self.margin,
            scroller_width: self.scroller_width,
            alignment: self.alignment.map(|it| it.into_api() as i32),
        }
    }
}

pub struct Scrollable {
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub direction: Option<ScrollableDirection>,
    pub child: WidgetDef,
}

impl IntoApi for Scrollable {
    type ApiType = widget::v0alpha1::Scrollable;

    fn into_api(self) -> Self::ApiType {
        widget::v0alpha1::Scrollable {
            width: self.width.map(IntoApi::into_api),
            height: self.height.map(IntoApi::into_api),
            direction: self.direction.map(IntoApi::into_api),
            child: Some(Box::new(self.child.into_api())),
        }
    }
}

impl Scrollable {
    pub fn new(child: impl Into<WidgetDef>) -> Self {
        Self {
            child: child.into(),
            width: None,
            height: None,
            direction: None,
        }
    }

    pub fn with_width(self, width: Length) -> Self {
        Self {
            width: Some(width),
            ..self
        }
    }

    pub fn with_height(self, height: Length) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    pub fn with_direction(self, direction: ScrollableDirection) -> Self {
        Self {
            direction: Some(direction),
            ..self
        }
    }
}
