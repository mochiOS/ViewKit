use std::cell::RefCell;
use std::rc::Rc;

use crate::layout::ViewExt;
use crate::theme::{ShadowStyle, Theme};

use super::{
    Button, ButtonInteractionState, ButtonSize, ButtonStyle, Icon, IconName, ZStackAlignment,
};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonTone {
    #[default]
    Plain,
    Accent,
}

pub struct IconButton {
    icon: IconName,
    tone: IconButtonTone,
    size: Option<ButtonSize>,
    enabled: bool,
    interaction: ButtonInteractionState,
    on_click: Option<Callback>,
    accessibility_label: Option<String>,
}

impl IconButton {
    pub fn new(icon: IconName) -> Self {
        Self {
            icon,
            tone: IconButtonTone::Plain,
            size: None,
            enabled: true,
            interaction: ButtonInteractionState::new(),
            on_click: None,
            accessibility_label: None,
        }
    }

    pub fn tone(mut self, tone: IconButtonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    pub fn on_click(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub(crate) fn button(&self, theme: &Theme) -> Button {
        let (style, icon_color, default_size) = match self.tone {
            IconButtonTone::Plain => {
                let icon_color = if self.enabled {
                    theme.button.ghost.rest.foreground
                } else {
                    theme.colors.text_disabled
                };

                (ButtonStyle::Ghost, icon_color, ButtonSize::Small)
            }
            IconButtonTone::Accent => {
                let icon_color = theme.button.accent.rest.foreground;
                (ButtonStyle::Accent, icon_color, ButtonSize::Medium)
            }
        };
        let size = self.size.unwrap_or(default_size);
        let control_size = size.height(theme);
        let icon_size = self.size.map_or_else(
            || self.icon.control_size(theme.layout),
            |size| size.icon_size(theme),
        );

        let mut button = Button::with_interaction(self.interaction.clone())
            .style(style)
            .size(size)
            .radius(theme.button.radius)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Center)
            .enabled(self.enabled)
            .accessibility_label_option(self.accessibility_label.clone())
            .content(
                Icon::new(self.icon)
                    .size(icon_size)
                    .color(icon_color)
                    .frame(control_size, control_size),
            );

        if let Some(on_click) = self.on_click.as_ref() {
            let on_click = Rc::clone(on_click);
            button = button.on_click(move || {
                (on_click.borrow_mut())();
            });
        }

        button
    }
}

impl crate::view::View for IconButton {
    fn measure(
        &self,
        constraints: crate::view::Constraints,
        context: &mut crate::view::MeasureContext<'_>,
    ) -> crate::geometry::Size {
        self.button(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: crate::geometry::Rect, context: &mut crate::view::PaintContext<'_>) {
        self.button(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: crate::geometry::Rect,
        event: &crate::event::ViewEvent,
        context: &mut crate::event::EventContext<'_>,
    ) -> crate::event::EventResult {
        self.button(context.theme)
            .handle_event(bounds, event, context)
    }
}
