use super::*;
use backend::{Command, Event, EventSink, PageDriver, SessionDriver};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use viewkit::{
    draw_command::DisplayList,
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    theme::Theme,
    typography::TextMeasurer,
    view::{Constraints, MeasureContext, PaintContext, View},
};

#[derive(Default)]
struct Visual {
    paints: Cell<u32>,
    measures: Cell<u32>,
    events: Cell<u32>,
}
impl View for Visual {
    fn measure(&self, c: Constraints, _: &mut MeasureContext<'_>) -> Size {
        self.measures.set(self.measures.get() + 1);
        c.maximum
    }
    fn paint(&self, _: Rect, _: &mut PaintContext<'_>) {
        self.paints.set(self.paints.get() + 1);
    }
    fn handle_event(&self, _: Rect, _: &ViewEvent, _: &mut EventContext<'_>) -> EventResult {
        self.events.set(self.events.get() + 1);
        EventResult::Consumed
    }
}
#[derive(Default)]
struct DriverPage {
    commands: RefCell<Vec<Command>>,
    reject: RefCell<Option<WebError>>,
    closes: Cell<u32>,
    visual: Rc<Visual>,
}
impl PageDriver for DriverPage {
    fn enqueue(&self, c: Command) -> Result<(), WebError> {
        if let Some(error) = self.reject.borrow().clone() {
            return Err(error);
        }
        self.commands.borrow_mut().push(c);
        Ok(())
    }
    fn view(&self) -> Rc<dyn View> {
        self.visual.clone()
    }
    fn close(&self) {
        self.closes.set(self.closes.get() + 1);
    }
}
struct Driver {
    sink: EventSink,
    pages: RefCell<Vec<(u64, Rc<DriverPage>)>>,
    dispatches: Cell<u32>,
    again: Cell<bool>,
}
impl SessionDriver for Driver {
    fn create_page(&self, id: u64) -> Result<Rc<dyn PageDriver>, WebError> {
        let page = Rc::new(DriverPage::default());
        self.pages.borrow_mut().push((id, page.clone()));
        Ok(page)
    }
    fn dispatch(&self, budget: usize) -> bool {
        assert_eq!(budget, backend::EVENT_BUDGET);
        self.dispatches.set(self.dispatches.get() + 1);
        self.again.replace(false)
    }
}
fn fixture() -> (WebSession, WebPage, Rc<Driver>, Rc<DriverPage>) {
    let slot = Rc::new(RefCell::new(None));
    let capture = slot.clone();
    let session = WebSession::start(WebSessionOptions::default(), move |_, sink| {
        let driver = Rc::new(Driver {
            sink,
            pages: RefCell::default(),
            dispatches: Cell::new(0),
            again: Cell::new(false),
        });
        *capture.borrow_mut() = Some(driver.clone());
        Ok(driver)
    })
    .unwrap();
    let page = session.create_page().unwrap();
    let driver = slot.borrow_mut().take().unwrap();
    let native = driver.pages.borrow()[0].1.clone();
    (session, page, driver, native)
}
fn send(driver: &Driver, event: Event) {
    assert!(driver.sink.push(driver.pages.borrow()[0].0, event));
}
fn committed(id: NavigationId, url: &str, back: bool) -> Event {
    Event::Committed {
        navigation: id,
        url: url.into(),
        title: "Document".into(),
        back,
        forward: false,
    }
}
#[test]
fn product_constructor_reports_unavailable_instead_of_a_fake_engine() {
    assert!(matches!(
        WebSession::new(Default::default()),
        Err(WebError::Unavailable)
    ));
}
#[test]
fn acceptance_is_not_commit_or_finish_and_notifications_are_deferred() {
    let (session, page, driver, _) = fixture();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let _token = page.subscribe(move |e| seen.borrow_mut().push(e)).unwrap();
    let id = page.load_url("https://example.com").unwrap();
    assert_eq!(page.state().load_state, WebLoadState::Requested);
    assert_eq!(page.state().url, None);
    assert!(events.borrow().is_empty());
    send(&driver, Event::Started(id));
    send(&driver, Event::Finished(id));
    assert!(events.borrow().is_empty());
    session.inner.dispatch();
    assert_eq!(page.state().load_state, WebLoadState::Loading); // Finished without commit is invalid.
    send(
        &driver,
        committed(id, "https://example.com/redirected", true),
    );
    send(&driver, Event::Finished(id));
    session.inner.dispatch();
    assert_eq!(
        page.state().url.as_deref(),
        Some("https://example.com/redirected")
    );
    assert_eq!(page.state().load_state, WebLoadState::Ready);
    assert!(page.state().can_go_back);
    assert_eq!(events.borrow().len(), 2);
}
#[test]
fn stale_navigation_cannot_replace_new_page_state_or_history() {
    let (session, page, driver, _) = fixture();
    let old = page.load_url("https://old.test").unwrap();
    send(&driver, committed(old, "https://old.test/", true));
    send(&driver, Event::Finished(old));
    session.inner.dispatch();
    let new = page.load_url("https://new.test").unwrap();
    assert_ne!(old, new);
    let pending = page.state();
    send(&driver, Event::Title(old, "Wrong title".into()));
    send(
        &driver,
        Event::Failed(old, WebError::Backend("late failure".into())),
    );
    send(&driver, committed(old, "https://wrong.test/", false));
    send(&driver, Event::Finished(old));
    session.inner.dispatch();
    assert_eq!(page.state(), pending);
    send(&driver, committed(new, "https://new.test/", false));
    send(&driver, Event::Finished(new));
    session.inner.dispatch();
    assert_eq!(page.state().url.as_deref(), Some("https://new.test/"));
    assert!(!page.state().can_go_back);
}
#[test]
fn rejected_command_leaves_state_unchanged_and_does_not_reuse_id() {
    let (_, page, _, native) = fixture();
    *native.reject.borrow_mut() = Some(WebError::Unsupported);
    assert_eq!(
        page.load_url("https://example.com"),
        Err(WebError::Unsupported)
    );
    assert_eq!(page.state(), WebPageState::default());
    *native.reject.borrow_mut() = None;
    let accepted = page.load_url("https://example.com").unwrap();
    assert_eq!(accepted.get(), 2);
    assert_eq!(native.commands.borrow().len(), 1);
}
#[test]
fn invalid_urls_zoom_and_unavailable_history_never_reach_backend() {
    let (_, page, _, native) = fixture();
    assert_eq!(page.load_url("search terms"), Err(WebError::InvalidUrl));
    assert_eq!(page.go_back(), Err(WebError::ActionUnavailable));
    assert_eq!(page.go_forward(), Err(WebError::ActionUnavailable));
    assert_eq!(page.reload(), Err(WebError::ActionUnavailable));
    assert_eq!(page.stop(), Err(WebError::ActionUnavailable));
    for zoom in [0., -1., f64::NAN, f64::INFINITY] {
        assert_eq!(page.set_zoom(zoom), Err(WebError::InvalidZoom));
    }
    assert!(native.commands.borrow().is_empty());
    page.set_zoom(1.25).unwrap();
    assert_eq!(*native.commands.borrow(), vec![Command::Zoom(1.25)]);
}
#[test]
fn stop_waits_for_engine_confirmation_and_late_finish_cannot_restart_it() {
    let (session, page, driver, native) = fixture();
    let id = page.load_url("https://example.com").unwrap();
    page.stop().unwrap();
    assert_eq!(page.state().load_state, WebLoadState::Requested);
    assert_eq!(native.commands.borrow()[1], Command::Stop(id));
    send(&driver, Event::Stopped(id));
    send(&driver, committed(id, "https://example.com/", true));
    send(&driver, Event::Finished(id));
    session.inner.dispatch();
    assert_eq!(page.state().load_state, WebLoadState::Stopped);
    assert_eq!(page.state().url, None);
}
#[test]
fn callback_can_navigate_and_unsubscribe_without_a_refcell_panic() {
    let (session, page, _, _) = fixture();
    let token_slot = Rc::new(RefCell::new(None));
    let slot = token_slot.clone();
    let target = page.clone();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    *token_slot.borrow_mut() = Some(
        page.subscribe(move |_| {
            seen.set(seen.get() + 1);
            slot.borrow_mut().take();
            target.load_url("https://next.test").unwrap();
        })
        .unwrap(),
    );
    page.load_url("https://first.test").unwrap();
    session.inner.dispatch();
    session.inner.dispatch();
    assert_eq!(count.get(), 1);
    assert_eq!(page.state().navigation.unwrap().get(), 2);
}
#[test]
fn unsubscribe_during_delivery_cancels_a_later_subscriber_and_no_ownership_cycle_remains() {
    let (session, page, _, _) = fixture();
    let weak = Rc::downgrade(&page.inner);
    let slot = Rc::new(RefCell::new(None));
    let captured = slot.clone();
    let token = page
        .subscribe(move |_| {
            captured.borrow_mut().take();
        })
        .unwrap();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let held = page.clone();
    *slot.borrow_mut() = Some(
        page.subscribe(move |_| {
            let _ = held.state();
            seen.set(seen.get() + 1);
        })
        .unwrap(),
    );
    page.load_url("https://example.com").unwrap();
    session.inner.dispatch();
    assert_eq!(count.get(), 0);
    drop(token);
    drop(page);
    assert!(weak.upgrade().is_none());
}
#[test]
fn closing_is_idempotent_rejects_commands_and_discards_queued_events() {
    let (session, page, driver, native) = fixture();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let _token = page.subscribe(move |_| seen.set(seen.get() + 1)).unwrap();
    let id = page.load_url("https://example.com").unwrap();
    send(&driver, committed(id, "https://example.com/", true));
    page.close();
    page.close();
    session.inner.dispatch();
    assert_eq!(count.get(), 0);
    assert_eq!(native.closes.get(), 1);
    assert_eq!(page.state().load_state, WebLoadState::Closed);
    assert_eq!(page.load_url("https://example.com"), Err(WebError::Closed));
    drop(page);
    assert_eq!(native.closes.get(), 1);
}
#[test]
fn page_and_view_keep_session_alive_and_clones_do_not_recreate_presentation() {
    let (session, page, _, native) = fixture();
    let weak = Rc::downgrade(&session.inner);
    drop(session);
    let view = WebView::new(page.clone()).unwrap();
    let retained = view.clone();
    assert!(matches!(
        WebView::new(page.clone()),
        Err(WebError::AlreadyPresented)
    ));
    drop(view);
    assert!(matches!(
        WebView::new(page.clone()),
        Err(WebError::AlreadyPresented)
    ));
    drop(retained);
    let final_view = WebView::new(page.clone()).unwrap();
    drop(page);
    assert!(weak.upgrade().is_some());
    assert_eq!(native.closes.get(), 0);
    drop(final_view);
    assert!(weak.upgrade().is_none());
    assert_eq!(native.closes.get(), 1);
}
#[test]
fn process_exit_is_terminal_and_does_not_complete_the_load() {
    let (session, page, driver, native) = fixture();
    let id = page.load_url("https://example.com").unwrap();
    send(&driver, Event::ProcessExited);
    send(&driver, committed(id, "https://example.com/", false));
    send(&driver, Event::Finished(id));
    session.inner.dispatch();
    assert_eq!(page.state().load_state, WebLoadState::Terminated);
    assert_eq!(page.state().url, None);
    assert_eq!(native.closes.get(), 1);
    assert_eq!(page.reload(), Err(WebError::ProcessExited));
}
#[test]
fn bounded_dispatch_and_overflow_are_explicit_and_do_not_silently_drop_success() {
    let (session, page, driver, native) = fixture();
    let id = page.load_url("https://example.com").unwrap();
    for _ in 0..100 {
        send(&driver, Event::Started(id));
    }
    session.inner.dispatch();
    assert_eq!(driver.sink.queue.lock().unwrap().events.len(), 36);
    session.inner.dispatch();
    assert!(driver.sink.queue.lock().unwrap().events.is_empty());
    for _ in 0..256 {
        assert!(driver.sink.push(1, Event::Started(id)));
    }
    assert!(!driver.sink.push(1, Event::Finished(id)));
    session.inner.dispatch();
    assert_eq!(
        page.state().load_state,
        WebLoadState::Failed(WebError::EventQueueOverflow)
    );
    assert_eq!(native.closes.get(), 1);
    assert!(matches!(
        session.create_page(),
        Err(WebError::EventQueueOverflow)
    ));
    assert_eq!(
        page.load_url("https://example.com"),
        Err(WebError::EventQueueOverflow)
    );
}
#[test]
fn worker_events_do_not_run_callbacks_and_ui_state_is_not_sendable() {
    let (session, page, driver, _) = fixture();
    let id = page.load_url("https://example.com").unwrap();
    let seen = Rc::new(Cell::new(0));
    let count = seen.clone();
    let thread = std::thread::current().id();
    let _token = page
        .subscribe(move |_| {
            assert_eq!(std::thread::current().id(), thread);
            count.set(count.get() + 1);
        })
        .unwrap();
    let sink = driver.sink.clone();
    std::thread::spawn(move || {
        assert!(sink.push(1, Event::Started(id)));
    })
    .join()
    .unwrap();
    assert_eq!(seen.get(), 0);
    session.inner.dispatch();
    assert_eq!(seen.get(), 1);
}
#[test]
fn view_delegates_without_dispatching_loading_or_adding_decoration_and_closed_view_stops_input() {
    let (_, page, driver, native) = fixture();
    let view = WebView::new(page.clone()).unwrap();
    let theme = Theme::LIGHT;
    let mut text = TextMeasurer::new();
    let mut list = DisplayList::new();
    let bounds = Rect::new(0., 0., 320., 240.);
    view.measure(
        Constraints::loose(bounds.size),
        &mut MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut text,
        },
    );
    view.paint(
        bounds,
        &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
    );
    let mut context = EventContext::new(&theme, &theme.typography, &mut text);
    assert_eq!(
        view.handle_event(bounds, &ViewEvent::PointerLeft, &mut context),
        EventResult::Consumed
    );
    assert_eq!(native.visual.paints.get(), 1);
    assert_eq!(native.visual.events.get(), 1);
    assert_eq!(driver.dispatches.get(), 0);
    assert!(native.commands.borrow().is_empty());
    assert!(list.commands().is_empty());
    page.close();
    assert_eq!(
        view.handle_event(bounds, &ViewEvent::PointerLeft, &mut context),
        EventResult::Ignored
    );
    view.paint(
        bounds,
        &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text),
    );
    assert_eq!(native.visual.paints.get(), 1);
    assert_eq!(native.visual.events.get(), 1);
}
#[test]
fn sessions_and_pages_have_separate_state_and_unknown_pages_are_ignored() {
    let (session, first, driver, _) = fixture();
    let second = session.create_page().unwrap();
    let id = first.load_url("https://example.com").unwrap();
    assert!(
        driver
            .sink
            .push(u64::MAX, committed(id, "https://wrong.test/", true))
    );
    send(&driver, committed(id, "https://example.com/", true));
    session.inner.dispatch();
    assert_eq!(second.state(), WebPageState::default());
    assert_eq!(first.state().url.as_deref(), Some("https://example.com/"));
    let (other_session, other, _, _) = fixture();
    assert_eq!(other.state(), WebPageState::default());
    assert!(!Rc::ptr_eq(&session.inner, &other_session.inner));
}

