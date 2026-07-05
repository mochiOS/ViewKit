// TODO:
// 現在のContextMenuは、親Viewと同じViewツリー内へMenuを重ねて描画し、
// 右クリック位置の補正、外側クリックによる閉鎖、入力の捕捉も自身で処理している。
//
// 正式実装では、Menu/MenuItemの構築・見た目・アクションはViewKitに残し、
// ポップアップSurfaceの生成、画面端への位置補正、フォーカスおよびポインターの捕捉、
// 外側クリックによる閉鎖はPlatformWindow経由でwindow service / compositorへ委譲する。
//
// 想定API:
// PlatformWindow::show_popup(PopupRequest)
//
// ContextMenuは最終的に、メニューを直接描画するコンポーネントではなく、
// platform層へポップアップ表示を要求する宣言的なViewラッパーに変更する。

use std::cell::Cell;

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::platform::PointerButton;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

pub struct ContextMenu {
    content: StackChild,
    menu: StackChild,
    position: Cell<Option<Point>>,
}

impl ContextMenu {
    pub fn new<C, M>(content: C, menu: M) -> Self
    where
        C: IntoStackChild,
        M: IntoStackChild,
    {
        Self {
            content: content.into_stack_child(),
            menu: menu.into_stack_child(),
            position: Cell::new(None),
        }
    }

    pub fn is_open(&self) -> bool {
        self.position.get().is_some()
    }

    pub fn close(&self) {
        self.position.set(None);
    }

    fn positioned_menu_bounds(bounds: Rect, position: Point, menu_size: Size) -> Rect {
        let maximum_x =
            (bounds.origin.x + bounds.size.width - menu_size.width).max(bounds.origin.x);

        let maximum_y =
            (bounds.origin.y + bounds.size.height - menu_size.height).max(bounds.origin.y);

        Rect::new(
            position.x.clamp(bounds.origin.x, maximum_x),
            position.y.clamp(bounds.origin.y, maximum_y),
            menu_size.width,
            menu_size.height,
        )
    }

    fn menu_bounds_for_paint(
        &self,
        bounds: Rect,
        position: Point,
        context: &mut PaintContext<'_>,
    ) -> Rect {
        let menu_size = {
            let mut measure_context = MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            };

            self.menu
                .measure(Constraints::loose(bounds.size), &mut measure_context)
        };

        Self::positioned_menu_bounds(bounds, position, menu_size)
    }

    fn menu_bounds_for_event(
        &self,
        bounds: Rect,
        position: Point,
        context: &mut EventContext<'_>,
    ) -> Rect {
        let menu_size = {
            let theme = context.theme;
            let typography = context.typography;
            let text_measurer = &mut *context.text_measurer;

            let mut measure_context = MeasureContext {
                theme,
                typography,
                text_measurer,
            };

            self.menu
                .measure(Constraints::loose(bounds.size), &mut measure_context)
        };

        Self::positioned_menu_bounds(bounds, position, menu_size)
    }

    fn open_at(
        &self,
        bounds: Rect,
        position: Point,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !bounds.contains(position) {
            return EventResult::Ignored;
        }

        self.content
            .handle_event(bounds, &ViewEvent::PointerLeft, context);

        self.position.set(Some(position));
        context.request_redraw();

        EventResult::Consumed
    }
}

impl View for ContextMenu {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.content.paint(bounds, context);

        let Some(position) = self.position.get() else {
            return;
        };

        let menu_bounds = self.menu_bounds_for_paint(bounds, position, context);

        self.menu.paint(menu_bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let Some(anchor) = self.position.get() else {
            if let ViewEvent::PointerPressed {
                position,
                button: PointerButton::Secondary,
            } = event
            {
                return self.open_at(bounds, *position, context);
            }

            return self.content.handle_event(bounds, event, context);
        };

        let menu_bounds = self.menu_bounds_for_event(bounds, anchor, context);

        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Secondary,
            } => {
                self.menu
                    .handle_event(menu_bounds, &ViewEvent::PointerLeft, context);

                self.position.set(Some(*position));
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !menu_bounds.contains(*position) {
                    self.menu
                        .handle_event(menu_bounds, &ViewEvent::PointerLeft, context);

                    self.close();
                    context.request_redraw();

                    return EventResult::Consumed;
                }

                self.menu.handle_event(menu_bounds, event, context);

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let result = self.menu.handle_event(menu_bounds, event, context);

                if result.is_consumed() {
                    self.close();
                    context.request_redraw();
                }

                if menu_bounds.contains(*position) {
                    EventResult::Consumed
                } else {
                    result
                }
            }

            ViewEvent::PointerMoved { .. } => self.menu.handle_event(menu_bounds, event, context),

            ViewEvent::PointerLeft => self.menu.handle_event(menu_bounds, event, context),

            ViewEvent::FocusChanged { focused: false } => {
                self.menu
                    .handle_event(menu_bounds, &ViewEvent::PointerLeft, context);

                self.close();
                context.request_redraw();

                EventResult::Consumed
            }

            ViewEvent::PointerFocusRequested { .. } => EventResult::Consumed,

            _ => self.menu.handle_event(menu_bounds, event, context),
        }
    }
}
