#![allow(clippy::missing_const_for_thread_local)]

//! ViewKitアプリケーションとプラットフォームバックエンドをガッッッッタイ！します

use std::cell::Cell;
use std::time::Instant;

use crate::accessibility::AccessibilityNode;
use crate::app::{App, ViewContext};
use crate::appearance::AppearanceSettings;
use crate::draw_command::{DisplayList, DrawCommand};
use crate::components::{BorderStyle, Rectangle, RectangleColor, Text};
use crate::event::{ContextMenuRequest, EventContext, EventDispatcher, RedrawRequest};
use crate::geometry::{Point, Rect};
use crate::platform::{
    ButtonState, Key, PlatformApplication, PlatformEvent, PlatformWindow, PointerButton,
    WindowConfig,
};
use crate::renderer::Viewport;
use crate::state::take_state_changed;
use crate::theme::{ShadowStyle, Theme};
use crate::typography::TextMeasurer;
use crate::view::{PaintContext, RedrawSchedule, View};

thread_local! {
    static EXIT_REQUESTED: Cell<bool> = const { Cell::new(false) };
}

/// 現在のViewKitアプリケーションへ正常終了を要求します。
///
/// プラットフォームのイベントループを抜け、WindowやSurfaceを破棄してから
/// [`run`]を返します。
pub fn request_exit() {
    EXIT_REQUESTED.with(|requested| requested.set(true));
}

fn exit_requested() -> bool {
    EXIT_REQUESTED.with(Cell::get)
}

fn reset_exit_request() {
    EXIT_REQUESTED.with(|requested| requested.set(false));
}

/// `App`をプラットフォームバックエンド上で実行するランタイムです。
pub(crate) struct ApplicationRuntime<A>
where
    A: App,
{
    app: A,

    root: Option<A::Body>,
    viewport: Option<Viewport>,
    theme: Theme,
    text_measurer: TextMeasurer,
    accessibility_nodes: Vec<AccessibilityNode>,
    appearance: AppearanceSettings,

    event_dispatcher: EventDispatcher,
    redraw_schedule: RedrawSchedule,
    pending_redraw: RedrawRequest,
    fallback_context_menu: Option<FallbackContextMenu>,
}

struct FallbackContextMenu {
    request: ContextMenuRequest,
    pointer: Option<Point>,
}

impl<A> ApplicationRuntime<A>
where
    A: App,
{
    pub(crate) fn new(app: A) -> Self {
        reset_exit_request();
        let appearance = AppearanceSettings::load();
        let theme = appearance.theme();
        Theme::set_current(theme);
        let mut text_measurer = TextMeasurer::new();
        text_measurer.set_font_scale(appearance.font_scale());
        Self {
            app,

            root: None,
            viewport: None,
            theme,
            text_measurer,
            accessibility_nodes: Vec::new(),
            appearance,

            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),
            pending_redraw: RedrawRequest::None,
            fallback_context_menu: None,
        }
    }

    fn rebuild_root(&mut self, viewport: Viewport) {
        self.rebuild_root_with_redraw(viewport, RedrawRequest::Full);
    }

    fn rebuild_root_with_redraw(&mut self, viewport: Viewport, redraw: RedrawRequest) {
        let context = ViewContext::new(viewport);

        Theme::set_current(self.theme);
        self.root = Some(self.app.body(&context));
        self.viewport = Some(viewport);
        self.pending_redraw = redraw;

        let _ = take_state_changed();
    }

    fn ensure_root(&mut self, viewport: Viewport) {
        let viewport_changed = self.viewport != Some(viewport);

        if self.root.is_none() || viewport_changed {
            self.rebuild_root(viewport);
        }
    }
}

