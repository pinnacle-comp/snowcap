use std::{any::Any, collections::HashMap};

use iced::{
    widget::{Column, Row, Scrollable},
    Command,
};
use iced_runtime::Program;
use iced_wgpu::core::Element;
use snowcap_api_defs::snowcap::widget::{
    self,
    v0alpha1::{widget_def, WidgetDef},
};

use crate::util::convert::FromApi;

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
        widget_def::Widget::Text(text_def) => {
            let horizontal_alignment = text_def.horizontal_alignment();
            let vertical_alignment = text_def.vertical_alignment();

            let widget::v0alpha1::Text {
                text,
                pixels,
                width,
                height,
                horizontal_alignment: _,
                vertical_alignment: _,
            } = text_def;

            let f: WidgetFn = Box::new(move |_states| {
                let mut text = iced::widget::Text::new(text.clone().unwrap_or_default());
                if let Some(pixels) = pixels {
                    text = text.size(pixels);
                }
                if let Some(width) = width.clone() {
                    text = text.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    text = text.height(iced::Length::from_api(height));
                }

                match horizontal_alignment {
                    widget::v0alpha1::Alignment::Unspecified => (),
                    widget::v0alpha1::Alignment::Start => {
                        text = text.horizontal_alignment(iced::alignment::Horizontal::Left)
                    }
                    widget::v0alpha1::Alignment::Center => {
                        text = text.horizontal_alignment(iced::alignment::Horizontal::Center)
                    }
                    widget::v0alpha1::Alignment::End => {
                        text = text.horizontal_alignment(iced::alignment::Horizontal::Right)
                    }
                }

                match vertical_alignment {
                    widget::v0alpha1::Alignment::Unspecified => (),
                    widget::v0alpha1::Alignment::Start => {
                        text = text.vertical_alignment(iced::alignment::Vertical::Top)
                    }
                    widget::v0alpha1::Alignment::Center => {
                        text = text.vertical_alignment(iced::alignment::Vertical::Center)
                    }
                    widget::v0alpha1::Alignment::End => {
                        text = text.vertical_alignment(iced::alignment::Vertical::Bottom)
                    }
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

                if let Some(width) = width.clone() {
                    column = column.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    column = column.height(iced::Length::from_api(height));
                }
                if let Some(max_width) = max_width {
                    column = column.max_width(max_width);
                }
                if let Some(clip) = clip {
                    column = column.clip(clip);
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

                for child in children_widget_fns.iter() {
                    column = column.push(child(states));
                }

                column.into()
            });

            Some(f)
        }
        widget_def::Widget::Row(widget::v0alpha1::Row {
            spacing,
            padding,
            item_alignment,
            width,
            height,
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
                let mut row = Row::new();

                if let Some(spacing) = spacing {
                    row = row.spacing(spacing);
                }

                if let Some(width) = width.clone() {
                    row = row.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    row = row.height(iced::Length::from_api(height));
                }
                if let Some(clip) = clip {
                    row = row.clip(clip);
                }

                if let Some(widget::v0alpha1::Padding {
                    top,
                    right,
                    bottom,
                    left,
                }) = padding
                {
                    row = row.padding([
                        top.unwrap_or_default(),
                        right.unwrap_or_default(),
                        bottom.unwrap_or_default(),
                        left.unwrap_or_default(),
                    ]);
                }

                if let Some(alignment) = item_alignment {
                    row = row.align_items(match alignment {
                        // FIXME: actual conversion logic
                        1 => iced::Alignment::Start,
                        2 => iced::Alignment::Center,
                        3 => iced::Alignment::End,
                        _ => iced::Alignment::Start,
                    });
                }

                for child in children_widget_fns.iter() {
                    row = row.push(child(states));
                }

                row.into()
            });

            Some(f)
        }
        widget_def::Widget::Scrollable(scrollable_def) => {
            let widget::v0alpha1::Scrollable {
                width,
                height,
                direction,
                child,
            } = *scrollable_def;

            let child_widget_fn = child.and_then(|def| {
                *current_id += 1;
                widget_def_to_fn_inner(*def, current_id, states)
            });

            let f: WidgetFn = Box::new(move |states| {
                let mut scrollable = Scrollable::new(
                    child_widget_fn
                        .as_ref()
                        .map(|child| child(states))
                        .unwrap_or_else(|| iced::widget::Text::new("NULL").into()),
                );

                if let Some(width) = width.clone() {
                    scrollable = scrollable.width(iced::Length::from_api(width));
                }
                if let Some(height) = height.clone() {
                    scrollable = scrollable.height(iced::Length::from_api(height));
                }
                if let Some(direction) = direction.clone() {
                    scrollable = scrollable
                        .direction(iced::widget::scrollable::Direction::from_api(direction));
                }

                scrollable.into()
            });

            Some(f)
        }
    }
}
