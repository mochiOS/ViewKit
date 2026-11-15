use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::input_geometry::{field_text_line, input_text_role};
use crate::accessibility::AccessibilityRole;
use crate::event::{ContextMenuItem, ContextMenuRequest, EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::platform::PointerButton;
use crate::theme::{CornerRadius, ShadowStyle, Theme};
use crate::typography::Typography;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Button, ButtonInteractionState, ButtonStyle, Icon, SymbolName, Text, ZStackAlignment};

pub struct Picker {
    size: crate::theme::ControlSize,
    label: String,
    enabled: bool,
    toolbar: bool,
    radius: Option<CornerRadius>,
    interaction: ButtonInteractionState,
    options: Vec<PickerOption>,
    active_request: Cell<Option<u64>>,
}

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

struct PickerOption {
    label: String,
    enabled: bool,
    on_select: Callback,
}

impl Picker {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            size: crate::theme::ControlSize::Medium,
            label: label.into(),
            enabled: true,
            toolbar: false,
            radius: None,
            interaction: ButtonInteractionState::new(),
            options: Vec::new(),
            active_request: Cell::new(None),
        }
    }

    /// Uses the standard ghost treatment for toolbar/status-bar selectors.
    /// Menu dispatch, focus and the full label's hit target stay unchanged.
    pub fn toolbar(mut self) -> Self {
        self.toolbar = true;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// 入力欄と共通の高さ・文字サイズで表示します。既定値はMediumです。
    pub fn size(mut self, size: crate::theme::ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Adds a selectable value to the picker menu.
    pub fn option(mut self, label: impl Into<String>, on_select: impl FnMut() + 'static) -> Self {
        self.options.push(PickerOption {
            label: label.into(),
            enabled: true,
            on_select: Rc::new(RefCell::new(Box::new(on_select))),
        });
        self
    }

    /// Adds a disabled value while preserving menu ordering.
    pub fn disabled_option(mut self, label: impl Into<String>) -> Self {
        self.options.push(PickerOption {
            label: label.into(),
            enabled: false,
            on_select: Rc::new(RefCell::new(Box::new(|| {}))),
        });
        self
    }

    fn open_menu(&self, bounds: Rect, context: &mut EventContext<'_>) {
        if self.options.is_empty() {
            return;
        }
        let request_id = ContextMenuRequest::next_id();
        self.active_request.set(Some(request_id));
        context.show_context_menu(ContextMenuRequest {
            request_id,
            position: Point::new(bounds.origin.x, bounds.origin.y + bounds.size.height),
            items: self
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| ContextMenuItem {
                    command_id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                    label: option.label.clone(),
                    enabled: option.enabled,
                    checked: option.label == self.label,
                    destructive: false,
                    separator: false,
                })
                .collect(),
        });
    }

    fn button(&self, theme: &Theme) -> Button {
        let foreground = if self.enabled {
            theme.picker.foreground
        } else {
            theme.picker.disabled_foreground
        };

        Button::with_interaction(self.interaction.clone())
            .content_padding(false)
            .style(if self.toolbar {
                ButtonStyle::Ghost
            } else {
                ButtonStyle::Custom {
                    background: theme.picker.background,
                    hovered_background: theme.picker.hovered_background,
                    border: theme.picker.border,
                    hovered_border: theme.picker.hovered_border,
                    foreground,
                }
            })
            .radius(self.radius.unwrap_or(theme.picker.radius))
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .accessibility_role(AccessibilityRole::ComboBox)
            .accessibility_label(self.label.clone())
            .content(PickerContent {
                label: self.label.clone(),
                size: self.size,
                enabled: self.enabled,
                toolbar: self.toolbar,
            })
    }
}

struct PickerContent {
    label: String,
    size: crate::theme::ControlSize,
    enabled: bool,
    toolbar: bool,
}