impl<A> PlatformApplication for ApplicationRuntime<A>
where
    A: App,
{
    fn handle_platform_message(&mut self, message: &[u8]) -> bool {
        let handled = self.app.handle_platform_message(message);
        if handled
            && take_state_changed()
            && let Some(viewport) = self.viewport
        {
            self.rebuild_root(viewport);
        }
        handled
    }

    fn handle_event(&mut self, mut event: PlatformEvent, window: &dyn PlatformWindow) {
        match &event {
            PlatformEvent::Resumed { viewport }
            | PlatformEvent::Resized { viewport }
            | PlatformEvent::ScaleFactorChanged { viewport } => {
                self.rebuild_root(*viewport);
                return;
            }

            PlatformEvent::RedrawRequested | PlatformEvent::CloseRequested => {
                return;
            }

            _ => {}
        }

        let viewport = window.viewport();

        self.ensure_root(viewport);

        if let Some(menu) = self.fallback_context_menu.as_mut() {
            match event.clone() {
                PlatformEvent::PointerMoved { x, y } => {
                    menu.pointer = Some(Point::new(x, y));
                    self.pending_redraw = RedrawRequest::Full;
                    window.request_redraw();
                    return;
                }
                PlatformEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Pressed,
                } => return,
                PlatformEvent::PointerButton {
                    button: PointerButton::Primary,
                    state: ButtonState::Released,
                } => {
                    let request_id = menu.request.request_id;
                    let command_id = menu.pointer.and_then(|position| {
                        fallback_menu_command_at(
                            &menu.request,
                            position,
                            viewport.logical_bounds(),
                            &self.theme,
                        )
                    });
                    self.fallback_context_menu = None;
                    event = PlatformEvent::ContextMenuResult {
                        request_id,
                        command_id,
                    };
                }
                PlatformEvent::KeyPressed {
                    key: Key::Escape, ..
                }
                | PlatformEvent::Focused(false) => {
                    let request_id = menu.request.request_id;
                    self.fallback_context_menu = None;
                    event = PlatformEvent::ContextMenuResult {
                        request_id,
                        command_id: None,
                    };
                }
                _ => return,
            }
        }

        let (redraw_request, cursor_icon, context_menu_request) = {
            let root = self
                .root
                .as_ref()
                .expect("root view must exist after ensure_root");

            let mut context = EventContext::new(
                &self.theme,
                &self.theme.typography,
                &mut self.text_measurer,
            );

            self.event_dispatcher
                .dispatch(root, viewport.logical_bounds(), &event, &mut context);

            (
                context.redraw_request(),
                context.cursor_icon(),
                context.take_context_menu_request(),
            )
        };

        if let Some(cursor_icon) = cursor_icon {
            window.set_cursor(cursor_icon);
        }
        if let Some(request) = context_menu_request {
            if !window.show_context_menu(&request) {
                self.fallback_context_menu = Some(FallbackContextMenu {
                    request,
                    pointer: self.event_dispatcher.pointer_position(),
                });
                self.pending_redraw = RedrawRequest::Full;
                window.request_redraw();
            }
        }

        let state_changed = take_state_changed();
        let redraw_request = redraw_after_event(state_changed, redraw_request);

        if state_changed {
            self.rebuild_root_with_redraw(viewport, redraw_request);
        } else {
            self.pending_redraw = self.pending_redraw.merge(redraw_request);
        }

        if state_changed || redraw_request.is_requested() {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) -> Rect {
        self.ensure_root(viewport);
        if take_state_changed() {
            self.rebuild_root(viewport);
        }

        let viewport_bounds = viewport.logical_bounds();
        let scheduled_redraw = self.redraw_schedule.take_due(Instant::now());
        self.pending_redraw = self.pending_redraw.merge(scheduled_redraw);

        let dirty_bounds = match std::mem::take(&mut self.pending_redraw) {
            RedrawRequest::Region(bounds) => bounds
                .intersection(viewport_bounds)
                .unwrap_or(viewport_bounds),

            RedrawRequest::None | RedrawRequest::Full => viewport_bounds,
        };

        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();
        self.accessibility_nodes.clear();

        let mut context = PaintContext::new(
            display_list,
            &self.theme,
            &self.theme.typography,
            &mut self.text_measurer,
        )
        .with_redraw_schedule(&mut self.redraw_schedule)
        .with_accessibility_nodes(&mut self.accessibility_nodes);

        let root = self
            .root
            .as_ref()
            .expect("root view must exist after ensure_root");

        root.paint(viewport_bounds, &mut context);
        if let Some(menu) = &self.fallback_context_menu {
            paint_fallback_context_menu(
                menu,
                viewport_bounds,
                &mut context,
            );
        }
        drop(context);
        self.event_dispatcher
            .set_accessibility_nodes(&self.accessibility_nodes);

        dirty_bounds
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        self.redraw_schedule.deadline()
    }

    fn accessibility_nodes(&self) -> &[AccessibilityNode] {
        &self.accessibility_nodes
    }

    fn reload_appearance(&mut self) -> bool {
        let appearance = AppearanceSettings::load();
        if appearance == self.appearance {
            return false;
        }
        let theme = appearance.theme();
        let font_scale = appearance.font_scale();
        self.appearance = appearance;
        self.theme = theme;
        Theme::set_current(self.theme);
        self.text_measurer.set_font_scale(font_scale);
        self.app.appearance_changed();
        self.root = None;
        self.pending_redraw = RedrawRequest::Full;
        true
    }

    fn interface_scale_factor(&self) -> f64 {
        self.appearance.ui_scale()
    }

    fn exit_requested(&self) -> bool {
        exit_requested()
    }
}

