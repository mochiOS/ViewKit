//! Process-isolated ViewKit applications inside the Linux Binder preview.
//!
//! This private, versioned host protocol carries the application's actual
//! display list and input events. The normal application runtime remains in
//! the child process, including dialogs, responders and independent windows.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use crate::app::WindowId;
use crate::draw_command::DisplayList;
use crate::geometry::Size;
use crate::platform::{
    CursorIcon, PlatformApplication, PlatformEvent, PlatformWindow, PlatformWindowCommand,
    WindowConfig,
};
use crate::renderer::Viewport;

pub const SOCKET_ENV: &str = "VIEWKIT_PREVIEW_SOCKET";
const MAGIC: &[u8; 4] = b"VKP1";
const MAX_PACKET: usize = 64 * 1024 * 1024;
const IO_BUDGET: usize = 4 * 1024 * 1024;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Packet {
    Open {
        id: u64,
        config: WindowConfig,
    },
    Close {
        id: u64,
    },
    CloseCancelled {
        id: u64,
    },
    Frame {
        id: u64,
        list: DisplayList,
        cursor: CursorIcon,
    },
    Title {
        id: u64,
        title: String,
    },
    Event {
        id: u64,
        event: PlatformEvent,
    },
    ReloadAppearance,
    ControlCenterCard {
        bundle_id: String,
        item_id: String,
        title: String,
        rows: Vec<(String, String)>,
    },
    UserNotification {
        id: u64,
        bundle_id: String,
        title: String,
        body: String,
    },
    FilePanelRequest {
        configuration: Vec<u8>,
        reply_socket: std::path::PathBuf,
    },
    FilePanelSelection {
        path: Option<std::path::PathBuf>,
    },
    FilePanelOperation {
        error: Option<String>,
    },
    OpenDocument {
        path: std::path::PathBuf,
        content_type: String,
        bundle_id: String,
        roles: u16,
    },
    DocumentOpened {
        process_id: Option<u32>,
        error: Option<String>,
    },
}

/// Opens an image through the authenticated host preview broker.
pub fn open_document(
    path: &std::path::Path,
    content_type: &str,
    bundle_id: &str,
    roles: u16,
) -> io::Result<u64> {
    let socket = std::env::var_os(SOCKET_ENV).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "The preview desktop is unavailable.",
        )
    })?;
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut connection = Connection::new(UnixStream::connect(socket)?)?;
    connection.send(Packet::OpenDocument {
        path,
        content_type: content_type.into(),
        bundle_id: bundle_id.into(),
        roles,
    })?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        for packet in connection.poll()? {
            if let Packet::DocumentOpened { process_id, error } = packet {
                return process_id
                    .filter(|pid| *pid != 0)
                    .map(u64::from)
                    .ok_or_else(|| {
                        io::Error::other(
                            error.unwrap_or_else(|| "The document could not be opened.".into()),
                        )
                    });
            }
            return Err(invalid("invalid document-open response"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "The preview desktop did not respond.",
    ))
}

thread_local! {
    static APP_MESSAGES: RefCell<VecDeque<Packet>> = const { RefCell::new(VecDeque::new()) };
}

/// Queues declarative application data for the preview shell, including registration before App::new finishes.
pub fn publish_control_center_card(
    bundle_id: String,
    item_id: String,
    title: String,
    rows: Vec<(String, String)>,
) -> io::Result<()> {
    APP_MESSAGES.with(|queue| {
        let mut queue = queue.borrow_mut();
        queue.retain(|packet| !matches!(packet, Packet::ControlCenterCard { bundle_id: old_bundle, item_id: old_item, .. } if old_bundle == &bundle_id && old_item == &item_id));
        if queue.iter().filter(|packet| matches!(packet, Packet::ControlCenterCard { .. })).count() >= 32 { return Err(io::Error::other("too many pending Control Center cards")); }
        queue.push_back(Packet::ControlCenterCard { bundle_id, item_id, title, rows });
        Ok(())
    })
}