impl PickerContent {
    fn text_bounds(
        &self,
        bounds: Rect,
        theme: &Theme,
        typography: &Typography,
        scale: f32,
    ) -> (Rect, Rect) {
        let padding = theme
            .picker
            .horizontal_padding
            .min(bounds.size.width.max(0.) * 0.5);
        let available = (bounds.size.width - padding * 2.).max(0.);
        let icon = theme
            .layout
            .compact_icon_size
            .min(available)
            .min(bounds.size.height.max(0.));
        let indicator = Rect::new(
            bounds.origin.x + bounds.size.width - padding - icon,
            bounds.origin.y + (bounds.size.height - icon) * 0.5,
            icon,
            icon,
        );
        let width = (available - icon - theme.spacing.small).max(0.);
        let text = field_text_line(
            Rect::new(
                bounds.origin.x + padding,
                bounds.origin.y,
                width,
                bounds.size.height,
            ),
            typography.style(input_text_role(self.size)),
            scale,
            theme.text_field.text_offset_y,
        );
        (text, indicator)
    }
}
impl View for PickerContent {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let text = Text::styled(&self.label, input_text_role(self.size)).measure(
            Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
            context,
        );
        constraints.constrain(Size::new(
            text.width
                + context.theme.picker.horizontal_padding * 2.
                + context.theme.layout.compact_icon_size
                + context.theme.spacing.small,
            self.size.height(&context.theme.controls),
        ))
    }
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let (text, indicator) = self.text_bounds(
            bounds,
            context.theme,
            context.typography,
            context.text_measurer.font_scale(),
        );
        let foreground = if !self.enabled {
            context.theme.picker.disabled_foreground
        } else if self.toolbar {
            context
                .control_foreground()
                .unwrap_or(context.theme.picker.foreground)
        } else {
            context.theme.picker.foreground
        };
        Text::styled(&self.label, input_text_role(self.size))
            .single_line(true)
            .accessibility_hidden(true)
            .color(foreground)
            .paint(text, context);
        Icon::new(SymbolName::ChevronDown)
            .size(indicator.size.width)
            .color(if self.enabled {
                context.theme.picker.indicator
            } else {
                foreground
            })
            .paint(indicator, context);
    }
}

impl View for Picker {
    fn first_baseline(&self, size: Size, context: &mut MeasureContext<'_>) -> Option<f32> {
        let content = PickerContent {
            label: self.label.clone(),
            size: self.size,
            enabled: self.enabled,
            toolbar: self.toolbar,
        };
        let (text, _) = content.text_bounds(
            Rect::new(0., 0., size.width, size.height),
            context.theme,
            context.typography,
            context.text_measurer.font_scale(),
        );
        Text::styled(&self.label, input_text_role(self.size))
            .first_baseline(text.size, context)
            .map(|baseline| baseline + text.origin.y)
    }
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(self.button(context.theme).measure(
            Constraints::new(
                Size::new(
                    context.theme.layout.control_min_width,
                    self.size.height(&context.theme.controls),
                ),
                Size::new(f32::INFINITY, self.size.height(&context.theme.controls)),
            ),
            context,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.button(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if let ViewEvent::ContextMenuResult {
            request_id,
            command_id,
        } = event
            && self.active_request.get() == Some(*request_id)
        {
            self.active_request.set(None);
            if let Some(index) = command_id.and_then(|id| id.checked_sub(1))
                && let Some(option) = self.options.get(index as usize)
                && option.enabled
            {
                (option.on_select.borrow_mut())();
                context.request_redraw();
            }
            return EventResult::Consumed;
        }

        let result = self
            .button(context.theme)
            .handle_event(bounds, event, context);
        if self.enabled
            && result.is_consumed()
            && matches!(
                event,
                ViewEvent::PointerReleased {
                    position,
                    button: PointerButton::Primary,
                } if bounds.contains(*position)
            )
        {
            self.open_menu(bounds, context);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn toolbar_picker_elides_long_labels_and_keeps_its_menu_target() {
        use crate::draw_command::{DisplayList, DrawCommand};
        let theme = Theme::LIGHT;
        let picker = Picker::new("a very long native application configuration name")
            .toolbar()
            .option("Debug", || {});
        let bounds = Rect::new(0., 0., 130., 30.);
        let mut text = crate::typography::TextMeasurer::new();
        let mut list = DisplayList::new();
        picker.paint(
            bounds,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
        );
        assert!(
            list.commands()
                .iter()
                .any(|c| matches!(c,DrawCommand::DrawText{command} if command.text.ends_with('…')))
        );
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        for event in [
            ViewEvent::PointerPressed {
                position: Point::new(16., 15.),
                button: crate::platform::PointerButton::Primary,
            },
            ViewEvent::PointerReleased {
                position: Point::new(16., 15.),
                button: crate::platform::PointerButton::Primary,
            },
        ] {
            picker.handle_event(bounds, &event, &mut cx);
        }
        assert_eq!(
            cx.take_context_menu_request().unwrap().items[0].label,
            "Debug"
        );
    }

    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn options_open_a_menu_and_dispatch_the_selected_callback() {
        let selected = Rc::new(Cell::new(false));
        let callback_state = Rc::clone(&selected);
        let picker = Picker::new("Japanese").option("English", move || {
            callback_state.set(true);
        });
        let bounds = Rect::new(20.0, 30.0, 264.0, 32.0);
        let position = Point::new(40.0, 45.0);
        let mut text_measurer = TextMeasurer::new();
        let mut context =
            EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut text_measurer);

        picker.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        picker.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            },
            &mut context,
        );

        let request = context.take_context_menu_request().expect("picker menu");
        assert_eq!(request.position, Point::new(20.0, 62.0));
        assert_eq!(request.items.len(), 1);
        assert_eq!(request.items[0].label, "English");
        assert!(!request.items[0].checked);

        picker.handle_event(
            bounds,
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id,
                command_id: Some(1),
            },
            &mut context,
        );
        assert!(selected.get());
    }
}

