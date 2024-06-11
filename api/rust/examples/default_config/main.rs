use snowcap_api::{
    layer::{ExclusiveZone, KeyboardInteractivity},
    widget::{Length, Row, Scrollable, Text, WidgetDef},
};

#[tokio::main]
async fn main() {
    let layer = snowcap_api::connect().await.unwrap();

    // let widget = WidgetDef::Text(Text::new("hello world! lorem ipsum weiohtwe wetoiph ewtoh pwt tewu weutih o uiogwte uiowte t twe uigowetig ouywtegio tw4iog wtiog pwt34ig owtgi ouwt igoyuwtg 8624g 789642g 7890624g78 642g78 2487g 42358g 754 8go7354w8og 523o 8g7352aw 8og7523 g8o7352g 87o3254g 87o3528 g7o5328 g7o532a8g 7o532g 87o532a 8g753 8g7538g 75a328 g75a3 8g7532a48og 7543gyo 4tgy ourtesgy uoterwgy uwetyg uwer gbrfe gvybrawefy uiwet yuwety guwte yguwet ugywetgy uoiwet gyuoiwetuy giowetugy wetguy iowetgy uwet"));
    // layer.new_widget(
    //     widget,
    //     400,
    //     200,
    //     None,
    //     KeyboardInteractivity::None,
    //     ExclusiveZone::Respect,
    // );

    let w = WidgetDef::Row(Row::with_children([
        Text::new("hi row 1")
            .with_width(Length::FillPortion(1))
            .into(),
        Text::new("hi row 2")
            .with_width(Length::FillPortion(1))
            .into(),
    ]));

    let scrollable = WidgetDef::Scrollable(Box::new(Scrollable::new(Text::new(
        "a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            a lot of text ioheth ioetih oetih oewtioh etwioh rwelhkwe \
            ",
    ))));

    layer.new_widget(
        scrollable,
        200,
        400,
        None,
        KeyboardInteractivity::None,
        ExclusiveZone::Respect,
    );
}
