use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::{
        calloop::LoopHandle,
        client::{
            protocol::{wl_keyboard::WlKeyboard, wl_pointer::WlPointer},
            Connection, QueueHandle,
        },
    },
    registry::RegistryState,
    seat::{keyboard::Modifiers, SeatState},
    shell::wlr_layer::LayerShell,
};

use crate::{handlers::keyboard::KeyboardFocus, layer::SnowcapLayer, wgpu::Wgpu};

pub struct State {
    pub loop_handle: LoopHandle<'static, State>,
    pub conn: Connection,

    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub layer_shell_state: LayerShell,

    pub queue_handle: QueueHandle<State>,

    pub wgpu: Wgpu,

    pub layers: Vec<SnowcapLayer>,

    // TODO: per wl_keyboard
    pub keyboard_focus: Option<KeyboardFocus>,
    pub keyboard_modifiers: Modifiers,
    pub keyboard: Option<WlKeyboard>, // TODO: multiple

    pub pointer: Option<WlPointer>, // TODO: multiple
}
