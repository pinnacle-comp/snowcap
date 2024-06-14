use snowcap_api_defs::snowcap::widget;

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct Font {
    pub family: Family,
    pub weight: Weight,
    pub stretch: Stretch,
    pub style: Style,
}

impl Font {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_family(family: Family) -> Self {
        Self {
            family,
            ..Default::default()
        }
    }

    pub fn family(self, family: Family) -> Self {
        Self { family, ..self }
    }

    pub fn weight(self, weight: Weight) -> Self {
        Self { weight, ..self }
    }

    pub fn stretch(self, stretch: Stretch) -> Self {
        Self { stretch, ..self }
    }

    pub fn style(self, style: Style) -> Self {
        Self { style, ..self }
    }
}

impl From<Font> for widget::v0alpha1::Font {
    fn from(value: Font) -> Self {
        Self {
            family: Some(value.family.into()),
            weight: Some(widget::v0alpha1::font::Weight::from(value.weight) as i32),
            stretch: Some(widget::v0alpha1::font::Stretch::from(value.stretch) as i32),
            style: Some(widget::v0alpha1::font::Style::from(value.style) as i32),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub enum Family {
    Name(String),
    Serif,
    #[default]
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

impl From<Family> for widget::v0alpha1::font::Family {
    fn from(value: Family) -> Self {
        Self {
            family: Some(match value {
                Family::Name(name) => widget::v0alpha1::font::family::Family::Name(name),
                Family::Serif => widget::v0alpha1::font::family::Family::Serif(()),
                Family::SansSerif => widget::v0alpha1::font::family::Family::SansSerif(()),
                Family::Cursive => widget::v0alpha1::font::family::Family::Cursive(()),
                Family::Fantasy => widget::v0alpha1::font::family::Family::Fantasy(()),
                Family::Monospace => widget::v0alpha1::font::family::Family::Monospace(()),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Weight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

impl From<Weight> for widget::v0alpha1::font::Weight {
    fn from(value: Weight) -> Self {
        match value {
            Weight::Thin => widget::v0alpha1::font::Weight::Thin,
            Weight::ExtraLight => widget::v0alpha1::font::Weight::ExtraLight,
            Weight::Light => widget::v0alpha1::font::Weight::Light,
            Weight::Normal => widget::v0alpha1::font::Weight::Normal,
            Weight::Medium => widget::v0alpha1::font::Weight::Medium,
            Weight::Semibold => widget::v0alpha1::font::Weight::Semibold,
            Weight::Bold => widget::v0alpha1::font::Weight::Bold,
            Weight::ExtraBold => widget::v0alpha1::font::Weight::ExtraBold,
            Weight::Black => widget::v0alpha1::font::Weight::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Stretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl From<Stretch> for widget::v0alpha1::font::Stretch {
    fn from(value: Stretch) -> Self {
        match value {
            Stretch::UltraCondensed => widget::v0alpha1::font::Stretch::UltraCondensed,
            Stretch::ExtraCondensed => widget::v0alpha1::font::Stretch::ExtraCondensed,
            Stretch::Condensed => widget::v0alpha1::font::Stretch::Condensed,
            Stretch::SemiCondensed => widget::v0alpha1::font::Stretch::SemiCondensed,
            Stretch::Normal => widget::v0alpha1::font::Stretch::Normal,
            Stretch::SemiExpanded => widget::v0alpha1::font::Stretch::SemiExpanded,
            Stretch::Expanded => widget::v0alpha1::font::Stretch::Expanded,
            Stretch::ExtraExpanded => widget::v0alpha1::font::Stretch::ExtraExpanded,
            Stretch::UltraExpanded => widget::v0alpha1::font::Stretch::UltraExpanded,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Style {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl From<Style> for widget::v0alpha1::font::Style {
    fn from(value: Style) -> Self {
        match value {
            Style::Normal => widget::v0alpha1::font::Style::Normal,
            Style::Italic => widget::v0alpha1::font::Style::Italic,
            Style::Oblique => widget::v0alpha1::font::Style::Oblique,
        }
    }
}
