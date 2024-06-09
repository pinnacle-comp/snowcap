use iced_wgpu::core::SmolStr;
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
        Connection, QueueHandle,
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
    shell::{wlr_layer::LayerSurface, WaylandSurface},
};

use crate::state::State;

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        surface: &WlSurface,
        serial: u32,
        raw: &[u32],
        keysyms: &[Keysym],
    ) {
        if let Some(layer) = self
            .layers
            .iter()
            .find(|sn_layer| sn_layer.layer.wl_surface() == surface)
        {
            self.keyboard_focus = Some(KeyboardFocus::Layer(layer.layer.clone()));
        }
    }

    fn leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        surface: &WlSurface,
        serial: u32,
    ) {
        if let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() {
            if layer.wl_surface() == surface {
                self.keyboard_focus = None;
            }
        }
    }

    fn press_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() else {
            return;
        };

        let Some(snowcap_layer) = self.layers.iter_mut().find(|sn_l| &sn_l.layer == layer) else {
            return;
        };

        let key = event
            .keysym
            .key_char()
            .map(|ch| iced::keyboard::Key::Character(SmolStr::new(ch.to_string())))
            .unwrap_or(iced::keyboard::Key::Unidentified);

        let mut modifiers = iced::keyboard::Modifiers::empty();
        if self.keyboard_modifiers.ctrl {
            modifiers |= iced::keyboard::Modifiers::CTRL;
        }
        if self.keyboard_modifiers.alt {
            modifiers |= iced::keyboard::Modifiers::ALT;
        }
        if self.keyboard_modifiers.shift {
            modifiers |= iced::keyboard::Modifiers::SHIFT;
        }
        if self.keyboard_modifiers.logo {
            modifiers |= iced::keyboard::Modifiers::LOGO;
        }

        snowcap_layer.widgets.queue_event(iced::Event::Keyboard(
            iced::keyboard::Event::KeyPressed {
                key,
                location: iced::keyboard::Location::Standard,
                modifiers,
                text: None,
            },
        ));
    }

    fn release_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        serial: u32,
        event: KeyEvent,
    ) {
        let Some(KeyboardFocus::Layer(layer)) = self.keyboard_focus.as_ref() else {
            return;
        };

        let Some(snowcap_layer) = self.layers.iter_mut().find(|sn_l| &sn_l.layer == layer) else {
            return;
        };

        let key = event
            .keysym
            .key_char()
            .map(|ch| iced::keyboard::Key::Character(SmolStr::new(ch.to_string())))
            .unwrap_or(iced::keyboard::Key::Unidentified);

        let mut modifiers = iced::keyboard::Modifiers::empty();
        if self.keyboard_modifiers.ctrl {
            modifiers |= iced::keyboard::Modifiers::CTRL;
        }
        if self.keyboard_modifiers.alt {
            modifiers |= iced::keyboard::Modifiers::ALT;
        }
        if self.keyboard_modifiers.shift {
            modifiers |= iced::keyboard::Modifiers::SHIFT;
        }
        if self.keyboard_modifiers.logo {
            modifiers |= iced::keyboard::Modifiers::LOGO;
        }

        snowcap_layer.widgets.queue_event(iced::Event::Keyboard(
            iced::keyboard::Event::KeyReleased {
                key,
                location: iced::keyboard::Location::Standard,
                modifiers,
            },
        ));
    }

    fn update_modifiers(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        serial: u32,
        modifiers: Modifiers,
        layout: u32,
    ) {
        // TODO: per wl_keyboard
        self.keyboard_modifiers = modifiers;
    }
}
delegate_keyboard!(State);

pub enum KeyboardFocus {
    Layer(LayerSurface),
}
