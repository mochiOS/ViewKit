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

#[test]
fn custom_artwork_uses_the_symbol_gutter_and_keeps_its_intrinsic_size() {
    struct Artwork(Rc<Cell<Option<Rect>>>);
    impl View for Artwork {
        fn measure(
            &self,
            c: crate::view::Constraints,
            _: &mut crate::view::MeasureContext<'_>,
        ) -> crate::geometry::Size {
            c.constrain(crate::geometry::Size::new(20., 20.))
        }
        fn paint(&self, bounds: Rect, _: &mut PaintContext<'_>) {
            self.0.set(Some(bounds));
        }
    }
    for theme in [Theme::LIGHT, Theme::DARK] {
        let recorded = Rc::new(Cell::new(None));
        let sidebar = SidebarItem::new("Title").leading(Artwork(recorded.clone()));
        let row = ListRow::new("Title").leading_view(Artwork(recorded.clone()));
        let reference = title(&paint(
            &SidebarItem::new("Title").symbol(SymbolName::Folder),
            &theme,
            true,
            false,
        ))
        .0;
        for view in [&sidebar as &dyn View, &row] {
            assert_eq!(
                title(&paint(view, &theme, true, false)).0.origin.x,
                reference.origin.x
            );
            let artwork = recorded.get().unwrap();
            assert_eq!(artwork.size, crate::geometry::Size::new(20., 20.));
            assert_eq!(
                artwork.origin.x + 10.,
                10. + theme.list.row_padding + theme.layout.list_leading_size / 2.
            );
            assert_eq!(artwork.origin.y + 10., 20. + 48. / 2.);
        }
    }
}

#[test]
fn keyboard_focus_stays_inside_joined_rows_and_does_not_imply_selection() {
    for mut theme in [Theme::LIGHT, Theme::DARK] {
        theme.motion = theme.motion.reduced();
        for edges in [(true, true), (true, false), (false, false), (false, true)] {
            for sidebar in [false, true] {
                let state = ButtonInteractionState::new();
                let view: Box<dyn View> = if sidebar {
                    Box::new(SidebarItem::with_interaction(state.clone(), "Title"))
                } else {
                    Box::new(
                        ListRow::with_interaction(state.clone(), "Title")
                            .adjacent_rows(!edges.0, !edges.1),
                    )
                };
                let bounds = Rect::new(10., 20., 260., 48.);
                let mut text = TextMeasurer::new();
                let mut context = EventContext::new(&theme, &theme.typography, &mut text);
                view.handle_event(
                    bounds,
                    &ViewEvent::KeyboardFocusRequested {
                        bounds: Some(bounds),
                    },
                    &mut context,
                );
                let pointer = paint(view.as_ref(), &theme, true, false);
                assert!(!pointer.commands().iter().any(|command| matches!(command,
                    DrawCommand::StrokeInsideRoundedCornersRect { color, .. } if *color == theme.button.focus_ring)));
                view.handle_event(
                    bounds,
                    &ViewEvent::FocusVisibilityChanged { visible: true },
                    &mut context,
                );
                let keyboard = paint(view.as_ref(), &theme, true, false);
                let (rect, radii) = keyboard
                    .commands()
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::StrokeInsideRoundedCornersRect {
                            rect, radii, color, ..
                        } if *color == theme.button.focus_ring => Some((*rect, *radii)),
                        _ => None,
                    })
                    .expect("keyboard focus must survive the row's clip");
                assert_eq!(rect, bounds);
                if !sidebar {
                    assert_eq!(radii[0] > 0., edges.0);
                    assert_eq!(radii[2] > 0., edges.1);
                }
                assert_eq!(background(&keyboard), background(&pointer));
                let inactive = paint(view.as_ref(), &theme, false, false);
                assert!(!inactive.commands().iter().any(|command| matches!(command,
                    DrawCommand::StrokeInsideRoundedCornersRect { color, .. } if *color == theme.button.focus_ring)));
            }
        }
    }
}
