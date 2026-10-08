use std::cell::RefCell;
use std::rc::Rc;

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::ViewExt;
use crate::platform::PointerButton;
use crate::theme::{Color, ControlAppearance, ControlVisualState, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::omochi_shape::{OmochiPreset, OmochiShape};
use super::{
    Button, ButtonInteractionState, ButtonSize, ButtonStyle, Icon, SymbolName, ZStackAlignment,
};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonTone {
    #[default]
    Plain,
    Accent,
}

pub struct IconButton {
    icon: SymbolName,
    tone: IconButtonTone,
    size: Option<ButtonSize>,
    enabled: bool,
    interaction: ButtonInteractionState,
    surface: OmochiShape,
    on_click: Option<Callback>,
    accessibility_label: Option<String>,
}

impl IconButton {
    pub fn new(icon: SymbolName) -> Self {
        Self {
            icon,
            tone: IconButtonTone::Plain,
            size: None,
            enabled: true,
            interaction: ButtonInteractionState::new(),
            surface: OmochiShape::velocity(OmochiPreset::CompactControl),
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

    fn resolved_size(&self) -> ButtonSize {
        self.size.unwrap_or(match self.tone {
            IconButtonTone::Plain => ButtonSize::Small,
            IconButtonTone::Accent => ButtonSize::Medium,
        })
    }

    fn visual_state(&self) -> ControlVisualState {
        if !self.enabled {
            ControlVisualState::Disabled
        } else if self.interaction.is_pressed() {
            ControlVisualState::Pressed
        } else if self.interaction.is_hovered() {
            ControlVisualState::Hovered
        } else if self.interaction.is_focused() {
            ControlVisualState::Focused
        } else {
            ControlVisualState::Rest
        }
    }

    fn appearance(&self, theme: &Theme) -> ControlAppearance {
        let state = self.visual_state();
        let palette = match self.tone {
            IconButtonTone::Plain => theme.button.ghost,
            IconButtonTone::Accent => theme.button.accent,
        };
        let mut appearance = palette.resolve(state);

        if state == ControlVisualState::Disabled {
            appearance.background =
                color_with_opacity(appearance.background, theme.button.disabled_opacity);
            appearance.border =
                color_with_opacity(appearance.border, theme.button.disabled_opacity);
            appearance.foreground = match self.tone {
                IconButtonTone::Plain => theme.colors.text_disabled,
                IconButtonTone::Accent => {
                    color_with_opacity(appearance.foreground, theme.button.disabled_opacity)
                }
            };
        }

        appearance
    }

    fn button(&self, theme: &Theme, icon_color: Color) -> Button {
        let size = self.resolved_size();
        let control_size = size.height(theme);
        let icon_size = self.size.map_or_else(
            || self.icon.control_size(theme.layout),
            |size| size.icon_size(theme),
        );

        let mut button = Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::TRANSPARENT,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: icon_color,
            })
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

impl View for IconButton {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let appearance = self.appearance(context.theme);

        self.button(context.theme, appearance.foreground)
            .measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        if !self.enabled && self.surface.is_animating() {
            self.surface.reset();
        }

        let appearance = self.appearance(context.theme);
        let radius = context.theme.button.radius.resolve(
            &context.theme.radius,
            bounds.size.width,
            bounds.size.height,
        );
        self.surface
            .paint(bounds, radius, appearance.background, context);

        self.button(context.theme, appearance.foreground)
            .paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !self.enabled {
            if self.surface.is_animating() {
                self.surface.reset();
            }

            return self
                .button(context.theme, self.appearance(context.theme).foreground)
                .handle_event(bounds, event, context);
        }

        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(*position) => {
                self.surface.begin(*position);
            }
            ViewEvent::PointerMoved { position } if self.surface.is_active() => {
                self.surface.moved(*position);
            }
            ViewEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } if self.surface.is_active() => {
                self.surface.end();
            }
            ViewEvent::PointerLeft if self.surface.is_active() => {
                self.surface.end();
            }
            _ => {}
        }

        let appearance = self.appearance(context.theme);
        let result = self
            .button(context.theme, appearance.foreground)
            .handle_event(bounds, event, context);

        if self.surface.is_animating() {
            context.request_redraw_in(bounds.expanded(24.0));
        }

        result
    }
}

fn color_with_opacity(color: Color, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };

    color.with_alpha((color.alpha as f32 * opacity).round() as u8)
}