fn fallback_menu_bounds(request: &ContextMenuRequest, viewport: Rect, theme: &Theme) -> Rect {
    let inset = theme.spacing.extra_small;
    let height = request.items.iter().fold(inset * 2.0, |height, item| {
        height
            + if item.separator {
                theme.spacing.small
            } else {
                theme.layout.compact_control_height
            }
    });
    let width = theme.layout.popover_width.min(viewport.size.width);
    let preferred_x = request.position.x;
    let preferred_y = request.position.y;
    let x = preferred_x.clamp(
        viewport.origin.x,
        (viewport.origin.x + viewport.size.width - width).max(viewport.origin.x),
    );
    let below_y = preferred_y;
    let above_y = preferred_y - height;
    let y = if below_y + height <= viewport.origin.y + viewport.size.height {
        below_y
    } else {
        above_y.max(viewport.origin.y)
    };
    Rect::new(x, y, width, height.min(viewport.size.height))
}

fn fallback_menu_rows(
    request: &ContextMenuRequest,
    viewport: Rect,
    theme: &Theme,
) -> Vec<(usize, Rect)> {
    let menu = fallback_menu_bounds(request, viewport, theme);
    let inset = theme.spacing.extra_small;
    let mut y = menu.origin.y + inset;
    request
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let height = if item.separator {
                theme.spacing.small
            } else {
                theme.layout.compact_control_height
            };
            let row = Rect::new(
                menu.origin.x + inset,
                y,
                (menu.size.width - inset * 2.0).max(0.0),
                height,
            );
            y += height;
            (index, row)
        })
        .collect()
}

fn fallback_menu_command_at(
    request: &ContextMenuRequest,
    position: Point,
    viewport: Rect,
    theme: &Theme,
) -> Option<u32> {
    fallback_menu_rows(request, viewport, theme)
        .into_iter()
        .find_map(|(index, bounds)| {
            let item = request.items.get(index)?;
            (bounds.contains(position) && item.enabled && !item.separator)
                .then_some(item.command_id)
        })
}

