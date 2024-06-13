use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity, ZLayer},
    widget::{Alignment, Color, Column, Container, Length, Padding, Row, Text},
};

#[tokio::main]
async fn main() {
    let (layer, recv) = snowcap_api::connect().await.unwrap();

    let test_key_descs = [
        ("Super + Enter", "Open alacritty"),
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
                .map(|(_, desc)| {
                    Text::new(desc)
                        .with_horizontal_alignment(Alignment::End)
                        .with_width(Length::Fill)
                        .into()
                })
                .chain([Row::new_with_children([
                    Text::new("first")
                        .with_horizontal_alignment(Alignment::End)
                        .into(),
                    Container::new(
                        Text::new("alacritty").with_horizontal_alignment(Alignment::End),
                    )
                    .with_background_color(Color {
                        red: 0.5,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 1.0,
                    })
                    .with_width(Length::Shrink)
                    .with_horizontal_alignment(Alignment::End)
                    .into(),
                ])
                .into()]),
        )
        .with_width(Length::FillPortion(1))
        .with_item_alignment(Alignment::End)
        .into(),
    ]))
    .with_width(Length::Fill)
    .with_height(Length::Fill)
    .with_padding(Padding {
        top: 12.0,
        right: 12.0,
        bottom: 12.0,
        left: 12.0,
    })
    .with_border_radius(64.0)
    .with_border_thickness(6.0)
    .with_background_color(Color {
        red: 0.5,
        green: 0.0,
        blue: 0.0,
        alpha: 0.3,
    });

    layer
        .new_widget(
            widget.into(),
            400,
            500,
            None,
            KeyboardInteractivity::Exclusive,
            ExclusiveZone::Respect,
            ZLayer::Top,
        )
        .on_key_press(|handle, _key, _mods| {
            handle.close();
        });

    snowcap_api::listen(recv).await;

    // let widget = layer.new_widget(...);
    //
    // widget.close();

    // layer.new_widget(...)
    //     .on_key_press(|widget, key, mods| {
    //         if key == Key::Escape {
    //             widget.close();
    //         }
    //     })
    //
    // OR
    //
    // let widget = layer.new_widget(...);
    //
    // widget.on_key_press(|key, mods| {
    //     if key == Key::Escape {
    //         widget.close();
    //     }
    // })
    //
}
