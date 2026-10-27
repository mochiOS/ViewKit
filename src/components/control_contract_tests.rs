use super::*;
use crate::draw_command::{DisplayList, DrawCommand};
use crate::geometry::{Rect, Size};
use crate::layout::{StackGap, ViewExt};
use crate::state::State;
use crate::theme::{Color, ControlSize, Theme};
use crate::typography::TextMeasurer;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[test]
fn controls_share_size_categories_and_respect_theme_metrics() {
    for mut theme in [Theme::LIGHT, Theme::DARK] {
        theme.controls.small_height = 25.0;
        theme.controls.height = 31.0;
        theme.controls.large_height = 35.0;
        let mut text = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut text,
        };
        for (size, button, field) in [
            (ControlSize::Small, ButtonSize::Small, TextFieldSize::Small),
            (
                ControlSize::Medium,
                ButtonSize::Medium,
                TextFieldSize::Medium,
            ),
            (ControlSize::Large, ButtonSize::Large, TextFieldSize::Large),
        ] {
            let views: Vec<Box<dyn View>> = vec![
                Box::new(Button::new("Apply").size(button)),
                Box::new(TextField::new(State::new("value".into()).binding()).size(field)),
                Box::new(Picker::new("Tokyo").size(size)),
                Box::new(Stepper::new(State::new(3).binding()).size(size)),
            ];
            for view in views {
                assert_eq!(
                    view.measure(Constraints::loose(Size::new(300.0, 100.0)), &mut context)
                        .height,
                    size.height(&theme.controls)
                );
                assert_eq!(
                    view.measure(
                        Constraints::new(Size::new(120.0, 22.0), Size::new(120.0, 22.0)),
                        &mut context
                    )
                    .height,
                    22.0
                );
            }
        }
    }
}

#[test]
fn custom_button_content_inherits_state_without_leaking_or_overwriting_explicit_colors() {
    for theme in [Theme::LIGHT, Theme::DARK] {
        for enabled in [true, false] {
            let button = Button::new("")
                .style(ButtonStyle::Primary)
                .enabled(enabled)
                .content(
                    HStack::new()
                        .gap(StackGap::Small)
                        .child(Icon::new(SymbolName::Plus).size(16.0).frame(16.0, 16.0))
                        .child(Text::label("Inherited"))
                        .child(Text::label("Explicit").color(Color::from_rgb_hex(0x884422))),
                );
            let mut list = DisplayList::new();
            let mut text = TextMeasurer::new();
            let mut context = PaintContext::new(&mut list, &theme, &theme.typography, &mut text);
            button.paint(Rect::new(0.0, 0.0, 260.0, 36.0), &mut context);
            assert_eq!(context.control_foreground(), None);
            Icon::new(SymbolName::Plus)
                .size(16.0)
                .paint(Rect::new(0.0, 50.0, 16.0, 16.0), &mut context);
            let inherited = list
                .commands()
                .iter()
                .find_map(|command| match command {
                    DrawCommand::DrawText { command } if command.text == "Inherited" => {
                        Some(command.color)
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                inherited,
                Color::WHITE.with_alpha(if enabled {
                    255
                } else {
                    (255.0 * theme.button.disabled_opacity).round() as u8
                })
            );
            let symbols: Vec<_> = list
                .commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::DrawSvg { command } => command.tint,
                    _ => None,
                })
                .collect();
            assert_eq!(symbols, vec![inherited, theme.colors.text_primary]);
            assert!(list.commands().iter().any(|command| matches!(command,
                DrawCommand::DrawText { command } if command.text == "Explicit" && command.color == Color::from_rgb_hex(0x884422)
            )));
        }
    }
}

#[test]
fn toolbar_buttons_keep_compact_symbols_and_quiet_resting_surfaces() {
    let theme = Theme::LIGHT;
    let mut text = TextMeasurer::new();
    let button = Button::new("").leading_symbol(SymbolName::Plus).toolbar();
    let measured = button.measure(
        Constraints::loose(Size::new(300.0, 100.0)),
        &mut MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut text,
        },
    );
    assert_eq!(
        measured,
        Size::new(theme.controls.height, theme.controls.height)
    );
    let mut list = DisplayList::new();
    button.paint(
        Rect::new(0.0, 0.0, 28.0, 30.0),
        &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
    );
    assert!(list.commands().iter().any(|command| matches!(command, DrawCommand::DrawSvg { command } if command.bounds.size.width > 10.0)));
    assert!(!list.commands().iter().any(|command| matches!(command,
        DrawCommand::FillRoundedRect { color, .. } if color.alpha != 0
    )));
}