#[test]
fn malformed_commits_fail_instead_of_leaving_a_success_or_an_endless_load() {
    let (session, page, driver, _) = fixture();
    let id = page.load_url("https://example.com").unwrap();
    send(&driver, committed(id, "not a URL", true));
    send(&driver, Event::Finished(id));
    session.inner.dispatch();
    assert!(matches!(page.state().load_state, WebLoadState::Failed(_)));
    assert_eq!(page.state().url, None);
    assert!(!page.state().can_go_back);
}
#[test]
fn queue_text_budget_is_reclaimed_and_large_payloads_fail_closed() {
    let (session, page, driver, _) = fixture();
    let id = page.load_url("https://example.com").unwrap();
    send(&driver, Event::Title(id, "discarded before commit".into()));
    assert!(driver.sink.queue.lock().unwrap().text_bytes > 0);
    session.inner.dispatch();
    assert_eq!(driver.sink.queue.lock().unwrap().text_bytes, 0);
    assert!(
        !driver
            .sink
            .push(1, Event::Title(id, "x".repeat(4 * 1024 * 1024 + 1)))
    );
    session.inner.dispatch();
    assert_eq!(
        page.state().load_state,
        WebLoadState::Failed(WebError::EventQueueOverflow)
    );
}

#[test]
fn constructor_failure_does_not_leak_ui_dispatch_subscriptions() {
    for _ in 0..300 {
        assert!(matches!(
            WebSession::new(Default::default()),
            Err(WebError::Unavailable)
        ));
    }
}
#[test]
fn closing_from_a_callback_cancels_the_remaining_delivery() {
    let (session, page, _, native) = fixture();
    let target = page.clone();
    let _first = page.subscribe(move |_| target.close()).unwrap();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let _second = page.subscribe(move |_| seen.set(seen.get() + 1)).unwrap();
    page.load_url("https://example.com").unwrap();
    session.inner.dispatch();
    assert_eq!(count.get(), 0);
    assert_eq!(native.closes.get(), 1);
    assert_eq!(page.state().load_state, WebLoadState::Closed);
}