/// Queues a session-local notification through the authenticated app connection.
/// The ID is unique across preview processes, including registration in App::new.
pub fn publish_notification(bundle_id: String, title: String, body: String) -> io::Result<u64> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT_ID: AtomicU32 = AtomicU32::new(1);
    APP_MESSAGES.with(|queue| {
        let mut queue = queue.borrow_mut();
        if queue
            .iter()
            .filter(|packet| matches!(packet, Packet::UserNotification { .. }))
            .count()
            >= 128
        {
            return Err(io::Error::other("too many pending notifications"));
        }
        let local = NEXT_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| io::Error::other("notification IDs exhausted"))?;
        let id = ((std::process::id() as u64) << 32) | u64::from(local);
        queue.push_back(Packet::UserNotification {
            id,
            bundle_id,
            title,
            body,
        });
        Ok(id)
    })
}

/// Nonblocking, bounded framing shared by the shell and application backend.
pub struct Connection {
    stream: UnixStream,
    input: Vec<u8>,
    output: VecDeque<(Vec<u8>, usize, Option<u64>)>,
}

impl Connection {
    pub fn new(stream: UnixStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            input: Vec::new(),
            output: VecDeque::new(),
        })
    }

    pub fn send(&mut self, packet: Packet) -> io::Result<()> {
        let frame_id = match &packet {
            Packet::Frame { id, .. } => Some(*id),
            _ => None,
        };
        let payload = postcard::to_stdvec(&packet).map_err(invalid)?;
        if payload.len() > MAX_PACKET {
            return Err(invalid("preview packet is too large"));
        }
        // A slow host needs only the latest complete frame for each window.
        if let Some(id) = frame_id {
            self.output
                .retain(|(_, offset, old)| *offset != 0 || *old != Some(id));
        }
        let pending: usize = self
            .output
            .iter()
            .map(|(bytes, offset, _)| bytes.len() - offset)
            .sum();
        if pending + payload.len() > MAX_PACKET * 2 {
            return Err(invalid("preview output queue is full"));
        }
        let mut bytes = Vec::with_capacity(payload.len() + 8);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        self.output.push_back((bytes, 0, frame_id));
        self.flush()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let mut budget = IO_BUDGET;
        while let Some((bytes, offset, _)) = self.output.front_mut() {
            if budget == 0 {
                break;
            }
            let end = bytes.len().min(*offset + budget);
            match self.stream.write(&bytes[*offset..end]) {
                Ok(0) => return Err(io::ErrorKind::BrokenPipe.into()),
                Ok(n) => {
                    *offset += n;
                    budget -= n;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
            if *offset == bytes.len() {
                self.output.pop_front();
            }
        }
        Ok(())
    }

    pub fn poll(&mut self) -> io::Result<Vec<Packet>> {
        self.flush()?;
        let mut budget = IO_BUDGET;
        let mut eof = false;
        while budget > 0 {
            let mut chunk = [0; 16384];
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => {
                    self.input.extend_from_slice(&chunk[..n]);
                    budget = budget.saturating_sub(n);
                    if self.input.len() > MAX_PACKET + IO_BUDGET + 8 {
                        return Err(invalid("preview input buffer is full"));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        let mut packets = Vec::new();
        let mut consumed = 0;
        while self.input.len() - consumed >= 8 {
            let header = &self.input[consumed..consumed + 8];
            if &header[..4] != MAGIC {
                return Err(invalid("invalid preview protocol version"));
            }
            let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
            if length > MAX_PACKET {
                return Err(invalid("preview packet is too large"));
            }
            if self.input.len() - consumed < length + 8 {
                break;
            }
            let payload = &self.input[consumed + 8..consumed + 8 + length];
            let (packet, rest) = postcard::take_from_bytes(payload).map_err(invalid)?;
            if !rest.is_empty() {
                return Err(invalid("trailing preview data"));
            }
            packets.push(packet);
            consumed += length + 8;
        }
        self.input.drain(..consumed);
        if eof && packets.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(packets)
    }
}

fn invalid(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

struct PreviewWindow {
    id: WindowId,
    viewport: Viewport,
    redraw: Cell<bool>,
    title: RefCell<String>,
    sent_title: String,
    cursor: Cell<CursorIcon>,
}

impl PreviewWindow {
    fn new(id: WindowId, config: &WindowConfig) -> Self {
        let width = config.size.width.round().clamp(1.0, 4096.0) as u32;
        let height = config.size.height.round().clamp(1.0, 4096.0) as u32;
        Self {
            id,
            viewport: viewport(width, height),
            redraw: Cell::new(true),
            title: RefCell::new(config.title.clone()),
            sent_title: config.title.clone(),
            cursor: Cell::new(CursorIcon::Default),
        }
    }
}

impl PlatformWindow for PreviewWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn viewport(&self) -> Viewport {
        self.viewport
    }
    fn request_redraw(&self) {
        self.redraw.set(true);
    }
    fn set_title(&self, title: &str) {
        *self.title.borrow_mut() = title.to_owned();
    }
    fn set_cursor(&self, cursor: CursorIcon) {
        self.cursor.set(cursor);
    }
}

pub fn viewport(width: u32, height: u32) -> Viewport {
    Viewport::new(Size::new(width as f32, height as f32), width, height, 1.0)
}

/// Runs the standard App runtime without creating independent host OS windows.
pub(crate) fn run<A: PlatformApplication>(
    mut app: A,
    config: WindowConfig,
    socket: &OsStr,
) -> io::Result<()> {
    let mut connection = Connection::new(UnixStream::connect(socket)?)?;
    let mut windows = BTreeMap::new();
    open(
        &mut app,
        &mut connection,
        &mut windows,
        WindowId::PRIMARY,
        config,
    )?;
    let mut first_frame = true;
    let mut appearance_check = Instant::now();
    loop {
        if appearance_check.elapsed() >= Duration::from_millis(200) {
            appearance_check = Instant::now();
            app.reload_appearance();
        }
        for packet in connection.poll()? {
            match packet {
                Packet::Event { id, event } => {
                    let Some(id) = WindowId::from_raw(id) else {
                        continue;
                    };
                    let Some(window) = windows.get_mut(&id) else {
                        continue;
                    };
                    if matches!(event, PlatformEvent::CloseRequested) {
                        if app.should_close_window(window) {
                            windows.remove(&id);
                            connection.send(Packet::Close { id: id.raw() })?;
                        } else {
                            connection.send(Packet::CloseCancelled { id: id.raw() })?;
                        }
                    } else {
                        if let PlatformEvent::Resized { viewport } = &event {
                            window.viewport = *viewport;
                            window.request_redraw();
                        }
                        app.handle_event(event, window);
                    }
                }
                Packet::ReloadAppearance => {
                    if app.reload_appearance() {
                        for window in windows.values() {
                            window.request_redraw();
                        }
                    }
                }
                _ => return Err(invalid("unexpected packet from preview shell")),
            }
        }
        for command in app.take_window_commands() {
            match command {
                PlatformWindowCommand::Open { id, config } => {
                    open(&mut app, &mut connection, &mut windows, id, config)?;
                }
                PlatformWindowCommand::Close { id } => {
                    if windows.remove(&id).is_some() {
                        connection.send(Packet::Close { id: id.raw() })?;
                    }
                }
                PlatformWindowCommand::RequestClose { id } => {
                    if let Some(window) = windows.get(&id) {
                        if app.should_close_window(window) {
                            windows.remove(&id);
                            connection.send(Packet::Close { id: id.raw() })?;
                        }
                    }
                }
                PlatformWindowCommand::Redraw { id } => {
                    if let Some(window) = windows.get(&id) {
                        window.request_redraw();
                    }
                }
            }
        }
        let now = Instant::now();
        for (&id, window) in &mut windows {
            if app
                .next_redraw_at(id)
                .is_some_and(|deadline| deadline <= now)
            {
                window.request_redraw();
            }
            if window.redraw.replace(false) {
                let mut list = DisplayList::new();
                app.handle_event(PlatformEvent::RedrawRequested, window);
                let dirty = app.draw(window, &mut list);
                if !dirty.is_empty() {
                    connection.send(Packet::Frame {
                        id: id.raw(),
                        list,
                        cursor: window.cursor.get(),
                    })?;
                    if first_frame {
                        app.first_frame_presented();
                        first_frame = false;
                    }
                }
            }
            if *window.title.borrow() != window.sent_title {
                window.sent_title = window.title.borrow().clone();
                connection.send(Packet::Title {
                    id: id.raw(),
                    title: window.sent_title.clone(),
                })?;
            }
        }
        // Delivery from event handlers must be flushed before final window
        // close/exit as well as during normal frames.
        let messages = APP_MESSAGES.with(|queue| std::mem::take(&mut *queue.borrow_mut()));
        for packet in messages {
            connection.send(packet)?;
        }
        connection.flush()?;
        if windows.is_empty() || app.exit_requested() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(8));
    }
}

fn open<A: PlatformApplication>(
    app: &mut A,
    connection: &mut Connection,
    windows: &mut BTreeMap<WindowId, PreviewWindow>,
    id: WindowId,
    config: WindowConfig,
) -> io::Result<()> {
    if windows.contains_key(&id) {
        return Ok(());
    }
    if windows.len() >= 64 {
        return Err(invalid("too many preview windows"));
    }
    let window = PreviewWindow::new(id, &config);
    connection.send(Packet::Open {
        id: id.raw(),
        config,
    })?;
    app.handle_event(
        PlatformEvent::Resumed {
            viewport: window.viewport,
        },
        &window,
    );
    windows.insert(id, window);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DrawCommand;
    use crate::geometry::Rect;
    use crate::theme::Color;

    #[test]
    fn display_list_and_unicode_input_cross_process_transport() {
        let (a, b) = UnixStream::pair().unwrap();
        let mut a = Connection::new(a).unwrap();
        let mut b = Connection::new(b).unwrap();
        let mut list = DisplayList::new();
        list.push(DrawCommand::FillRoundedRect {
            rect: Rect::new(1.0, 2.0, 30.0, 40.0),
            radius: 6.0,
            color: Color::WHITE,
        });
        list.push(DrawCommand::DrawContour {
            points: vec![
                crate::geometry::Point::new(0.0, 0.0),
                crate::geometry::Point::new(30.0, 0.0),
                crate::geometry::Point::new(15.0, 10.0),
            ],
            fill: Color::WHITE,
            stroke: Color::BLACK,
            stroke_width: 1.0,
        });
        a.send(Packet::Frame {
            id: 3,
            list: list.clone(),
            cursor: CursorIcon::Text,
        })
        .unwrap();
        let packets = b.poll().unwrap();
        assert!(
            matches!(&packets[0], Packet::Frame { id: 3, list: decoded, .. } if decoded.commands() == list.commands())
        );
        b.send(Packet::Event {
            id: 3,
            event: PlatformEvent::TextInput {
                text: "もちOS".into(),
            },
        })
        .unwrap();
        assert!(
            matches!(&a.poll().unwrap()[0], Packet::Event { event: PlatformEvent::TextInput { text }, .. } if text == "もちOS")
        );
    }

    #[test]
    fn rejects_unknown_version_and_oversized_length_before_payload() {
        for header in [
            *b"BAD!\0\0\0\0",
            [b'V', b'K', b'P', b'1', 255, 255, 255, 255],
        ] {
            let (mut a, b) = UnixStream::pair().unwrap();
            let mut b = Connection::new(b).unwrap();
            a.write_all(&header).unwrap();
            assert_eq!(b.poll().unwrap_err().kind(), io::ErrorKind::InvalidData);
        }
    }

    #[test]
    fn notification_published_on_final_close_is_delivered_before_exit() {
        use std::os::unix::net::UnixListener;
        struct Probe;
        impl PlatformApplication for Probe {
            fn handle_event(&mut self, event: PlatformEvent, _: &dyn PlatformWindow) {
                if matches!(event, PlatformEvent::CloseRequested) {
                    publish_notification(
                        "org.example.app".into(),
                        "完了".into(),
                        "保存しました".into(),
                    )
                    .unwrap();
                }
            }
        }
        let directory =
            std::env::temp_dir().join(format!("viewkit-notification-exit-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let socket = directory.join("preview.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let child_socket = socket.clone();
        let child = std::thread::spawn(move || {
            run(Probe, WindowConfig::default(), child_socket.as_os_str()).unwrap()
        });
        let (stream, _) = listener.accept().unwrap();
        let mut server = Connection::new(stream).unwrap();
        server
            .send(Packet::Event {
                id: 1,
                event: PlatformEvent::CloseRequested,
            })
            .unwrap();
        child.join().unwrap();
        let packets = server.poll().unwrap();
        assert!(packets.iter().any(|packet| matches!(packet, Packet::UserNotification { id, bundle_id, title, body }
            if id >> 32 == std::process::id() as u64 && bundle_id == "org.example.app" && title == "完了" && body == "保存しました")));
        drop(listener);
        std::fs::remove_file(socket).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn backend_runs_real_window_lifecycle_without_a_host_display() {
        use crate::platform::{Key, KeyModifiers};
        use std::os::unix::net::UnixListener;

        struct Probe {
            commands: Vec<PlatformWindowCommand>,
            can_close: bool,
        }
        impl PlatformApplication for Probe {
            fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
                if let PlatformEvent::KeyPressed {
                    key: Key::Character('n'),
                    ..
                } = event
                {
                    self.commands.push(PlatformWindowCommand::Open {
                        id: WindowId::from_raw(2).unwrap(),
                        config: WindowConfig::default(),
                    });
                    self.can_close = true;
                }
                window.request_redraw();
            }
            fn should_close_window(&mut self, _: &dyn PlatformWindow) -> bool {
                self.can_close
            }
            fn take_window_commands(&mut self) -> Vec<PlatformWindowCommand> {
                std::mem::take(&mut self.commands)
            }
            fn draw(&mut self, window: &dyn PlatformWindow, list: &mut DisplayList) -> Rect {
                list.push(DrawCommand::FillRect {
                    rect: window.viewport().logical_bounds(),
                    color: Color::WHITE,
                });
                window.viewport().logical_bounds()
            }
        }
        fn receive(connection: &mut Connection, expected: impl Fn(&Packet) -> bool) -> Packet {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                for packet in connection.poll().unwrap() {
                    if expected(&packet) {
                        return packet;
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            panic!("preview backend did not produce the expected packet");
        }

        let directory =
            std::env::temp_dir().join(format!("viewkit-preview-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let socket = directory.join("test.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let client_socket = socket.clone();
        let child = std::thread::spawn(move || {
            run(
                Probe {
                    commands: Vec::new(),
                    can_close: false,
                },
                WindowConfig::default(),
                client_socket.as_os_str(),
            )
            .unwrap();
        });
        let (stream, _) = listener.accept().unwrap();
        let mut server = Connection::new(stream).unwrap();
        receive(&mut server, |packet| {
            matches!(packet, Packet::Open { id: 1, .. })
        });
        server
            .send(Packet::Event {
                id: 1,
                event: PlatformEvent::CloseRequested,
            })
            .unwrap();
        receive(&mut server, |packet| {
            matches!(packet, Packet::CloseCancelled { id: 1 })
        });
        server
            .send(Packet::Event {
                id: 1,
                event: PlatformEvent::Resized {
                    viewport: viewport(321, 234),
                },
            })
            .unwrap();
        let packet = receive(&mut server, |packet| {
            matches!(packet, Packet::Frame { list, .. }
            if matches!(list.commands().first(), Some(DrawCommand::FillRect { rect, .. }) if rect.size == Size::new(321.0, 234.0)))
        });
        assert!(matches!(packet, Packet::Frame { id: 1, .. }));
        server
            .send(Packet::Event {
                id: 1,
                event: PlatformEvent::KeyPressed {
                    key: Key::Character('n'),
                    modifiers: KeyModifiers::default(),
                },
            })
            .unwrap();
        receive(&mut server, |packet| {
            matches!(packet, Packet::Open { id: 2, .. })
        });
        server
            .send(Packet::Event {
                id: 2,
                event: PlatformEvent::CloseRequested,
            })
            .unwrap();
        receive(&mut server, |packet| {
            matches!(packet, Packet::Close { id: 2 })
        });
        server
            .send(Packet::Event {
                id: 1,
                event: PlatformEvent::CloseRequested,
            })
            .unwrap();
        child.join().unwrap();
        drop(listener);
        std::fs::remove_file(socket).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
