use std::{any::Any, collections::HashMap};

use iced::{widget::Column, Command};
use iced_runtime::Program;
use iced_wgpu::core::Element;
use snowcap_api_defs::snowcap::widget::{
    self,
    v0alpha1::{widget_def, WidgetDef},
};

pub struct SnowcapWidgetProgram {
    pub widgets: WidgetFn,
    pub widget_state: HashMap<u32, Box<dyn Any + Send>>,
}

pub type WidgetFn = Box<
    dyn for<'a> Fn(
        &'a HashMap<u32, Box<dyn Any + Send>>,
    ) -> Element<'a, UpdateMessage, iced::Theme, iced_wgpu::Renderer>,
>;

pub type UpdateMessage = (u32, Box<dyn Any + Send>);

impl Program for SnowcapWidgetProgram {
    type Renderer = iced_wgpu::Renderer;

    type Theme = iced::Theme;

    type Message = UpdateMessage;

    fn update(&mut self, (id, data): Self::Message) -> Command<Self::Message> {
        self.widget_state.insert(id, data);
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        (self.widgets)(&self.widget_state)
    }
}

pub fn widget_def_to_fn(def: WidgetDef) -> Option<(WidgetFn, HashMap<u32, Box<dyn Any + Send>>)> {
    let mut states = HashMap::new();
    let mut current_id = 0;

    let f = widget_def_to_fn_inner(def, &mut current_id, &mut states);

    f.map(|f| (f, states))
}

fn widget_def_to_fn_inner(
    def: WidgetDef,
    current_id: &mut u32,
    states: &mut HashMap<u32, Box<dyn Any + Send>>,
) -> Option<WidgetFn> {
    let def = def.widget?;
    match def {
        widget_def::Widget::Text(widget::v0alpha1::Text { text, pixels }) => {
            let f: WidgetFn = Box::new(move |_states| {
                let mut text = iced::widget::Text::new(text.clone().unwrap_or_default());
                if let Some(pixels) = pixels {
                    text = text.size(pixels);
                }

                text.into()
            });
            Some(f)
        }
        widget_def::Widget::Column(widget::v0alpha1::Column {
            spacing,
            padding,
            item_alignment,
            width,
            height,
            max_width,
            clip,
            children,
        }) => {
            let children_widget_fns = children
                .into_iter()
                .flat_map(|def| {
                    *current_id += 1;
                    widget_def_to_fn_inner(def, current_id, states)
                })
                .collect::<Vec<_>>();

            let f: WidgetFn = Box::new(move |states| {
                let mut column = Column::new();

                if let Some(spacing) = spacing {
                    column = column.spacing(spacing);
                }

                if let Some(widget::v0alpha1::Padding {
                    top,
                    right,
                    bottom,
                    left,
                }) = padding
                {
                    column = column.padding([
                        top.unwrap_or_default(),
                        right.unwrap_or_default(),
                        bottom.unwrap_or_default(),
                        left.unwrap_or_default(),
                    ]);
                }

                if let Some(alignment) = item_alignment {
                    column = column.align_items(match alignment {
                        // FIXME: actual conversion logic
                        1 => iced::Alignment::Start,
                        2 => iced::Alignment::Center,
                        3 => iced::Alignment::End,
                        _ => iced::Alignment::Start,
                    });
                }

                // TODO: the rest

                for child in children_widget_fns.iter() {
                    column = column.push(child(states));
                }

                column.into()
            });

            Some(f)
        }
    }
}
