use iced::{
    widget::{column, slider},
    Length, Theme,
};
use iced_runtime::{Command, Program};
use iced_wgpu::core::Element;

#[derive(Default)]
pub struct Prog {
    slider_val: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    SliderChanged(f32),
}

impl Program for Prog {
    type Renderer = iced_wgpu::Renderer<Theme>;

    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::SliderChanged(num) => self.slider_val = num,
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        let col = column![
            slider(0.0..=1.0, self.slider_val, Message::SliderChanged).step(0.1),
            slider(0.0..=5.0, self.slider_val, Message::SliderChanged).step(0.25)
        ]
        .width(Length::Fill);

        col.into()
    }
}
