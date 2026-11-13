use super::*;
use crate::{
    components::{Identified, TextField, TextFieldInteractionState, TitleBar},
    decoration::{DecorationConfig, DecorationLayout, WindowState},
    runtime::ComponentInstanceId,
    state::State,
};
use crate::{
    geometry::Size,
    platform::{ButtonState, PointerButton},
};

struct DecorationApp {
    height: State<f32>,
    dark: State<bool>,
    text: State<String>,
    show: State<bool>,
    field: TextFieldInteractionState,
    notifications: Rc<RefCell<Vec<DecorationLayout>>>,
}
impl App for DecorationApp {
    type Body = Rectangle;
    fn new() -> Self {
        Self {
            height: State::new(44.0),
            dark: State::new(false),
            text: State::new("First".into()),
            show: State::new(true),
            field: TextFieldInteractionState::new(),
            notifications: Rc::new(RefCell::new(Vec::new())),
        }
    }
    fn window(&self) -> crate::app::WindowOptions {
        let mut bar = TitleBar::new()
            .height(self.height.get())
            .center(crate::components::Text::new(self.text.get()));
        if self.show.get() {
            bar = bar.leading(Identified::new(
                ComponentInstanceId(17),
                TextField::with_interaction(self.field.clone()),
            ));
        }
        crate::app::WindowOptions::new("Decoration Test").titlebar(bar)
    }
    fn body(&self, _: &crate::app::ViewContext) -> Self::Body {
        Rectangle::new()
    }
    fn theme(&self) -> Option<Theme> {
        Some(if self.dark.get() {
            Theme::DARK
        } else {
            Theme::LIGHT
        })
    }
    fn decoration_changed(&mut self, _: WindowId, layout: DecorationLayout) {
        self.notifications.borrow_mut().push(layout);
    }
}
struct DecorationWindow {
    calls: Cell<usize>,
    viewport: Cell<Viewport>,
    layout: Cell<DecorationLayout>,
}
impl DecorationWindow {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
            viewport: Cell::new(Viewport::new(Size::new(640.0, 480.0), 640, 480, 1.0)),
            layout: Cell::new(DecorationLayout::default()),
        }
    }
}
impl PlatformWindow for DecorationWindow {
    fn id(&self) -> WindowId {
        WindowId::PRIMARY
    }
    fn request_redraw(&self) {}
    fn set_title(&self, _: &str) {}
    fn viewport(&self) -> Viewport {
        self.viewport.get()
    }
    fn input_serial(&self) -> u64 {
        73
    }
    fn decoration_layout(&self) -> Option<DecorationLayout> {
        Some(self.layout.get())
    }
    fn configure_decoration(
        &self,
        config: DecorationConfig,
    ) -> Result<DecorationLayout, crate::decoration::DecorationError> {
        self.calls.set(self.calls.get() + 1);
        let layout = DecorationLayout {
            height: config
                .height
                .max(crate::decoration::metrics::TITLE_BAR_HEIGHT as f32),
            controls_width: crate::decoration::metrics::CONTROLS_WIDTH as f32,
            state: self.layout.get().state,
        };
        self.layout.set(layout);
        Ok(layout)
    }
}
#[test]
fn ordinary_titlebar_updates_do_not_renegotiate_or_replace_the_window() {
    let mut runtime = ApplicationRuntime::new(DecorationApp::new());
    let window = DecorationWindow::new();
    let mut list = DisplayList::new();
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 1);
    runtime.app.text.set("Changed".into());
    runtime.draw(&window, &mut list);
    runtime.app.show.set(false);
    runtime.draw(&window, &mut list);
    runtime.app.show.set(true);
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 1);
    assert!(runtime.pending_window_commands.is_empty());
    assert_eq!(runtime.windows.len(), 1);
    assert!(runtime.windows.contains_key(&WindowId::PRIMARY));
}
#[test]
fn height_scale_and_theme_changes_relayout_and_state_notifications_do_not_renegotiate() {
    let mut runtime = ApplicationRuntime::new(DecorationApp::new());
    let window = DecorationWindow::new();
    let mut list = DisplayList::new();
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 1);
    runtime.app.height.set(52.0);
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 2);
    window
        .viewport
        .set(Viewport::new(Size::new(640.0, 480.0), 1280, 960, 2.0));
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 3);
    runtime.app.dark.set(true);
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 4);
    let mut layout = window.layout.get();
    layout.state = WindowState::Maximized;
    window.layout.set(layout);
    runtime.draw(&window, &mut list);
    assert_eq!(window.calls.get(), 4);
    assert_eq!(
        runtime.app.notifications.borrow().last().unwrap().state,
        WindowState::Maximized
    );
    assert_eq!(
        runtime.windows[&WindowId::PRIMARY]
            .root
            .as_ref()
            .unwrap()
            .layout,
        layout
    );
}
#[test]
fn removing_a_titlebar_field_clears_its_real_focus_without_touching_other_components() {
    let mut runtime = ApplicationRuntime::new(DecorationApp::new());
    let window = DecorationWindow::new();
    let mut list = DisplayList::new();
    runtime.handle_event(PlatformEvent::Focused(true), &window);
    runtime.draw(&window, &mut list);
    runtime.handle_event(PlatformEvent::PointerMoved { x: 10.0, y: 20.0 }, &window);
    runtime.handle_event(
        PlatformEvent::PointerButton {
            button: PointerButton::Primary,
            state: ButtonState::Pressed,
        },
        &window,
    );
    assert!(runtime.app.field.is_focused());
    runtime.app.show.set(false);
    runtime.draw(&window, &mut list);
    assert!(!runtime.app.field.is_focused());
    assert_eq!(window.calls.get(), 1);
}