#[cfg(test)]
mod input_contract_tests {
    use super::*;
    use crate::{
        components::{TextField, TextFieldInteractionState, TextFieldSize},
        draw_command::{DisplayList, DrawCommand, TextCommand},
        typography::TextMeasurer,
    };

    fn paint(view: &dyn View, bounds: Rect, theme: &Theme, scale: f32) -> DisplayList {
        let mut list = DisplayList::new();
        let mut text = TextMeasurer::new();
        text.set_font_scale(scale);
        view.paint(
            bounds,
            &mut PaintContext::new(&mut list, theme, &theme.typography, &mut text),
        );
        list
    }
    fn label(list: &DisplayList) -> &TextCommand {
        list.commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::DrawText { command } => Some(command),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn picker_and_input_share_insets_text_lines_and_baselines_at_every_size() {
        use crate::theme::ControlSize;
        for mut theme in [Theme::LIGHT, Theme::DARK] {
            theme.text_field.text_offset_y = 1.;
            for (size, field_size) in [
                (ControlSize::Small, TextFieldSize::Small),
                (ControlSize::Medium, TextFieldSize::Medium),
                (ControlSize::Large, TextFieldSize::Large),
            ] {
                for scale in [1., 1.5, 2.] {
                    let picker = Picker::new("Value").size(size);
                    let field = TextField::with_interaction(TextFieldInteractionState::new())
                        .value("Value")
                        .size(field_size);
                    for width in [100., 260.] {
                        let bounds = Rect::new(10., 20., width, size.height(&theme.controls));
                        let a = paint(&picker, bounds, &theme, scale);
                        let b = paint(&field, bounds, &theme, scale);
                        assert_eq!(label(&a).bounds.origin, label(&b).bounds.origin);
                        assert_eq!(label(&a).font_size, label(&b).font_size);
                        assert_eq!(label(&a).line_height, label(&b).line_height);
                        let mut text = TextMeasurer::new();
                        text.set_font_scale(scale);
                        let mut context = MeasureContext {
                            theme: &theme,
                            typography: &theme.typography,
                            text_measurer: &mut text,
                        };
                        assert_eq!(
                            picker.first_baseline(bounds.size, &mut context),
                            field.first_baseline(bounds.size, &mut context)
                        );
                        assert_eq!(
                            picker
                                .measure(Constraints::loose(Size::new(400., 100.)), &mut context)
                                .height,
                            bounds.size.height
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn picker_elides_before_its_indicator_and_dims_the_disabled_indicator() {
        for theme in [Theme::LIGHT, Theme::DARK] {
            for width in [10., 80., 200.] {
                let picker =
                    Picker::new("A long value that does not fit in this picker").enabled(false);
                let bounds = Rect::new(10., 20., width, 30.);
                let list = paint(&picker, bounds, &theme, 1.);
                for command in list.commands() {
                    if let DrawCommand::DrawSvg { command } = command {
                        assert_eq!(command.tint, Some(theme.picker.disabled_foreground));
                        assert!(command.bounds.origin.x >= bounds.origin.x);
                        assert!(
                            command.bounds.origin.x + command.bounds.size.width
                                <= bounds.origin.x + width
                        );
                        // VK Symbols centers visible artwork, not the SVG canvas.
                        let svg = SymbolName::ChevronDown.svg().unwrap();
                        let ink = svg.content_bounds();
                        let center = command.bounds.origin.y
                            + (ink.y + ink.height / 2.) * command.bounds.size.height / svg.height();
                        assert!((center - 35.).abs() < 0.01);
                    }
                }
                if width >= 80. {
                    assert!(label(&list).text.ends_with('…'));
                    assert_eq!(label(&list).color, theme.picker.disabled_foreground);
                }
            }
        }
    }
}
