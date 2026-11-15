//! Selection and hover colors shared by list and navigation rows.
use super::ButtonStyle;
use crate::theme::{Color, Theme};
use crate::{
    draw_command::DrawCommand,
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    view::{Constraints, MeasureContext, PaintContext, View},
};
use std::rc::Rc;

/// Shared icon gutter. Preserve an artwork's intrinsic size within the slot.
pub(super) struct RowLeading(pub Rc<dyn View>);
impl RowLeading {
    fn content_bounds(&self, bounds: Rect, context: &mut MeasureContext<'_>) -> Rect {
        let constraints = Constraints::loose(bounds.size);
        let mut size = constraints.constrain(self.0.measure(constraints, context));
        // Legacy paint-only views have no intrinsic size and own the full slot.
        if size.width == 0. {
            size.width = bounds.size.width;
        }
        if size.height == 0. {
            size.height = bounds.size.height;
        }
        Rect::new(
            bounds.origin.x + (bounds.size.width - size.width) / 2.,
            bounds.origin.y + (bounds.size.height - size.height) / 2.,
            size.width,
            size.height,
        )
    }
}
impl View for RowLeading {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let side = context.theme.layout.list_leading_size;
        constraints.constrain(Size::new(side, side))
    }
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let content = self.content_bounds(
            bounds,
            &mut MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            },
        );
        context
            .display_list
            .push(DrawCommand::PushClip { rect: bounds });
        self.0.paint(content, context);
        context.display_list.push(DrawCommand::PopClip);
    }
    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let content = self.content_bounds(bounds, &mut context.measure_context());
        self.0.handle_event(content, event, context)
    }
}

pub(super) fn paint_keyboard_focus(
    bounds: Rect,
    radii: [f32; 4],
    interaction: &super::ButtonInteractionState,
    enabled: bool,
    context: &mut PaintContext<'_>,
) {
    if enabled && context.window_active() && interaction.is_focus_visible() {
        context
            .display_list
            .push(DrawCommand::StrokeInsideRoundedCornersRect {
                rect: bounds,
                radii,
                color: context.theme.button.focus_ring,
                width: context.theme.button.focus_ring_width,
            });
    }
}

#[derive(Clone, Copy)]
pub(super) struct RowAppearance {
    pub foreground: Color,
    pub secondary: Color,
    pub leading: Color,
    pub background: Color,
    pub hovered_background: Color,
}
impl RowAppearance {
    pub fn resolve(theme: &Theme, selected: bool, enabled: bool, active: bool) -> Self {
        let tokens = &theme.list;
        let foreground = if !enabled {
            tokens.disabled_foreground
        } else if selected && active {
            tokens.selected_foreground
        } else if selected {
            tokens.inactive_selected_foreground
        } else {
            tokens.foreground
        };
        let background = if selected {
            if active && enabled {
                tokens.selected_background
            } else {
                tokens.inactive_selected_background
            }
        } else {
            tokens.background
        };
        Self {
            foreground,
            secondary: if enabled {
                tokens.secondary_foreground
            } else {
                tokens.disabled_foreground
            },
            leading: if !enabled {
                tokens.disabled_foreground
            } else if selected {
                foreground
            } else {
                tokens.leading_foreground
            },
            background,
            hovered_background: if selected || !active || !enabled {
                background
            } else {
                tokens.hovered_background
            },
        }
    }
    pub fn button_style(self) -> ButtonStyle {
        ButtonStyle::Custom {
            background: self.background,
            hovered_background: self.hovered_background,
            border: Color::TRANSPARENT,
            hovered_border: Color::TRANSPARENT,
            foreground: self.foreground,
        }
    }
}