struct DragCaptionApp {
    clicks: Rc<Cell<usize>>,
    button: crate::components::ButtonInteractionState,
    tabs: crate::components::TabsInteractionState,
    selection: State<usize>,
}
impl App for DragCaptionApp {
    type Body = Rectangle;
    fn new() -> Self {
        Self {
            clicks: Rc::new(Cell::new(0)),
            button: Default::default(),
            tabs: Default::default(),
            selection: State::new(0),
        }
    }
    fn window(&self) -> crate::app::WindowOptions {
        let button_clicks = self.clicks.clone();
        let tab_clicks = self.clicks.clone();
        let content = HStack::new()
            .child(
                Button::with_interaction_and_label(self.button.clone(), "Action")
                    .on_click(move || button_clicks.set(button_clicks.get() + 1)),
            )
            .child(
                crate::components::Tabs::with_interaction(
                    self.selection.binding(),
                    self.tabs.clone(),
                )
                .item(0, "First")
                .item(1, "Second")
                .on_selected_activate(move || tab_clicks.set(tab_clicks.get() + 1)),
            );
        crate::app::WindowOptions::new("Drag caption").titlebar(
            TitleBar::new()
                .height(44.0)
                .center(crate::components::DragRegion::new(content).including_controls()),
        )
    }
    fn body(&self, _: &crate::app::ViewContext) -> Rectangle {
        Rectangle::new()
    }
}
struct DragCaptionWindow {
    base: DecorationWindow,
    event_serial: Cell<u64>,
    held_serial: Cell<u64>,
    moves: Cell<usize>,
}
impl PlatformWindow for DragCaptionWindow {
    fn id(&self) -> WindowId {
        WindowId::PRIMARY
    }
    fn viewport(&self) -> Viewport {
        self.base.viewport()
    }
    fn request_redraw(&self) {}
    fn set_title(&self, _: &str) {}
    fn input_serial(&self) -> u64 {
        self.event_serial.get()
    }
    fn configure_decoration(
        &self,
        config: DecorationConfig,
    ) -> Result<DecorationLayout, crate::decoration::DecorationError> {
        self.base.configure_decoration(config)
    }
    fn decoration_layout(&self) -> Option<DecorationLayout> {
        self.base.decoration_layout()
    }
    fn begin_window_interaction(
        &self,
        serial: u64,
        operation: crate::decoration::WindowInteraction,
    ) -> bool {
        if serial == 0 || serial != self.held_serial.replace(0) {
            return false;
        }
        assert_eq!(operation, crate::decoration::WindowInteraction::Move);
        self.moves.set(self.moves.get() + 1);
        true
    }
}
#[test]
fn dragging_caption_buttons_and_tabs_moves_the_window_without_clicking_them() {
    for label in ["Action", "First"] {
        let mut runtime = ApplicationRuntime::new(DragCaptionApp::new());
        let window = DragCaptionWindow {
            base: DecorationWindow::new(),
            event_serial: Cell::new(0),
            held_serial: Cell::new(0),
            moves: Cell::new(0),
        };
        runtime.draw(&window, &mut DisplayList::new());
        let rect = runtime.windows[&WindowId::PRIMARY]
            .accessibility_nodes
            .iter()
            .find(|n| n.label.as_deref() == Some(label))
            .unwrap()
            .bounds;
        let point = Point::new(
            rect.origin.x + rect.size.width / 2.0,
            rect.origin.y + rect.size.height / 2.0,
        );
        let press = PlatformEvent::PointerButton {
            button: PointerButton::Primary,
            state: ButtonState::Pressed,
        };
        let release = PlatformEvent::PointerButton {
            button: PointerButton::Primary,
            state: ButtonState::Released,
        };
        runtime.handle_event(
            PlatformEvent::PointerMoved {
                x: point.x,
                y: point.y,
            },
            &window,
        );
        window.event_serial.set(73);
        window.held_serial.set(73);
        runtime.handle_event(press.clone(), &window);
        window.event_serial.set(0);
        runtime.handle_event(
            PlatformEvent::PointerMoved {
                x: point.x + 0.5,
                y: point.y,
            },
            &window,
        );
        assert_eq!(window.moves.get(), 0);
        window.held_serial.set(0);
        runtime.handle_event(release.clone(), &window);
        assert_eq!(runtime.app.clicks.get(), 1, "normal click on {label}");
        runtime.draw(&window, &mut DisplayList::new());
        runtime.handle_event(
            PlatformEvent::PointerMoved {
                x: point.x,
                y: point.y,
            },
            &window,
        );
        window.event_serial.set(74);
        window.held_serial.set(74);
        runtime.handle_event(press, &window);
        window.event_serial.set(0);
        runtime.handle_event(
            PlatformEvent::PointerMoved {
                x: point.x + 20.0,
                y: point.y,
            },
            &window,
        );
        assert_eq!(window.moves.get(), 1, "drag on {label}");
        runtime.handle_event(
            PlatformEvent::PointerMoved {
                x: point.x,
                y: point.y,
            },
            &window,
        );
        runtime.handle_event(release, &window);
        assert_eq!(
            runtime.app.clicks.get(),
            1,
            "release after dragging must not click {label}"
        );
        assert_eq!(window.moves.get(), 1);
        assert!(
            runtime.windows[&WindowId::PRIMARY]
                .pending_window_drag
                .is_none()
        );
    }
}
#[test]
fn released_or_unfocused_caption_presses_cannot_start_a_later_move() {
    for finish in [
        PlatformEvent::Focused(false),
        PlatformEvent::PointerButton {
            button: PointerButton::Primary,
            state: ButtonState::Released,
        },
    ] {
        let mut runtime = ApplicationRuntime::new(DragCaptionApp::new());
        let window = DragCaptionWindow {
            base: DecorationWindow::new(),
            event_serial: Cell::new(73),
            held_serial: Cell::new(73),
            moves: Cell::new(0),
        };
        runtime.draw(&window, &mut DisplayList::new());
        let rect = runtime.windows[&WindowId::PRIMARY]
            .accessibility_nodes
            .iter()
            .find(|n| n.label.as_deref() == Some("Action"))
            .unwrap()
            .bounds;
        let x = rect.origin.x + rect.size.width / 2.0;
        let y = rect.origin.y + rect.size.height / 2.0;
        runtime.handle_event(PlatformEvent::PointerMoved { x, y }, &window);
        runtime.handle_event(
            PlatformEvent::PointerButton {
                button: PointerButton::Primary,
                state: ButtonState::Pressed,
            },
            &window,
        );
        assert!(
            runtime.windows[&WindowId::PRIMARY]
                .pending_window_drag
                .is_some()
        );
        window.event_serial.set(0);
        window.held_serial.set(0);
        runtime.handle_event(finish, &window);
        runtime.handle_event(PlatformEvent::PointerMoved { x: x + 50.0, y }, &window);
        assert_eq!(window.moves.get(), 0);
        assert!(
            runtime.windows[&WindowId::PRIMARY]
                .pending_window_drag
                .is_none()
        );
    }
}