fn paint_fallback_context_menu(
    menu: &FallbackContextMenu,
    viewport: Rect,
    context: &mut PaintContext<'_>,
) {
    let bounds = fallback_menu_bounds(&menu.request, viewport, context.theme);
    Rectangle::new()
        .color(RectangleColor::Custom(context.theme.card.background))
        .radius(context.theme.menu.surface_radius)
        .shadow(ShadowStyle::Card)
        .border(BorderStyle::custom(
            context.theme.card.border,
            context.theme.menu.surface_stroke_width,
        ))
        .paint(bounds, context);

    for (index, row) in fallback_menu_rows(&menu.request, viewport, context.theme) {
        let Some(item) = menu.request.items.get(index) else {
            continue;
        };
        if item.separator {
            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.colors.border))
                .paint(
                    Rect::new(
                        row.origin.x,
                        row.origin.y + row.size.height * 0.5,
                        row.size.width,
                        context.theme.divider.thickness,
                    ),
                    context,
                );
            continue;
        }

        let hovered = menu.pointer.is_some_and(|position| row.contains(position));
        let background = if item.checked {
            context.theme.colors.accent_soft
        } else if hovered && item.enabled {
            context.theme.menu.item_hovered_background
        } else {
            context.theme.menu.item_background
        };
        Rectangle::new()
            .color(RectangleColor::Custom(background))
            .radius(context.theme.menu.item_radius)
            .paint(row, context);
        let foreground = if item.enabled {
            context.theme.menu.foreground
        } else {
            context.theme.menu.disabled_foreground
        };
        Text::label(item.label.clone()).color(foreground).paint(
            Rect::new(
                row.origin.x + context.theme.menu.item_horizontal_padding,
                row.origin.y,
                (row.size.width - context.theme.menu.item_horizontal_padding * 2.0).max(0.0),
                row.size.height,
            ),
            context,
        );
    }
}

fn redraw_after_event(state_changed: bool, redraw_request: RedrawRequest) -> RedrawRequest {
    if state_changed {
        RedrawRequest::Full
    } else {
        redraw_request
    }
}

/// ViewKitアプリケーションを起動します.
///
/// アプリケーションの初期状態とウィンドウを作成し、
/// 現在のプラットフォームに対応するイベントループを開始します。
pub fn run<A>() -> Result<(), ViewKitError>
where
    A: App,
{
    let app = A::new();
    let options = app.window();

    let runtime = ApplicationRuntime::new(app);

    #[cfg(target_os = "linux")]
    {
        use crate::platform::linux::LinuxBackend;

        let backend = LinuxBackend::new(
            runtime,
            WindowConfig {
                title: options.title().to_owned(),
                size: options.initial_size(),
                resizable: options.is_resizable(),
                fullscreen: options.is_fullscreen(),
                secure_overlay: options.is_secure_overlay(),
            },
        );

        backend.run()?;

        Ok(())
    }

    #[cfg(target_os = "mochios")]
    {
        use crate::platform::mochios::MochiOsBackend;

        let backend = MochiOsBackend::new(
            runtime,
            WindowConfig {
                title: options.title().to_owned(),
                size: options.initial_size(),
                resizable: options.is_resizable(),
                fullscreen: options.is_fullscreen(),
                secure_overlay: options.is_secure_overlay(),
            },
        );

        backend.run()?;

        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use crate::platform::windows::WindowsBackend;

        let backend = WindowsBackend::new(
            runtime,
            WindowConfig {
                title: options.title().to_owned(),
                size: options.initial_size(),
                resizable: options.is_resizable(),
                fullscreen: options.is_fullscreen(),
                secure_overlay: options.is_secure_overlay(),
            },
        );

        backend.run()?;

        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "mochios", target_os = "windows")))]
    {
        let _ = runtime;
        let _ = options;

        Err(ViewKitError::UnsupportedPlatform)
    }
}

