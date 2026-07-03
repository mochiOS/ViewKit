//! Viewツリー内部で使用するイベント配送API

use crate::geometry::{Point, Rect};
use crate::platform::{ButtonState, PlatformEvent, PointerButton};
use crate::theme::Theme;
use crate::typography::{TextMeasurer, Typography};
use crate::view::View;

#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    PointerMoved {
        position: Point,
    },

    PointerPressed {
        position: Point,
        button: PointerButton,
    },

    PointerReleased {
        position: Point,
        button: PointerButton,
    },

    PointerLeft,

    Scroll {
        position: Point,
        delta_x: f32,
        delta_y: f32,
    },

    TextInput {
        text: String,
    },

    Backspace,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    Delete,

    SelectRight,
    SelectLeft,
    SelectHome,
    SelectEnd,
    SelectAll,

    PointerFocusRequested {
        position: Point,
    },

    FocusChanged {
        focused: bool,
    },
}

impl ViewEvent {
    pub fn position(&self) -> Option<Point> {
        match self {
            Self::PointerMoved { position }
            | Self::PointerPressed { position, .. }
            | Self::PointerReleased { position, .. }
            | Self::Scroll { position, .. }
            | Self::PointerFocusRequested { position } => Some(*position),

            Self::PointerLeft
            | Self::TextInput { .. }
            | Self::FocusChanged { .. }
            | Self::Backspace
            | Self::Delete
            | Self::ArrowLeft
            | Self::ArrowRight
            | Self::Home
            | Self::End
            | Self::SelectRight
            | Self::SelectLeft
            | Self::SelectHome
            | Self::SelectEnd
            | Self::SelectAll => None,
        }
    }

    pub fn is_inside(&self, bounds: Rect) -> bool {
        self.position()
            .map(|position| bounds.contains(position))
            .unwrap_or(true)
    }

    // TODO: ポインターキャプチャへ置き換える
    pub fn requires_broadcast(&self) -> bool {
        matches!(
            self,
            Self::PointerMoved { .. }
                | Self::PointerReleased { .. }
                | Self::PointerFocusRequested { .. }
                | Self::PointerLeft
                | Self::TextInput { .. }
                | Self::Backspace
                | Self::Delete
                | Self::Home
                | Self::End
                | Self::ArrowLeft
                | Self::ArrowRight
                | Self::FocusChanged { .. }
                | Self::SelectLeft
                | Self::SelectRight
                | Self::SelectHome
                | Self::SelectEnd
                | Self::SelectAll
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventResult {
    #[default]
    Ignored,

    Consumed,
}

impl EventResult {
    pub fn is_consumed(self) -> bool {
        self == Self::Consumed
    }

    pub fn merge(self, other: Self) -> Self {
        if self.is_consumed() || other.is_consumed() {
            Self::Consumed
        } else {
            Self::Ignored
        }
    }
}

pub struct EventContext<'a> {
    pub(crate) theme: &'a Theme,

    pub(crate) typography: &'a Typography,

    pub(crate) text_measurer: &'a mut TextMeasurer,

    redraw_requested: bool,
}

impl<'a> EventContext<'a> {
    pub fn new(
        theme: &'a Theme,
        typography: &'a Typography,
        text_measurer: &'a mut TextMeasurer,
    ) -> Self {
        Self {
            theme,
            typography,
            text_measurer,
            redraw_requested: false,
        }
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    pub fn typography(&self) -> &Typography {
        self.typography
    }

    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn redraw_requested(&self) -> bool {
        self.redraw_requested
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EventDispatcher {
    pointer_position: Option<Point>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pointer_position(&self) -> Option<Point> {
        self.pointer_position
    }

    pub fn dispatch(
        &mut self,
        root: &dyn View,
        bounds: Rect,
        event: &PlatformEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let mut result = EventResult::Ignored;

        let is_primary_press = matches!(
            event,
            PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            }
        );

        if is_primary_press {
            if let Some(position) = self.pointer_position {
                result = result.merge(root.handle_event(
                    bounds,
                    &ViewEvent::PointerFocusRequested { position },
                    context,
                ));
            }
        }

        let Some(view_event) = self.convert_event(event) else {
            return result;
        };

        result.merge(root.handle_event(bounds, &view_event, context))
    }

    fn convert_event(&mut self, event: &PlatformEvent) -> Option<ViewEvent> {
        match event {
            PlatformEvent::PointerMoved { x, y } => {
                let position = Point::new(*x, *y);

                self.pointer_position = Some(position);

                Some(ViewEvent::PointerMoved { position })
            }

            PlatformEvent::PointerButton { button, state } => {
                let position = self.pointer_position?;

                match state {
                    ButtonState::Pressed => Some(ViewEvent::PointerPressed {
                        position,
                        button: *button,
                    }),

                    ButtonState::Released => Some(ViewEvent::PointerReleased {
                        position,
                        button: *button,
                    }),
                }
            }

            PlatformEvent::PointerLeft => {
                self.pointer_position = None;

                Some(ViewEvent::PointerLeft)
            }

            PlatformEvent::Scroll { delta_x, delta_y } => {
                let position = self.pointer_position?;

                Some(ViewEvent::Scroll {
                    position,
                    delta_x: *delta_x,
                    delta_y: *delta_y,
                })
            }

            PlatformEvent::Focused(focused) => {
                if !focused {
                    self.pointer_position = None;
                }

                Some(ViewEvent::FocusChanged { focused: *focused })
            }

            PlatformEvent::TextInput { text } => Some(ViewEvent::TextInput { text: text.clone() }),
            PlatformEvent::Backspace => Some(ViewEvent::Backspace),
            PlatformEvent::Delete => Some(ViewEvent::Delete),
            PlatformEvent::ArrowLeft => Some(ViewEvent::ArrowLeft),
            PlatformEvent::ArrowRight => Some(ViewEvent::ArrowRight),
            PlatformEvent::Home => Some(ViewEvent::Home),
            PlatformEvent::End => Some(ViewEvent::End),

            PlatformEvent::SelectLeft => Some(ViewEvent::SelectLeft),
            PlatformEvent::SelectRight => Some(ViewEvent::SelectRight),
            PlatformEvent::SelectHome => Some(ViewEvent::SelectHome),
            PlatformEvent::SelectEnd => Some(ViewEvent::SelectEnd),
            PlatformEvent::SelectAll => Some(ViewEvent::SelectAll),

            PlatformEvent::Resumed { .. }
            | PlatformEvent::Resized { .. }
            | PlatformEvent::ScaleFactorChanged { .. }
            | PlatformEvent::RedrawRequested
            | PlatformEvent::CloseRequested => None,
        }
    }
}
