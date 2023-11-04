#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct CallbackId(pub u32);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum Anchor {
    Top,
    Bottom,
    Left,
    Right,
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

#[derive(Debug, serde::Deserialize)]
pub enum Msg {
    NewWidget {
        widget: WidgetDefinition,
        width: u32,
        height: u32,
        anchor: Anchor,
    },
}

#[derive(Debug, serde::Serialize)]
pub enum OutgoingMsg {
    CallCallback {
        callback_id: CallbackId,
        #[serde(default)]
        args: Option<Args>,
    },
}

#[derive(Debug, serde::Deserialize)]
pub enum WidgetDefinition {
    Slider {
        range_start: f32,
        range_end: f32,
        // value: ,
        on_change: CallbackId,
        #[serde(default)]
        on_release: Option<CallbackId>,
        #[serde(with = "LengthDef")]
        width: iced::Length,
        height: u16,
        step: f32,
    },
    Column {
        spacing: u16,
        #[serde(with = "PaddingDef")]
        padding: iced::Padding,
        #[serde(with = "LengthDef")]
        width: iced::Length,
        #[serde(with = "LengthDef")]
        height: iced::Length,
        max_width: u16,
        #[serde(with = "AlignmentDef")]
        alignment: iced::Alignment,
        children: Vec<WidgetDefinition>,
    },
    Button {
        #[serde(with = "LengthDef")]
        width: iced::Length,
        #[serde(with = "LengthDef")]
        height: iced::Length,
        #[serde(with = "PaddingDef")]
        padding: iced::Padding,
        child: Box<WidgetDefinition>,
    },
    Text {
        text: String,
        // size: u16,
        // line_height: TODO:
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(remote = "iced::Alignment")]
enum AlignmentDef {
    Start,
    Center,
    End,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(remote = "iced::Padding")]
struct PaddingDef {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(remote = "iced::Length")]
enum LengthDef {
    Fill,
    FillPortion(u16),
    Shrink,
    Fixed(f32),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Args {
    SliderValue(f32),
}
