use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChild, StackAlignment, StackGap, ViewExt};
use crate::theme::{Color, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Button, ButtonInteractionState, ButtonStyle, Divider, HStack, Padding, Spacer, Surface, Text,
    VStack, ZStackAlignment,
};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

struct ViewRef<'a, V>
where
    V: View + ?Sized,
{
    view: &'a V,
}

impl<'a, V> ViewRef<'a, V>
where
    V: View + ?Sized,
{
    fn new(view: &'a V) -> Self {
        Self { view }
    }
}

impl<V> View for ViewRef<'_, V>
where
    V: View + ?Sized,
{
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.view.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.view.paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.view.handle_event(bounds, event, context)
    }
}

pub struct MenuItem {
    compact: bool,
    label: String,
    shortcut: Option<String>,

    enabled: bool,
    danger: bool,

    interaction: ButtonInteractionState,
    on_select: Option<Callback>,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            compact: false,
            label: label.into(),
            shortcut: None,

            enabled: true,
            danger: false,

            interaction: ButtonInteractionState::new(),
            on_select: None,
        }
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    pub fn on_select(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_select = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    fn button(&self, theme: &Theme) -> Button {
        let foreground = if !self.enabled {
            theme.menu.disabled_foreground
        } else if self.danger {
            theme.menu.danger_foreground
        } else {
            theme.menu.foreground
        };

        let shortcut_color = if self.enabled {
            theme.menu.secondary_foreground
        } else {
            theme.menu.disabled_foreground
        };

        let label = Text::label(self.label.clone())
            .accessibility_hidden(true)
            .color(foreground);
        let label = if self.compact {
            label
                .font_size(theme.menu.compact_font_size)
                .line_height(theme.menu.compact_row_height)
        } else {
            label
        };
        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(label.layout().flex_grow(1.0));

        if let Some(shortcut) = self.shortcut.as_ref() {
            content = content.child(
                Text::caption(shortcut.clone())
                    .accessibility_hidden(true)
                    .color(shortcut_color),
            );
        }

        let style = if self.danger {
            ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::TRANSPARENT,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.menu.danger_foreground,
            }
        } else {
            ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::TRANSPARENT,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.menu.foreground,
            }
        };

        let mut button = Button::with_interaction(self.interaction.clone())
            .content_padding(false)
            .style(style)
            .radius(theme.menu.item_radius)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .accessibility_role(AccessibilityRole::MenuItem)
            .accessibility_label(self.label.clone())
            .accessibility_value_option(self.shortcut.clone())
            .content(
                Padding::symmetric(
                    if self.compact {
                        0.0
                    } else {
                        theme.menu.item_horizontal_padding
                    },
                    if self.compact {
                        0.0
                    } else {
                        theme.menu.item_vertical_padding
                    },
                )
                .content(content),
            );

        if let Some(on_select) = self.on_select.as_ref() {
            let on_select = Rc::clone(on_select);

            button = button.on_click(move || {
                (on_select.borrow_mut())();
            });
        }

        button
    }
}

impl View for MenuItem {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.button(context.theme).measure(constraints, context)
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
        self.button(context.theme)
            .handle_event(bounds, event, context)
    }
}

pub struct Menu {
    content: VStack,
    compact: bool,
    rows: Vec<(f32, f32, bool, ButtonInteractionState)>,
    height: f32,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            compact: false,
            rows: Vec::new(),
            height: 0.0,
            content: VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None),
        }
    }
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects compact desktop presentation before adding items; other menus retain their defaults.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Separates semantic groups with whitespace.
    pub fn group(mut self) -> Self {
        self.height += Theme::current().menu.compact_group_gap;
        self.content = std::mem::take(&mut self.content).child(
            Spacer::new()
                .into_stack_child()
                .height(Theme::current().menu.compact_group_gap),
        );
        self
    }

    pub fn item(mut self, mut item: MenuItem) -> Self {
        item.compact = self.compact;
        let height = if self.compact {
            Theme::current().menu.compact_row_height
        } else {
            Theme::current().layout.compact_control_height
        };
        self.rows
            .push((self.height, height, item.enabled, item.interaction.clone()));
        self.height += height;
        self.content = std::mem::take(&mut self.content).child(item.height(height));
        self
    }

    pub fn separator(mut self) -> Self {
        let spacing = Theme::current().spacing.extra_small;
        self.height += spacing * 2.0 + Theme::current().divider.thickness;
        self.content = std::mem::take(&mut self.content)
            .child(Spacer::new().into_stack_child().height(spacing))
            .child(Divider::new())
            .child(Spacer::new().into_stack_child().height(spacing));

        self
    }

    /// メニュー項目・グループ間隔・外側余白から必要な高さを返します。
    ///
    /// 構築時と同じテーマを渡してください。ポップアップの描画・入力範囲を
    /// この高さで揃え、フォントや行高の変更時に末尾の項目がはみ出すのを防ぎます。
    pub fn preferred_height(&self, theme: &Theme) -> f32 {
        self.height
            + 2.0
                * if self.compact {
                    theme.menu.compact_padding
                } else {
                    theme.card.compact_padding
                }
    }

    /// Hit-test the visible menu surface using the same continuous corners.
    pub fn contains(&self, bounds: Rect, point: crate::geometry::Point, theme: &Theme) -> bool {
        let radius = if self.compact {
            crate::theme::CornerRadius::Custom(theme.menu.compact_radius)
        } else {
            theme.menu.surface_radius
        };
        bounds.contains_rounded(
            radius.resolve(&theme.radius, bounds.size.width, bounds.size.height),
            point,
        )
    }

    fn card(&self, theme: &Theme) -> Surface<Padding<ViewRef<'_, VStack>>> {
        Surface::floating()
            .radius(theme.menu.surface_radius)
            .content(Padding::all(theme.card.compact_padding).content(ViewRef::new(&self.content)))
    }
}

