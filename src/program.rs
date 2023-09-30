use iced::{
    widget::{column, slider},
    Length, Theme,
};
use iced_runtime::{Command, Program};
use iced_wgpu::core::Element;

pub struct Prog;

impl Program for Prog {
    type Renderer = iced_wgpu::Renderer<Theme>;

    type Message = ();

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        let col = column![
            slider(0.0..=1.0, 0.5, |_| {}).step(0.1),
            slider(0.0..=5.0, 1.5, |_| {}).step(0.25)
        ]
        .width(Length::Fill);

        col.into()
    }
}
