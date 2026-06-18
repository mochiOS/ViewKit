//! ボタンはあったら押したくなる

use std::cell::RefCell;
use std::rc::Rc;

use crate::draw_command::DrawCommand;
use crate::event::{
    EventContext,
    EventResult,
    ViewEvent,
};
use crate::geometry::Rect;
use crate::layout::{
    IntoStackChild,
    StackChild,
};
use crate::platform::PointerButton;
use crate::theme::{
    Color,
    CornerRadius,
    ShadowStyle,
};
use crate::view::{
    PaintContext,
    View,
};

use super::{
    Rectangle,
    RectangleColor,
    ZStackAlignment,
};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
)]
pub enum ButtonColor {
    Surface,

    #[default]
    Accent,

    Destructive,

    Custom(Color),
}

impl ButtonColor {
    fn resolve(
        self,
        context: &PaintContext<'_>,
    ) -> Color {
        match self {
            Self::Surface => {
                context
                    .theme
                    .colors
                    .elevated_surface
            }

            Self::Accent => {
                context
                    .theme
                    .colors
                    .accent
            }

            Self::Destructive => {
                context
                    .theme
                    .colors
                    .destructive
            }

            Self::Custom(color) => color,
        }
    }

    fn hover_overlay(self) -> Color {
        match self {
            Self::Surface => {
                Color::rgba(
                    0,
                    0,
                    0,
                    16,
                )
            }

            Self::Accent
            | Self::Destructive
            | Self::Custom(_) => {
                Color::rgba(
                    255,
                    255,
                    255,
                    24,
                )
            }
        }
    }

    fn pressed_overlay(self) -> Color {
        match self {
            Self::Surface => {
                Color::rgba(
                    0,
                    0,
                    0,
                    32,
                )
            }

            Self::Accent
            | Self::Destructive
            | Self::Custom(_) => {
                Color::rgba(
                    0,
                    0,
                    0,
                    36,
                )
            }
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
)]
struct ButtonInteractionInner {
    hovered: bool,

    /*
     * このButton上でPrimaryボタンが
     * 押されたかを表します。
     */
    armed: bool,

    /*
     * armedかつ現在ポインターが
     * Button内にある場合にtrueです。
     */
    pressed: bool,

    clicked: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct ButtonInteractionState {
    inner:
        Rc<RefCell<ButtonInteractionInner>>,
}

impl Default for ButtonInteractionState {
    fn default() -> Self {
        Self {
            inner: Rc::new(
                RefCell::new(
                    ButtonInteractionInner {
                        enabled: true,

                        ..ButtonInteractionInner::default()
                    },
                ),
            ),
        }
    }
}

impl ButtonInteractionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_hovered(&self) -> bool {
        self.inner
            .borrow()
            .hovered
    }

    pub fn is_pressed(&self) -> bool {
        self.inner
            .borrow()
            .pressed
    }

    pub fn is_enabled(&self) -> bool {
        self.inner
            .borrow()
            .enabled
    }

    pub fn take_clicked(&self) -> bool {
        let mut inner =
            self.inner.borrow_mut();

        let clicked =
            inner.clicked;

        inner.clicked = false;

        clicked
    }

    pub fn reset(&self) {
        let mut inner =
            self.inner.borrow_mut();

        inner.hovered = false;
        inner.armed = false;
        inner.pressed = false;
        inner.clicked = false;
    }

    fn set_enabled(
        &self,
        enabled: bool,
    ) -> bool {
        let mut inner =
            self.inner.borrow_mut();

        let changed =
            inner.enabled != enabled;

        inner.enabled = enabled;

        if !enabled {
            inner.hovered = false;
            inner.armed = false;
            inner.pressed = false;
        }

        changed
    }

    fn visual_state(
        &self,
    ) -> ButtonVisualState {
        let inner =
            self.inner.borrow();

        if !inner.enabled {
            ButtonVisualState::Disabled
        } else if inner.pressed {
            ButtonVisualState::Pressed
        } else if inner.hovered {
            ButtonVisualState::Hovered
        } else {
            ButtonVisualState::Normal
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
)]
enum ButtonVisualState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

pub struct Button {
    interaction:
        ButtonInteractionState,

    content:
        Option<StackChild>,

    color:
        ButtonColor,

    radius:
        CornerRadius,

    shadow:
        ShadowStyle,

    alignment:
        ZStackAlignment,

    enabled:
        bool,
}

impl Button {
    pub fn new(
        interaction:
        ButtonInteractionState,
    ) -> Self {
        Self {
            interaction,
            content: None,
            color: ButtonColor::Accent,
            radius: CornerRadius::Medium,
            shadow: ShadowStyle::Small,
            alignment: ZStackAlignment::Center,
            enabled: true,
        }
    }

    pub fn content<C>(
        mut self,
        content: C,
    ) -> Self
    where
        C: IntoStackChild,
    {
        self.content = Some(
            content.into_stack_child(),
        );

        self
    }

