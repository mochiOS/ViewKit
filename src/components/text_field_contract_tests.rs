use super::*;
use crate::draw_command::{DisplayList, TextCommand};
use crate::geometry::Point;
use crate::theme::Theme;
use crate::typography::TextMeasurer;

fn paint(field: &TextField, bounds: Rect, theme: &Theme, scale: f32) -> DisplayList {
    let mut list = DisplayList::new();
    let mut text = TextMeasurer::new();
    text.set_font_scale(scale);
    field.paint(
        bounds,
        &mut PaintContext::new(&mut list, theme, &theme.typography, &mut text),
    );
    let mut depth = 0;
    for command in list.commands() {
        match command {
            DrawCommand::PushClip { .. }
            | DrawCommand::PushRoundedClip { .. }
            | DrawCommand::PushRoundedCornersClip { .. } => depth += 1,
            DrawCommand::PopClip => {
                depth -= 1;
                assert!(depth >= 0);
            }
            _ => {}
        }
    }
    assert_eq!(
        depth, 0,
        "all clips must close, including before IME popups"
    );
    list
}
fn text(list: &DisplayList) -> &TextCommand {
    list.commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::DrawText { command } => Some(command),
            _ => None,
        })
        .unwrap()
}

#[test]
fn scaled_text_and_symbols_share_the_field_center_at_every_control_size() {
    for theme in [Theme::LIGHT, Theme::DARK] {
        for size in [
            TextFieldSize::Small,
            TextFieldSize::Medium,
            TextFieldSize::Large,
        ] {
            for scale in [1.0, 1.5, 2.0] {
                for width in [10.0, 200.0] {
                    let state = TextFieldInteractionState::new();
                    state.set_value("Sample");
                    let field = TextField::with_interaction(state)
                        .size(size)
                        .leading_symbol(SymbolName::Search);
                    let bounds = Rect::new(10.0, 20.0, width, size.height(&theme));
                    let list = paint(&field, bounds, &theme, scale);
                    let command = text(&list);
                    assert_eq!(
                        command.bounds.origin.y + command.line_height * 0.5,
                        bounds.origin.y + bounds.size.height * 0.5
                    );
                    let geometry = field.geometry(bounds, &theme, &theme.typography, scale);
                    assert!(geometry.viewport.origin.x >= bounds.origin.x);
                    assert!(
                        geometry.viewport.origin.x + geometry.viewport.size.width
                            <= bounds.origin.x + bounds.size.width
                    );
                    assert!(geometry.viewport.origin.y >= bounds.origin.y);
                    assert!(
                        geometry.viewport.origin.y + geometry.viewport.size.height
                            <= bounds.origin.y + bounds.size.height
                    );
                    let symbol = geometry.symbol.unwrap();
                    assert_eq!(
                        symbol.origin.y + symbol.size.height * 0.5,
                        bounds.origin.y + bounds.size.height * 0.5
                    );
                }
            }
        }
    }
}

#[test]
fn placeholder_and_value_use_one_origin_and_long_placeholder_is_clipped() {
    let theme = Theme::LIGHT;
    let state = TextFieldInteractionState::new();
    let field = TextField::with_interaction(state.clone())
        .leading_symbol(SymbolName::Search)
        .placeholder("A long placeholder that must stay on one line");
    let bounds = Rect::new(0.0, 0.0, 120.0, 30.0);
    let placeholder = paint(&field, bounds, &theme, 1.0);
    let geometry = field.geometry(bounds, &theme, &theme.typography, 1.0);
    assert!(text(&placeholder).bounds.size.width > geometry.viewport.size.width);
    assert!(placeholder.commands().iter().any(
        |command| matches!(command, DrawCommand::PushClip { rect } if *rect == geometry.viewport)
    ));
    state.set_value("Value");
    let value = paint(&field, bounds, &theme, 1.0);
    assert_eq!(text(&placeholder).bounds.origin, text(&value).bounds.origin);
    assert_eq!(
        text(&placeholder).bounds.size.height,
        text(&value).bounds.size.height
    );
}

#[test]
fn pointer_focus_clears_keyboard_ring_and_typing_does_not_restore_it() {
    let mut theme = Theme::LIGHT;
    theme.motion = theme.motion.reduced();
    let state = TextFieldInteractionState::new();
    let field = TextField::with_interaction(state.clone());
    let bounds = Rect::new(10.0, 20.0, 200.0, 30.0);
    let mut measurer = TextMeasurer::new();
    let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
    field.handle_event(
        bounds,
        &ViewEvent::KeyboardFocusRequested {
            bounds: Some(bounds),
        },
        &mut context,
    );
    assert!(state.inner.borrow().focus_visible);
    assert!(paint(&field, bounds, &theme, 1.0).commands().iter().any(|command| matches!(command, DrawCommand::FillRoundedRect { rect, color, .. } if rect.size.width > bounds.size.width && *color == theme.text_field.focus_ring)));
    let position = Point::new(60.0, 35.0);
    for event in [
        ViewEvent::FocusVisibilityChanged { visible: false },
        ViewEvent::PointerFocusRequested { position },
        ViewEvent::PointerPressed {
            position,
            button: PointerButton::Primary,
        },
        ViewEvent::PointerReleased {
            position,
            button: PointerButton::Primary,
        },
        ViewEvent::FocusVisibilityChanged { visible: true },
        ViewEvent::TextInput { text: "a".into() },
    ] {
        field.handle_event(bounds, &event, &mut context);
    }
    assert_eq!(state.value(), "a");
    assert!(!state.inner.borrow().focus_visible);
    let list = paint(&field, bounds, &theme, 1.0);
    assert!(!list.commands().iter().any(|command| match command {
        DrawCommand::FillRoundedRect { rect, .. } => rect.size.width > bounds.size.width,
        DrawCommand::StrokeInsideRoundedRect { color, .. } =>
            *color == theme.text_field.focused_border,
        _ => false,
    }));
}