/// ViewKitアプリケーションの起動中に発生するエラーです。
#[derive(Debug, thiserror::Error)]
pub enum ViewKitError {
    #[cfg(target_os = "linux")]
    #[error(transparent)]
    Linux(#[from] crate::platform::linux::LinuxBackendError),

    #[cfg(target_os = "mochios")]
    #[error(transparent)]
    MochiOs(#[from] crate::platform::mochios::MochiOsBackendError),

    #[cfg(target_os = "windows")]
    #[error(transparent)]
    Windows(#[from] crate::platform::windows::WindowsBackendError),

    #[error("ViewKit does not support the current platform")]
    UnsupportedPlatform,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::WindowOptions;
    use crate::geometry::Size;
    use crate::state::State;
    use crate::view::{Constraints, MeasureContext};
    use std::rc::Rc;

    #[test]
    fn state_change_expands_component_redraw_to_full_window() {
        let region = RedrawRequest::Region(Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(redraw_after_event(true, region), RedrawRequest::Full);
    }

    #[test]
    fn component_redraw_stays_regional_without_state_change() {
        let region = RedrawRequest::Region(Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(redraw_after_event(false, region), region);
    }

    #[test]
    fn application_exit_request_can_be_reset() {
        reset_exit_request();
        assert!(!exit_requested());
        request_exit();
        assert!(exit_requested());
        reset_exit_request();
        assert!(!exit_requested());
    }

    #[test]
    fn fallback_menu_clamps_to_the_window_and_ignores_disabled_items() {
        let request = ContextMenuRequest {
            request_id: 7,
            position: Point::new(90.0, 90.0),
            items: vec![
                crate::event::ContextMenuItem {
                    command_id: 1,
                    label: String::from("Enabled"),
                    enabled: true,
                    checked: false,
                    destructive: false,
                    separator: false,
                },
                crate::event::ContextMenuItem {
                    command_id: 2,
                    label: String::from("Disabled"),
                    enabled: false,
                    checked: false,
                    destructive: false,
                    separator: false,
                },
            ],
        };
        let viewport = Rect::new(0.0, 0.0, 200.0, 120.0);
        let bounds = fallback_menu_bounds(&request, viewport, &Theme::LIGHT);
        assert!(bounds.origin.x >= viewport.origin.x);
        assert!(bounds.origin.y >= viewport.origin.y);
        assert!(bounds.origin.x + bounds.size.width <= viewport.size.width);
        assert!(bounds.origin.y + bounds.size.height <= viewport.size.height);
        let rows = fallback_menu_rows(&request, viewport, &Theme::LIGHT);
        assert_eq!(
            fallback_menu_command_at(
                &request,
                Point::new(rows[0].1.origin.x + 1.0, rows[0].1.origin.y + 1.0),
                viewport,
                &Theme::LIGHT,
            ),
            Some(1)
        );
        assert_eq!(
            fallback_menu_command_at(
                &request,
                Point::new(rows[1].1.origin.x + 1.0, rows[1].1.origin.y + 1.0),
                viewport,
                &Theme::LIGHT,
            ),
            None
        );
    }

    #[test]
    fn state_changed_during_paint_rebuilds_before_the_next_draw() {
        let _ = take_state_changed();
        let builds = Rc::new(Cell::new(0));
        let state = State::new(false);
        let app = PaintMutationApp {
            state: state.clone(),
            builds: Rc::clone(&builds),
        };
        let mut runtime = ApplicationRuntime::new(app);
        let viewport = Viewport::new(Size::new(100.0, 100.0), 100, 100, 1.0);
        let mut display_list = DisplayList::default();

        let _ = runtime.draw(viewport, &mut display_list);
        assert!(state.get());
        assert_eq!(builds.get(), 1);

        let _ = runtime.draw(viewport, &mut display_list);
        assert_eq!(builds.get(), 2);
    }

    struct PaintMutationApp {
        state: State<bool>,
        builds: Rc<Cell<usize>>,
    }

    impl App for PaintMutationApp {
        type Body = PaintMutationView;

        fn new() -> Self {
            Self {
                state: State::new(false),
                builds: Rc::new(Cell::new(0)),
            }
        }

        fn window(&self) -> WindowOptions {
            WindowOptions::new("paint mutation test")
        }

        fn body(&self, _context: &ViewContext) -> Self::Body {
            self.builds.set(self.builds.get() + 1);
            PaintMutationView {
                state: self.state.clone(),
            }
        }
    }

    struct PaintMutationView {
        state: State<bool>,
    }

    impl View for PaintMutationView {
        fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
            constraints.constrain(Size::new(100.0, 100.0))
        }

        fn paint(&self, _bounds: Rect, context: &mut PaintContext<'_>) {
            if !self.state.get() {
                self.state.set(true);
                context.request_redraw_at(Instant::now());
            }
        }
    }
}
