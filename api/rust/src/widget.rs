use snowcap_api_defs::snowcap::widget;

pub enum WidgetDef {
    Text(Text),
}

impl WidgetDef {
    pub(crate) fn into_api(self) -> widget::v0alpha1::WidgetDef {
        widget::v0alpha1::WidgetDef {
            widget: Some(match self {
                WidgetDef::Text(text) => {
                    widget::v0alpha1::widget_def::Widget::Text(text.into_api())
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

pub struct Text {
    pub text: String,
    pub size: Option<f32>,
}

impl Text {
    pub fn new(text: impl ToString) -> Self {
        Self {
            text: text.to_string(),
            size: None,
        }
    }

    pub fn with_size(self, size: f32) -> Self {
        Self {
            size: Some(size),
            ..self
        }
    }

    pub(crate) fn into_api(self) -> widget::v0alpha1::Text {
        widget::v0alpha1::Text {
            text: Some(self.text),
            pixels: self.size,
        }
    }
}