impl View for Menu {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        if self.compact {
            Padding::all(context.theme.menu.compact_padding)
                .content(ViewRef::new(&self.content))
                .measure(constraints, context)
        } else {
            self.card(context.theme).measure(constraints, context)
        }
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let mut node = AccessibilityNode::new(AccessibilityRole::Menu, bounds);
        node.focus_scope = self.compact;
        context.record_accessibility(node);
        if self.compact {
            Surface::floating()
                .radius(crate::theme::CornerRadius::Custom(
                    context.theme.menu.compact_radius,
                ))
                .paint(bounds, context);
            Padding::all(context.theme.menu.compact_padding)
                .content(ViewRef::new(&self.content))
                .paint(bounds, context);
        } else {
            self.card(context.theme).paint(bounds, context);
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if self.compact {
            if let ViewEvent::KeyPressed {
                key: crate::platform::Key::ArrowDown | crate::platform::Key::ArrowUp,
                ..
            } = event
            {
                let enabled = self
                    .rows
                    .iter()
                    .filter(|(_, _, enabled, _)| *enabled)
                    .collect::<Vec<_>>();
                if !enabled.is_empty() {
                    let current = enabled
                        .iter()
                        .position(|(_, _, _, state)| state.is_focused());
                    let down = matches!(
                        event,
                        ViewEvent::KeyPressed {
                            key: crate::platform::Key::ArrowDown,
                            ..
                        }
                    );
                    let next = match current {
                        Some(i) if down => (i + 1) % enabled.len(),
                        Some(i) => (i + enabled.len() - 1) % enabled.len(),
                        None if down => 0,
                        None => enabled.len() - 1,
                    };
                    let (offset, height, _, _) = enabled[next];
                    let padding = context.theme.menu.compact_padding;
                    context.request_keyboard_focus(Rect::new(
                        bounds.origin.x + padding,
                        bounds.origin.y + padding + offset,
                        (bounds.size.width - padding * 2.0).max(0.0),
                        *height,
                    ));
                }
                return EventResult::Consumed;
            }
        }
        if self.compact {
            Padding::all(context.theme.menu.compact_padding)
                .content(ViewRef::new(&self.content))
                .handle_event(bounds, event, context)
        } else {
            self.card(context.theme())
                .handle_event(bounds, event, context)
        }
    }
}

#[cfg(test)]
mod compact_tests {
    use super::*;
    use crate::{
        platform::{Key, KeyModifiers},
        typography::TextMeasurer,
    };
    use std::cell::Cell;
    #[test]
    fn compact_items_activate_by_keyboard_but_disabled_user_label_does_not() {
        let calls = Rc::new(Cell::new(0));
        let callback_calls = Rc::clone(&calls);
        let menu = Menu::new()
            .compact()
            .item(
                MenuItem::new("User")
                    .enabled(false)
                    .on_select(|| panic!("disabled item activated")),
            )
            .item(
                MenuItem::new("Settings")
                    .on_select(move || callback_calls.set(callback_calls.get() + 1)),
            );
        let theme = Theme::current();
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        let bounds = Rect::new(15.0, 47.0, 150.0, menu.preferred_height(&theme));
        let item_bounds = |index: f32| {
            Rect::new(
                bounds.origin.x + theme.menu.compact_padding,
                bounds.origin.y
                    + theme.menu.compact_padding
                    + theme.menu.compact_row_height * index,
                bounds.size.width - theme.menu.compact_padding * 2.0,
                theme.menu.compact_row_height,
            )
        };
        menu.handle_event(
            bounds,
            &ViewEvent::KeyboardFocusRequested {
                bounds: Some(item_bounds(0.0)),
            },
            &mut context,
        );
        menu.handle_event(
            bounds,
            &ViewEvent::KeyPressed {
                key: Key::Enter,
                modifiers: KeyModifiers::default(),
            },
            &mut context,
        );
        assert_eq!(calls.get(), 0);
        menu.handle_event(
            bounds,
            &ViewEvent::KeyboardFocusRequested {
                bounds: Some(item_bounds(1.0)),
            },
            &mut context,
        );
        menu.handle_event(
            bounds,
            &ViewEvent::KeyPressed {
                key: Key::Enter,
                modifiers: KeyModifiers::default(),
            },
            &mut context,
        );
        assert_eq!(calls.get(), 1);
    }
}
