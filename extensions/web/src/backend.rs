// Engine-adapter boundary, deliberately private. No fake implementation ships.
use crate::{NavigationId, WebError, WebSessionOptions};
use std::{
    collections::VecDeque,
    rc::Rc,
    sync::{Arc, Mutex},
};
use viewkit::{dispatch::UiNotifier, view::View};

pub(crate) const EVENT_BUDGET: usize = 64;
const QUEUE_LIMIT: usize = 256;
const QUEUE_TEXT_LIMIT: usize = 4 * 1024 * 1024;
#[derive(Clone)]
pub(crate) struct EventSink {
    pub queue: Arc<Mutex<Inbox>>,
    pub wake: UiNotifier,
}
#[derive(Default)]
pub(crate) struct Inbox {
    pub events: VecDeque<(u64, Event)>,
    pub overflow: bool,
    pub text_bytes: usize,
}
impl EventSink {
    #[allow(dead_code)] // Reserved for the native adapter; test adapter exercises it.
    pub fn push(&self, page: u64, event: Event) -> bool {
        let accepted = {
            let mut inbox = self.queue.lock().unwrap_or_else(|p| p.into_inner());
            if inbox.overflow {
                false
            } else if inbox.events.len() == QUEUE_LIMIT
                || event.text_bytes() > QUEUE_TEXT_LIMIT.saturating_sub(inbox.text_bytes)
            {
                inbox.overflow = true;
                inbox.events.clear();
                inbox.text_bytes = 0;
                false
            } else {
                inbox.text_bytes += event.text_bytes();
                inbox.events.push_back((page, event));
                true
            }
        };
        self.wake.notify() && accepted
    }
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Started(NavigationId),
    Committed {
        navigation: NavigationId,
        url: String,
        title: String,
        back: bool,
        forward: bool,
    },
    Finished(NavigationId),
    Failed(NavigationId, WebError),
    Stopped(NavigationId),
    Title(NavigationId, String),
    ProcessExited,
}
impl Event {
    pub fn text_bytes(&self) -> usize {
        match self {
            Self::Committed { url, title, .. } => url.len().saturating_add(title.len()),
            Self::Title(_, text) | Self::Failed(_, WebError::Backend(text)) => text.len(),
            _ => 0,
        }
    }
}
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Command {
    Load(NavigationId, String),
    Back(NavigationId),
    Forward(NavigationId),
    Reload(NavigationId),
    Stop(NavigationId),
    Zoom(f64),
}
pub(crate) trait SessionDriver {
    fn create_page(&self, id: u64) -> Result<Rc<dyn PageDriver>, WebError>;
    // Bounded, nonblocking work on the UI loop, never in paint/measure.
    // True requests another dispatch turn. Worker readiness also signals sink.wake.
    fn dispatch(&self, budget: usize) -> bool;
}
pub(crate) trait PageDriver {
    // Must copy/enqueue only; never invoke an application callback inline.
    fn enqueue(&self, command: Command) -> Result<(), WebError>;
    // Only a completed visual. No engine work in paint/measure.
    fn view(&self) -> Rc<dyn View>;
    fn close(&self);
}
pub(crate) fn connect(
    _options: &WebSessionOptions,
    _sink: EventSink,
) -> Result<Rc<dyn SessionDriver>, WebError> {
    // The C ABI/native WPE adapter is not implemented. Do not silently substitute
    // a host engine, disable process isolation, or create a successful blank page.
    Err(WebError::Unavailable)
}