#[test]
fn cursor_position_matches_rendered_text_after_a_leading_symbol_and_font_scaling() {
    let theme = Theme::LIGHT;
    for scale in [1.0, 2.0] {
        let state = TextFieldInteractionState::new();
        state.set_value("abあ");
        let field = TextField::with_interaction(state.clone()).leading_symbol(SymbolName::Search);
        let bounds = Rect::new(10.0, 20.0, 300.0, 30.0);
        let rendered = paint(&field, bounds, &theme, scale);
        let mut measurer = TextMeasurer::new();
        measurer.set_font_scale(scale);
        let width = Text::styled("a", TextRole::Label)
            .measure_unbounded_with_typography(&mut measurer, &theme.typography)
            .width;
        let position = Point::new(text(&rendered).bounds.origin.x + width, 35.0);
        field.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            },
            &mut EventContext::new(&theme, &theme.typography, &mut measurer),
        );
        assert_eq!(state.selection_range(), 1..1);
        // Begin the visible phase at paint time; font loading under parallel
        // test load must not turn this geometry assertion into a blink timer test.
        state.inner.borrow_mut().caret_blink_origin = None;
        let list = paint(&field, bounds, &theme, scale);
        assert!(list.commands().iter().any(|command| matches!(command, DrawCommand::FillRoundedRect { rect, color, .. } if *color == theme.text_field.caret && (rect.origin.x + rect.size.width * 0.5 - position.x).abs() < 0.01)));
    }
}

#[test]
fn disabled_symbols_and_text_use_disabled_tokens_without_an_error_outline() {
    let theme = Theme::LIGHT;
    let field = TextField::with_interaction(TextFieldInteractionState::new())
        .leading_symbol(SymbolName::Search)
        .placeholder("Disabled")
        .enabled(false)
        .invalid(true);
    let list = paint(&field, Rect::new(0.0, 0.0, 200.0, 30.0), &theme, 1.0);
    assert_eq!(text(&list).color, theme.text_field.disabled_foreground);
    assert!(list.commands().iter().any(|command| matches!(command, DrawCommand::DrawSvg { command } if command.tint == Some(theme.text_field.disabled_foreground))));
    assert!(!list.commands().iter().any(|command| matches!(command, DrawCommand::StrokeInsideRoundedRect { color, .. } if *color == theme.text_field.invalid_border)));
}

#[test]
fn disabling_cancels_pointer_selection_before_reenabling() {
    let theme = Theme::LIGHT;
    let state = TextFieldInteractionState::new();
    state.set_value("hello");
    let field = TextField::with_interaction(state.clone());
    let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
    let mut text = TextMeasurer::new();
    let mut context = EventContext::new(&theme, &theme.typography, &mut text);
    field.handle_event(
        bounds,
        &ViewEvent::PointerPressed {
            position: Point::new(20.0, 15.0),
            button: PointerButton::Primary,
        },
        &mut context,
    );
    let selection = state.selection_range();
    paint(
        &TextField::with_interaction(state.clone()).enabled(false),
        bounds,
        &theme,
        1.0,
    );
    field.handle_event(
        bounds,
        &ViewEvent::PointerMoved {
            position: Point::new(180.0, 15.0),
        },
        &mut context,
    );
    assert_eq!(state.selection_range(), selection);
    assert!(!state.inner.borrow().selecting);
    assert_eq!(state.value(), "hello");
}

#[test]
fn inactive_window_preserves_validation_error_without_a_focus_ring() {
    let theme = Theme::LIGHT;
    let state = TextFieldInteractionState::new();
    state.set_focused(true);
    state.inner.borrow_mut().focus_visible = true;
    let field = TextField::with_interaction(state).invalid(true);
    let bounds = Rect::new(0.0, 0.0, 200.0, 30.0);
    let mut text = TextMeasurer::new();
    let mut list = DisplayList::new();
    field.paint(
        bounds,
        &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text)
            .with_window_active(false),
    );
    assert!(list.commands().iter().any(|command| matches!(command, DrawCommand::StrokeInsideRoundedRect { color, .. } if *color == theme.text_field.invalid_border)));
    assert!(!list.commands().iter().any(|command| matches!(command, DrawCommand::FillRoundedRect { rect, .. } if rect.size.width > bounds.size.width)));
}
