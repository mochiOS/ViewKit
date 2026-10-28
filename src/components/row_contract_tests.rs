use super::*;
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{EventContext, ViewEvent};
use crate::geometry::{Point, Rect};
use crate::platform::PointerButton;
use crate::theme::{Color, Theme};
use crate::typography::TextMeasurer;
use crate::view::{PaintContext, View};
use std::cell::Cell;
use std::rc::Rc;

fn paint(view: &dyn View, theme: &Theme, active: bool, hovered: bool) -> DisplayList {
    let bounds = Rect::new(10.0, 20.0, 260.0, 48.0);
    let mut text = TextMeasurer::new();
    if hovered {
        view.handle_event(
            bounds,
            &ViewEvent::PointerMoved {
                position: Point::new(100.0, 40.0),
            },
            &mut EventContext::new(theme, &theme.typography, &mut text),
        );
    }
    let mut list = DisplayList::new();
    view.paint(
        bounds,
        &mut PaintContext::new(&mut list, theme, &theme.typography, &mut text)
            .with_window_active(active),
    );
    list
}
fn title(list: &DisplayList) -> (Rect, Color) {
    list.commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::DrawText { command } if command.text == "Title" => {
                Some((command.bounds, command.color))
            }
            _ => None,
        })
        .unwrap()
}
fn background(list: &DisplayList) -> Option<Color> {
    list.commands().iter().find_map(|command| match command {
        DrawCommand::FillRoundedRect { color, .. } => Some(*color),
        _ => None,
    })
}

#[test]
fn sidebar_and_list_share_selected_hovered_inactive_and_disabled_colors() {
    for mut theme in [Theme::LIGHT, Theme::DARK] {
        theme.motion = theme.motion.reduced();
        for active in [false, true] {
            for selected in [false, true] {
                for enabled in [false, true] {
                    for hovered in [false, true] {
                        let row = ListRow::new("Title").selected(selected).enabled(enabled);
                        let sidebar = SidebarItem::new("Title")
                            .selected(selected)
                            .enabled(enabled);
                        let row_paint = paint(&row, &theme, active, hovered);
                        let sidebar_paint = paint(&sidebar, &theme, active, hovered);
                        assert_eq!(background(&row_paint), background(&sidebar_paint));
                        assert_eq!(title(&row_paint), title(&sidebar_paint));
                        let expected = if !enabled {
                            theme.list.disabled_foreground
                        } else if selected && active {
                            theme.list.selected_foreground
                        } else if selected {
                            theme.list.inactive_selected_foreground
                        } else {
                            theme.list.foreground
                        };
                        assert_eq!(title(&row_paint).1, expected);
                        if selected && enabled {
                            assert_eq!(
                                background(&row_paint),
                                Some(if active {
                                    theme.list.selected_background
                                } else {
                                    theme.list.inactive_selected_background
                                })
                            );
                        }
                        if !selected && active && enabled && hovered {
                            assert_eq!(background(&row_paint), Some(theme.list.hovered_background));
                            assert_ne!(
                                theme.list.hovered_background,
                                theme.list.selected_background
                            );
                        }
                        if !selected && !active {
                            assert_eq!(
                                background(&row_paint).unwrap_or(Color::TRANSPARENT).alpha,
                                0
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn symbols_and_reserved_slots_align_labels_between_sidebar_and_list() {
    let theme = Theme::LIGHT;
    let row = ListRow::new("Title").icon(SymbolName::Folder);
    let sidebar = SidebarItem::new("Title").symbol(SymbolName::Folder);
    let reserved = SidebarItem::new("Title").reserve_symbol_space();
    let row_title = title(&paint(&row, &theme, true, false)).0;
    for view in [&sidebar, &reserved] {
        assert_eq!(
            title(&paint(view, &theme, true, false)).0.origin.x,
            row_title.origin.x
        );
    }
    assert_eq!(
        row_title.origin.x,
        10.0 + theme.list.row_padding + theme.layout.list_leading_size + theme.spacing.medium
    );
}

#[test]
fn focus_loss_clears_hover_and_pending_press_without_activating_or_deselecting() {
    let theme = Theme::LIGHT;
    for sidebar in [false, true] {
        let hits = Rc::new(Cell::new(0));
        let state = ButtonInteractionState::new();
        let hit = hits.clone();
        let view: Box<dyn View> = if sidebar {
            Box::new(
                SidebarItem::with_interaction(state.clone(), "Title")
                    .selected(true)
                    .on_select(move || hit.set(hit.get() + 1)),
            )
        } else {
            Box::new(
                ListRow::with_interaction(state.clone(), "Title")
                    .selected(true)
                    .on_select(move || hit.set(hit.get() + 1)),
            )
        };
        let bounds = Rect::new(10.0, 20.0, 260.0, 48.0);
        let point = Point::new(100.0, 40.0);
        let mut text = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut text);
        view.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position: point,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert!(state.is_pressed());
        view.handle_event(
            bounds,
            &ViewEvent::FocusChanged { focused: false },
            &mut context,
        );
        assert!(!state.is_hovered() && !state.is_pressed());
        view.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position: point,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert_eq!(hits.get(), 0);
        let mut nodes = Vec::new();
        let mut list = DisplayList::new();
        view.paint(
            bounds,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text)
                .with_window_active(false)
                .with_accessibility_nodes(&mut nodes),
        );
        assert!(nodes.iter().any(|node| node.selected));
    }
}
