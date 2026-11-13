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
