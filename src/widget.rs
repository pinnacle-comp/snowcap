use iced::Theme;
use iced_runtime::{Command, Program};
use iced_wgpu::{
    core::Element,
    graphics::Viewport,
    wgpu::{Adapter, Device, Queue},
    window::Surface,
};
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;

pub struct SnowcapWidgetProgram {
    pub widgets: WidgetFn,
}

pub type WidgetFn = Box<
    dyn Fn(&SnowcapWidgetProgram) -> Element<'static, SnowcapMessage, iced_wgpu::Renderer<Theme>>,
>;

pub struct SnowcapWidget {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface,
    pub renderer: iced_wgpu::Renderer<Theme>,

    pub width: u32,
    pub height: u32,
    pub viewport: Viewport,

    pub layer: LayerSurface,

    pub state: iced_runtime::program::State<SnowcapWidgetProgram>,
}

#[derive(Debug, Clone)]
pub enum SnowcapMessage {
    Nothing,
}

impl Program for SnowcapWidgetProgram {
    type Renderer = iced_wgpu::Renderer<Theme>;

    type Message = SnowcapMessage;

    fn update(&mut self, message: Self::Message) -> iced_runtime::Command<Self::Message> {
        match message {
            SnowcapMessage::Nothing => (),
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        (self.widgets)(self)
    }
}