    pub fn color(
        mut self,
        color: ButtonColor,
    ) -> Self {
        self.color = color;
        self
    }

    pub fn radius(
        mut self,
        radius: CornerRadius,
    ) -> Self {
        self.radius = radius;
        self
    }

    pub fn shadow(
        mut self,
        shadow: ShadowStyle,
    ) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn alignment(
        mut self,
        alignment: ZStackAlignment,
    ) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn enabled(
        mut self,
        enabled: bool,
    ) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn interaction(
        &self,
    ) -> &ButtonInteractionState {
        &self.interaction
    }
}

impl View for Button {
    fn paint(
        &self,
        bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        if bounds.size.width <= 0.0
            || bounds.size.height <= 0.0
        {
            return;
        }

        self.interaction
            .set_enabled(
                self.enabled,
            );

        let visual_state =
            self.interaction
                .visual_state();

        let mut background_color =
            self.color.resolve(
                context,
            );

        if visual_state
            == ButtonVisualState::Disabled
        {
            background_color =
                background_color
                    .with_alpha(96);
        }

        let shadow =
            if visual_state
                == ButtonVisualState::Pressed
            {
                ShadowStyle::None
            } else {
                self.shadow
            };

        Rectangle::new()
            .color(
                RectangleColor::Custom(
                    background_color,
                ),
            )
            .radius(
                self.radius,
            )
            .shadow(
                shadow,
            )
            .paint(
                bounds,
                context,
            );

        let overlay_color =
            match visual_state {
                ButtonVisualState::Hovered => {
                    Some(
                        self.color
                            .hover_overlay(),
                    )
                }

                ButtonVisualState::Pressed => {
                    Some(
                        self.color
                            .pressed_overlay(),
                    )
                }

                ButtonVisualState::Normal
                | ButtonVisualState::Disabled => {
                    None
                }
            };

        if let Some(color) =
            overlay_color
        {
            context.display_list.push(
                DrawCommand::FillRoundedRect {
                    rect: bounds,

                    radius:
                    self.radius.resolve(
                        &context
                            .theme
                            .radius,
                        bounds.size.width,
                        bounds.size.height,
                    ),

                    color,
                },
            );
        }

        let Some(content) =
            self.content.as_ref()
        else {
            return;
        };

        let content_size =
            content.overlay_size(
                bounds.size,
            );

        let content_bounds =
            self.alignment
                .child_bounds(
                    bounds,
                    content_size,
                );

        context.display_list.push(
            DrawCommand::PushClip {
                rect: bounds,
            },
        );

        content.paint(
            content_bounds,
            context,
        );

        context.display_list.push(
            DrawCommand::PopClip,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let enabled_changed =
            self.interaction
                .set_enabled(
                    self.enabled,
                );

        if enabled_changed {
            context.request_redraw();
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            ViewEvent::PointerMoved {
                position,
            } => {
                let mut inner =
                    self.interaction
                        .inner
                        .borrow_mut();

                let hovered =
                    bounds.contains(
                        *position,
                    );

                let pressed =
                    inner.armed
                        && hovered;

                let changed =
                    inner.hovered
                        != hovered
                        || inner.pressed
                        != pressed;

                inner.hovered =
                    hovered;

                inner.pressed =
                    pressed;

                let armed =
                    inner.armed;

                drop(inner);

                if changed {
                    context.request_redraw();
                }

                if armed {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::PointerPressed {
                position,
                button:
                PointerButton::Primary,
            } => {
                if !bounds.contains(
                    *position,
                ) {
                    return EventResult::Ignored;
                }

                let mut inner =
                    self.interaction
                        .inner
                        .borrow_mut();

                inner.hovered = true;
                inner.armed = true;
                inner.pressed = true;

                drop(inner);

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button:
                PointerButton::Primary,
            } => {
                let mut inner =
                    self.interaction
                        .inner
                        .borrow_mut();

                let owned_press =
                    inner.armed;

                if !owned_press {
                    return EventResult::Ignored;
                }

                let inside =
                    bounds.contains(
                        *position,
                    );

                if inside {
                    inner.clicked = true;
                }

                inner.hovered = inside;
                inner.armed = false;
                inner.pressed = false;

                drop(inner);

                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                let mut inner =
                    self.interaction
                        .inner
                        .borrow_mut();

                let changed =
                    inner.hovered
                        || inner.armed
                        || inner.pressed;

                inner.hovered = false;
                inner.armed = false;
                inner.pressed = false;

                drop(inner);

                if changed {
                    context.request_redraw();
                }

                EventResult::Ignored
            }

            ViewEvent::FocusChanged {
                focused: false,
            } => {
                let mut inner =
                    self.interaction
                        .inner
                        .borrow_mut();

                let changed =
                    inner.armed
                        || inner.pressed;

                inner.armed = false;
                inner.pressed = false;

                drop(inner);

                if changed {
                    context.request_redraw();
                }

                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }
}