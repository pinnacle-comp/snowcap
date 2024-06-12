use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity},
    widget::{Column, Container, Length, Padding, Row, Scrollable, Text, WidgetDef},
};

#[tokio::main]
async fn main() {
    let layer = snowcap_api::connect().await.unwrap();

    let test_key_descs = [
        ("Super + Enter", "Open `alacritty`"),
        ("Super + M", "Toggle maximized"),
        ("Super + F", "Toggle fullscreen"),
        ("Super + Shift + Q", "Exit Pinnacle"),
    ];

    let widget = Container::new(Row::new_with_children([
        Column::new_with_children(
            test_key_descs
                .iter()
                .map(|(keys, _)| Text::new(keys).into()),
        )
        .with_width(Length::FillPortion(1))
        .into(),
        Column::new_with_children(
            test_key_descs
                .iter()
                .map(|(_, desc)| Text::new(desc).into()),
        )
        .with_width(Length::FillPortion(1))
        .into(),
    ]))
    .with_width(Length::Fill)
    .with_height(Length::Fill)
    .with_padding(Padding {
        top: 4.0,
        right: 4.0,
        bottom: 4.0,
        left: 4.0,
    });

    layer.new_widget(
        widget.into(),
        400,
        500,
        None,
        KeyboardInteractivity::None,
        ExclusiveZone::Respect,
    );
}
