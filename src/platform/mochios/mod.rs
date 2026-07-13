use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::Instant;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use mochi_user_syscall as syscall;
use tiny_skia::{
    Color as SkiaColor, FillRule, FilterQuality, Mask, Paint, Path, PathBuilder, Pixmap,
    PixmapPaint, PixmapRef, Rect as SkiaRect, Stroke, Transform,
};

use crate::draw_command::{
    DisplayList, DrawCommand, ImageCommand, ImageSampling, SvgCommand, TextCommand,
};
use crate::font::create_font_system;
use crate::geometry::Rect;
use crate::image::ImageData;
use crate::platform::{
    ButtonState, CursorIcon, PlatformApplication, PlatformEvent, PlatformWindow, PointerButton,
    WindowConfig,
};
use crate::renderer::Viewport;
use crate::svg::SvgData;
use crate::theme::Color;

const COMPOSITOR_SERVICE_NAME: &str = "compositor.service";
const DISPLAY_SERVICE_NAME: &str = "display.driver";
const INPUT_SERVICE_NAME: &str = "input.service";
const WINDOW_OVERLAY_CAPABILITY: &str = "window.overlay";
const DISPLAY_GET_INFO_OPCODE: u32 = 1;
const OP_CREATE_SURFACE: u32 = 1;
const OP_ATTACH_BUFFER: u32 = 2;
const OP_DAMAGE: u32 = 3;
const OP_COMMIT: u32 = 4;
const ROLE_TOPLEVEL: u32 = 1;
const ROLE_BACKGROUND: u32 = 3;
const PIXEL_FORMAT_XRGB8888: u32 = 1;
const PAGE_SIZE: usize = 4096;
const MAX_SURFACE_EXTENT: u32 = 16_384;
const ERRNO_EAGAIN: u64 = 11;
const EVENT_POINTER_ENTER: u32 = 2;
const EVENT_POINTER_LEAVE: u32 = 3;
const EVENT_POINTER_MOTION: u32 = 4;
const EVENT_POINTER_BUTTON: u32 = 5;
const EVENT_KEY: u32 = 6;
const EVENT_FOCUS_GAINED: u32 = 8;
const EVENT_FOCUS_LOST: u32 = 9;
const EVENT_FRAME_DONE: u32 = 10;
const INPUT_SUBSCRIBE_OPCODE: u32 = 0x5355_4253;
const INPUT_EVENT_SIZE: usize = 32;
const INPUT_EVENT_KIND_POINTER_MOVE: u16 = 2;
const INPUT_EVENT_KIND_POINTER_BUTTON: u16 = 3;
const INPUT_EVENT_KIND_POINTER_ABSOLUTE: u16 = 5;
const KEY_BACKSPACE: u16 = 2;
const KEY_TAB: u16 = 3;
const KEY_ENTER: u16 = 4;
const KEY_SPACE: u16 = 5;
const KEY_DELETE: u16 = 79;
const KEY_HOME: u16 = 80;
const KEY_END: u16 = 81;
const KEY_LEFT: u16 = 82;
const KEY_RIGHT: u16 = 83;
const KEY_PAGE_UP: u16 = 86;
const KEY_PAGE_DOWN: u16 = 87;
const INPUT_FLAG_PRESS: u16 = 1 << 0;
const INPUT_FLAG_RELEASE: u16 = 1 << 1;
const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;
const SVG_SMALL_RENDER_LIMIT: f32 = 256.0;
const SVG_SMALL_RENDER_SUPERSAMPLE: f32 = 2.0;
const CURSOR_SVG_PATH: &str = "/system/icons/cursor.svg";
const CURSOR_WIDTH: u32 = 35;
const CURSOR_HEIGHT: u32 = 60;
const CURSOR_HOTSPOT_X: f32 = 1.0;
const CURSOR_HOTSPOT_Y: f32 = 1.0;
const METRICS_INTERVAL_TICKS: u64 = 500;
const SLOW_FRAME_THRESHOLD_TICKS: u64 = 16;
const INITIAL_FRAME_LOGS: u64 = 8;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextLayoutKey {
    text: String,
    font_family: String,
    font_size_bits: u32,
    line_height_bits: u32,
    width_bits: u32,
    height_bits: u32,
    scale_bits: u32,
    weight: u16,
    alignment: u8,
}

impl TextLayoutKey {
    fn new(command: &TextCommand, scale: f32) -> Self {
        Self {
            text: command.text.clone(),
            font_family: command.font_family.clone(),
            font_size_bits: canonical_f32_bits(command.font_size),
            line_height_bits: canonical_f32_bits(command.line_height),
            width_bits: canonical_f32_bits(command.bounds.size.width),
            height_bits: canonical_f32_bits(command.bounds.size.height),
            scale_bits: canonical_f32_bits(scale),
            weight: command.weight.clamp(1, 1000),
            alignment: alignment_key(command.alignment),
        }
    }
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    }
}

const fn alignment_key(alignment: crate::typography::TextAlignment) -> u8 {
    match alignment {
        crate::typography::TextAlignment::Start => 0,
        crate::typography::TextAlignment::Center => 1,
        crate::typography::TextAlignment::End => 2,
        crate::typography::TextAlignment::Justified => 3,
    }
}

static mut CREATE_SURFACE_REQ: [u8; 24] = [0; 24];
static mut ATTACH_BUFFER_REQ: [u8; 28] = [0; 28];
static mut TOKEN_REQ: [u8; 12] = [0; 12];
static mut DAMAGE_REQ: [u8; 28] = [0; 28];
static mut IPC_REPLY: [u8; 16] = [0; 16];
static mut EVENT_BUF: [u8; 32] = [0; 32];
static mut DISPLAY_REQ: [u8; 20] = [0; 20];
static mut DISPLAY_REPLY: [u8; 32] = [0; 32];
static mut INPUT_SUBSCRIBE_REQ: [u8; 16] = [0; 16];
static mut INPUT_SUBSCRIBE_REPLY: [u8; 1] = [0; 1];

#[derive(Debug, thiserror::Error)]
pub enum MochiOsBackendError {
    #[error("mochiOS syscall failed: {0}")]
    Syscall(u64),

    #[error("compositor.service was not found")]
    CompositorNotFound,

    #[error("invalid compositor reply")]
    InvalidReply,

    #[error("invalid window size")]
    InvalidWindowSize,

    #[error("arithmetic overflow")]
    ArithmeticOverflow,

    #[error("invalid compositor event")]
    InvalidEvent,
}

pub struct MochiOsBackend<A>
where
    A: PlatformApplication,
{
    app: A,
    config: WindowConfig,
    pressed_buttons: Vec<u16>,
    font_system: Option<FontSystem>,
    swash_cache: SwashCache,
    text_layout_cache: HashMap<TextLayoutKey, Buffer>,
    pixmap: Option<Pixmap>,
    direct_input: bool,
    pointer_x: f32,
    pointer_y: f32,
    cursor_image: Option<ImageData>,
    cursor_dirty: Option<Rect>,
    clear_color: Color,
    pending_pointer_motion: PendingPointerMotion,
    metrics: BackendMetrics,
}

#[derive(Default)]
struct PendingPointerMotion {
    absolute: Option<(f32, f32)>,
    relative_dx: f32,
    relative_dy: f32,
    pending: bool,
}

#[derive(Default)]
struct BackendMetrics {
    next_report_tick: u64,
    full_frames: u64,
    cursor_frames: u64,
    frame_logs_emitted: u64,
    input_events: u64,
    coalesced_pointer_events: u64,
    draw_cycles: u64,
    render_cycles: u64,
    attach_cycles: u64,
    commit_cycles: u64,
}

impl<A> MochiOsBackend<A>
where
    A: PlatformApplication,
{
    pub fn new(app: A, config: WindowConfig) -> Self {
        Self {
            app,
            config,
            pressed_buttons: Vec::new(),
            font_system: None,
            swash_cache: SwashCache::new(),
            text_layout_cache: HashMap::new(),
            pixmap: None,
            direct_input: false,
            pointer_x: 0.0,
            pointer_y: 0.0,
            cursor_image: None,
            cursor_dirty: None,
            clear_color: Color::BLACK,
            pending_pointer_motion: PendingPointerMotion::default(),
            metrics: BackendMetrics::default(),
        }
    }

    pub fn run(mut self) -> Result<(), MochiOsBackendError> {
        let compositor = find_compositor()?;
        let event_endpoint = create_event_endpoint()?;
        if self.config.fullscreen {
            require_window_overlay_capability()?;
        }
        let requested_size = if self.config.fullscreen {
            display_surface_size().unwrap_or_else(|| self.config.size)
        } else {
            self.config.size
        };
        let size = checked_surface_size(requested_size)?;
        let logical_size = if self.config.fullscreen {
            requested_size
        } else {
            self.config.size
        };
        let viewport = Viewport::new(logical_size, size.0, size.1, 1.0);
        let window = MochiOsWindow::new(viewport);
        let role = if self.config.fullscreen {
            ROLE_BACKGROUND
        } else {
            ROLE_TOPLEVEL
        };
        let token = create_surface(compositor, event_endpoint, role, size.0, size.1)?;
        let mut shared_buffer = SharedBuffer::new(size.0 as usize, size.1 as usize)?;
        self.pointer_x = (viewport.logical_size.width / 2.0).max(0.0);
        self.pointer_y = (viewport.logical_size.height / 2.0).max(0.0);
        self.direct_input = self.config.fullscreen && subscribe_input_events(event_endpoint);
        if self.direct_input {
            self.cursor_image = load_cursor_image();
        }
        self.log_backend_started(size);

        self.app
            .handle_event(PlatformEvent::Resumed { viewport }, &window);
        window.request_redraw();

        let mut display_list = DisplayList::new();
        loop {
            let mut handled_work = false;

            while let Some((len, event)) = try_recv_event()? {
                self.handle_or_queue_event_message(len, event, &window)?;
                handled_work = true;
            }
            if self.flush_pending_pointer_motion(&window) {
                handled_work = true;
            }

            let redraw_due = self
                .app
                .next_redraw_at()
                .is_some_and(|deadline| deadline <= Instant::now());

            let redraw_requested = window.take_redraw_requested();
            if redraw_requested || redraw_due {
                if self.font_system.is_none() {
                    self.font_system = Some(create_font_system());
                }
                display_list.clear();
                let frame_start = perf_counter();
                let frame_tick_start = perf_tick();
                let draw_start = perf_counter();
                let mut dirty_bounds = self.app.draw(window.viewport(), &mut display_list);
                let draw_cycles = perf_counter_elapsed(draw_start);
                self.metrics.draw_cycles = self.metrics.draw_cycles.saturating_add(draw_cycles);
                if let Some(cursor_rect) = self.current_cursor_rect(window.viewport()) {
                    dirty_bounds = dirty_bounds.union(cursor_rect);
                }
                self.cursor_dirty = None;
                let render_start = perf_counter();
                let clear_color = render_display_list(
                    window.viewport(),
                    dirty_bounds,
                    &display_list,
                    self.font_system
                        .as_mut()
                        .ok_or(MochiOsBackendError::InvalidWindowSize)?,
                    &mut self.swash_cache,
                    &mut self.text_layout_cache,
                    &mut self.pixmap,
                )?;
                let render_cycles = perf_counter_elapsed(render_start);
                self.metrics.render_cycles =
                    self.metrics.render_cycles.saturating_add(render_cycles);
                self.clear_color = clear_color;
                let pixmap = self
                    .pixmap
                    .as_ref()
                    .ok_or(MochiOsBackendError::InvalidWindowSize)?;
                let attach_start = perf_counter();
                attach_buffer(
                    compositor,
                    token,
                    window.width() as usize,
                    window.height() as usize,
                    pixmap,
                    clear_color,
                    &mut shared_buffer,
                    window.viewport(),
                    dirty_bounds,
                    self.cursor_blit(),
                )?;
                let attach_cycles = perf_counter_elapsed(attach_start);
                self.metrics.attach_cycles =
                    self.metrics.attach_cycles.saturating_add(attach_cycles);
                let commit_start = perf_counter();
                damage_token_request(compositor, token, window.viewport(), dirty_bounds)?;
                simple_token_request(compositor, OP_COMMIT, token)?;
                let commit_cycles = perf_counter_elapsed(commit_start);
                self.metrics.commit_cycles =
                    self.metrics.commit_cycles.saturating_add(commit_cycles);
                self.metrics.full_frames = self.metrics.full_frames.saturating_add(1);
                self.report_frame_timing(
                    "full",
                    perf_counter_elapsed(frame_start),
                    perf_tick_elapsed(frame_tick_start),
                    draw_cycles,
                    render_cycles,
                    attach_cycles,
                    commit_cycles,
                    dirty_bounds,
                );
                self.report_metrics_if_due();
                handled_work = true;
            } else if let Some(dirty_bounds) = self.cursor_dirty.take() {
                if let (Some(pixmap), Some(_cursor)) =
                    (self.pixmap.as_ref(), self.cursor_image.as_ref())
                {
                    let frame_start = perf_counter();
                    let frame_tick_start = perf_tick();
                    let attach_start = perf_counter();
                    attach_buffer(
                        compositor,
                        token,
                        window.width() as usize,
                        window.height() as usize,
                        pixmap,
                        self.clear_color,
                        &mut shared_buffer,
                        window.viewport(),
                        dirty_bounds,
                        self.cursor_blit(),
                    )?;
                    let attach_cycles = perf_counter_elapsed(attach_start);
                    self.metrics.attach_cycles =
                        self.metrics.attach_cycles.saturating_add(attach_cycles);
                    let commit_start = perf_counter();
                    damage_token_request(compositor, token, window.viewport(), dirty_bounds)?;
                    simple_token_request(compositor, OP_COMMIT, token)?;
                    let commit_cycles = perf_counter_elapsed(commit_start);
                    self.metrics.commit_cycles =
                        self.metrics.commit_cycles.saturating_add(commit_cycles);
                    self.metrics.cursor_frames = self.metrics.cursor_frames.saturating_add(1);
                    self.report_frame_timing(
                        "cursor",
                        perf_counter_elapsed(frame_start),
                        perf_tick_elapsed(frame_tick_start),
                        0,
                        0,
                        attach_cycles,
                        commit_cycles,
                        dirty_bounds,
                    );
                    self.report_metrics_if_due();
                    handled_work = true;
                }
            }

            if !handled_work {
                if let Some(deadline) = self.app.next_redraw_at() {
                    if wait_until_deadline(deadline, &window, &mut self)? {
                        continue;
                    }
                } else {
                    wait_for_event(event_endpoint, &window, &mut self)?;
                }
            }
        }
    }

    fn handle_event_message(
        &mut self,
        len: usize,
        event: [u8; 32],
        window: &MochiOsWindow,
    ) -> Result<(), MochiOsBackendError> {
        if self.direct_input && len == INPUT_EVENT_SIZE && self.handle_input_event(event, window) {
            return Ok(());
        }

        self.handle_compositor_event(event, window)
    }

    fn handle_or_queue_event_message(
        &mut self,
        len: usize,
        event: [u8; 32],
        window: &MochiOsWindow,
    ) -> Result<(), MochiOsBackendError> {
        if self.direct_input && len == INPUT_EVENT_SIZE && self.queue_pointer_motion(event) {
            self.metrics.input_events = self.metrics.input_events.saturating_add(1);
            self.metrics.coalesced_pointer_events =
                self.metrics.coalesced_pointer_events.saturating_add(1);
            return Ok(());
        }

        self.flush_pending_pointer_motion(window);
        self.metrics.input_events = self.metrics.input_events.saturating_add(1);
        self.handle_event_message(len, event, window)
    }

    fn queue_pointer_motion(&mut self, event: [u8; 32]) -> bool {
        let kind = u16::from_le_bytes([event[0], event[1]]);
        match kind {
            INPUT_EVENT_KIND_POINTER_MOVE => {
                let dx = i32::from_le_bytes([event[12], event[13], event[14], event[15]]) as f32;
                let dy = i32::from_le_bytes([event[16], event[17], event[18], event[19]]) as f32;
                self.pending_pointer_motion.relative_dx += dx;
                self.pending_pointer_motion.relative_dy += dy;
                self.pending_pointer_motion.pending = true;
                true
            }
            INPUT_EVENT_KIND_POINTER_ABSOLUTE => {
                let raw_x = i32::from_le_bytes([event[12], event[13], event[14], event[15]])
                    .clamp(0, 32_767) as f32;
                let raw_y = i32::from_le_bytes([event[16], event[17], event[18], event[19]])
                    .clamp(0, 32_767) as f32;
                self.pending_pointer_motion.absolute = Some((raw_x, raw_y));
                self.pending_pointer_motion.relative_dx = 0.0;
                self.pending_pointer_motion.relative_dy = 0.0;
                self.pending_pointer_motion.pending = true;
                true
            }
            _ => false,
        }
    }

    fn flush_pending_pointer_motion(&mut self, window: &MochiOsWindow) -> bool {
        if !self.pending_pointer_motion.pending {
            return false;
        }

        let previous = self.current_cursor_rect(window.viewport());
        let bounds = window.viewport().logical_bounds();
        if let Some((raw_x, raw_y)) = self.pending_pointer_motion.absolute.take() {
            self.pointer_x = bounds.origin.x + (raw_x / 32_767.0) * bounds.size.width;
            self.pointer_y = bounds.origin.y + (raw_y / 32_767.0) * bounds.size.height;
        }
        let max_x = (bounds.origin.x + bounds.size.width).max(bounds.origin.x);
        let max_y = (bounds.origin.y + bounds.size.height).max(bounds.origin.y);
        self.pointer_x = (self.pointer_x + self.pending_pointer_motion.relative_dx)
            .clamp(bounds.origin.x, max_x);
        self.pointer_y = (self.pointer_y + self.pending_pointer_motion.relative_dy)
            .clamp(bounds.origin.y, max_y);

        self.pending_pointer_motion.relative_dx = 0.0;
        self.pending_pointer_motion.relative_dy = 0.0;
        self.pending_pointer_motion.pending = false;

        self.app.handle_event(
            PlatformEvent::PointerMoved {
                x: self.pointer_x,
                y: self.pointer_y,
            },
            window,
        );
        self.mark_cursor_dirty(window.viewport(), previous);
        true
    }

    fn handle_input_event(&mut self, event: [u8; 32], window: &MochiOsWindow) -> bool {
        let kind = u16::from_le_bytes([event[0], event[1]]);
        match kind {
            INPUT_EVENT_KIND_POINTER_MOVE => {
                let previous = self.current_cursor_rect(window.viewport());
                let dx = i32::from_le_bytes([event[12], event[13], event[14], event[15]]) as f32;
                let dy = i32::from_le_bytes([event[16], event[17], event[18], event[19]]) as f32;
                let bounds = window.viewport().logical_bounds();
                let max_x = (bounds.origin.x + bounds.size.width).max(bounds.origin.x);
                let max_y = (bounds.origin.y + bounds.size.height).max(bounds.origin.y);
                self.pointer_x = (self.pointer_x + dx).clamp(bounds.origin.x, max_x);
                self.pointer_y = (self.pointer_y + dy).clamp(bounds.origin.y, max_y);
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: self.pointer_x,
                        y: self.pointer_y,
                    },
                    window,
                );
                self.mark_cursor_dirty(window.viewport(), previous);
                true
            }
            INPUT_EVENT_KIND_POINTER_ABSOLUTE => {
                let previous = self.current_cursor_rect(window.viewport());
                let raw_x = i32::from_le_bytes([event[12], event[13], event[14], event[15]])
                    .clamp(0, 32_767) as f32;
                let raw_y = i32::from_le_bytes([event[16], event[17], event[18], event[19]])
                    .clamp(0, 32_767) as f32;
                let bounds = window.viewport().logical_bounds();
                self.pointer_x = bounds.origin.x + (raw_x / 32_767.0) * bounds.size.width;
                self.pointer_y = bounds.origin.y + (raw_y / 32_767.0) * bounds.size.height;
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: self.pointer_x,
                        y: self.pointer_y,
                    },
                    window,
                );
                self.mark_cursor_dirty(window.viewport(), previous);
                true
            }
            INPUT_EVENT_KIND_POINTER_BUTTON => {
                let flags = u16::from_le_bytes([event[2], event[3]]);
                let detail = u16::from_le_bytes([event[6], event[7]]);
                let button = match detail {
                    1 => PointerButton::Primary,
                    2 => PointerButton::Secondary,
                    3 => PointerButton::Middle,
                    other => PointerButton::Other(other),
                };
                let state = if flags & INPUT_FLAG_PRESS != 0 {
                    ButtonState::Pressed
                } else if flags & INPUT_FLAG_RELEASE != 0 {
                    ButtonState::Released
                } else {
                    return true;
                };
                self.app
                    .handle_event(PlatformEvent::PointerButton { button, state }, window);
                true
            }
            _ => false,
        }
    }

    fn report_metrics_if_due(&mut self) {
        let now = perf_tick();
        if self.metrics.next_report_tick == 0 {
            self.metrics.next_report_tick = now.saturating_add(METRICS_INTERVAL_TICKS);
        }
        if now < self.metrics.next_report_tick {
            return;
        }

        let mut line = String::new();
        let _ = write!(
            line,
            "viewkit/mochios stats: full={} cursor={} input={} coalesced={} draw={}cy render={}cy attach={}cy commit={}cy\n",
            self.metrics.full_frames,
            self.metrics.cursor_frames,
            self.metrics.input_events,
            self.metrics.coalesced_pointer_events,
            self.metrics.draw_cycles,
            self.metrics.render_cycles,
            self.metrics.attach_cycles,
            self.metrics.commit_cycles,
        );
        perf_log(&line);

        self.metrics.full_frames = 0;
        self.metrics.cursor_frames = 0;
        self.metrics.input_events = 0;
        self.metrics.coalesced_pointer_events = 0;
        self.metrics.draw_cycles = 0;
        self.metrics.render_cycles = 0;
        self.metrics.attach_cycles = 0;
        self.metrics.commit_cycles = 0;
        self.metrics.next_report_tick = now.saturating_add(METRICS_INTERVAL_TICKS);
    }

    fn report_frame_timing(
        &mut self,
        kind: &str,
        total_cycles: u64,
        total_ticks: u64,
        draw_cycles: u64,
        render_cycles: u64,
        attach_cycles: u64,
        commit_cycles: u64,
        dirty_bounds: Rect,
    ) {
        let force_initial = self.metrics.frame_logs_emitted < INITIAL_FRAME_LOGS;
        if !force_initial && total_ticks < SLOW_FRAME_THRESHOLD_TICKS {
            return;
        }
        self.metrics.frame_logs_emitted = self.metrics.frame_logs_emitted.saturating_add(1);
        let label = if total_ticks < SLOW_FRAME_THRESHOLD_TICKS {
            "frame"
        } else {
            "slow-frame"
        };
        let mut line = String::new();
        let _ = write!(
            line,
            "viewkit/mochios {} kind={} total={}cy ticks={} draw={}cy render={}cy attach={}cy commit={}cy dirty=({:.0},{:.0} {:.0}x{:.0})\n",
            label,
            kind,
            total_cycles,
            total_ticks,
            draw_cycles,
            render_cycles,
            attach_cycles,
            commit_cycles,
            dirty_bounds.origin.x,
            dirty_bounds.origin.y,
            dirty_bounds.size.width,
            dirty_bounds.size.height,
        );
        perf_log(&line);
    }

    fn log_backend_started(&self, size: (u32, u32)) {
        let mut line = String::new();
        let _ = write!(
            line,
            "viewkit/mochios perf-start fullscreen={} size={}x{} direct_input={}\n",
            self.config.fullscreen, size.0, size.1, self.direct_input,
        );
        perf_log(&line);
    }

    fn current_cursor_rect(&self, viewport: Viewport) -> Option<Rect> {
        self.cursor_image.as_ref()?;
        let bounds = viewport.logical_bounds();
        Some(
            Rect::new(
                self.pointer_x - CURSOR_HOTSPOT_X,
                self.pointer_y - CURSOR_HOTSPOT_Y,
                CURSOR_WIDTH as f32,
                CURSOR_HEIGHT as f32,
            )
            .intersection(bounds)
            .unwrap_or_else(|| Rect::new(self.pointer_x, self.pointer_y, 1.0, 1.0)),
        )
    }

    fn mark_cursor_dirty(&mut self, viewport: Viewport, previous: Option<Rect>) {
        let Some(current) = self.current_cursor_rect(viewport) else {
            return;
        };
        let dirty = previous
            .map_or(current, |previous| previous.union(current))
            .expanded(2.0);
        self.cursor_dirty = Some(self.cursor_dirty.map_or(dirty, |old| old.union(dirty)));
    }

    fn cursor_blit(&self) -> Option<CursorBlit<'_>> {
        Some(CursorBlit {
            image: self.cursor_image.as_ref()?,
            x: self.pointer_x - CURSOR_HOTSPOT_X,
            y: self.pointer_y - CURSOR_HOTSPOT_Y,
        })
    }

    fn handle_compositor_event(
        &mut self,
        event: [u8; 32],
        window: &MochiOsWindow,
    ) -> Result<(), MochiOsBackendError> {
        let kind = unsafe { read_u32_raw(event.as_ptr(), 0) };
        let a = unsafe { read_i32_raw(event.as_ptr(), 4) };
        let b = unsafe { read_i32_raw(event.as_ptr(), 8) };
        let c = unsafe { read_u32_raw(event.as_ptr(), 12) };

        match kind {
            EVENT_POINTER_ENTER | EVENT_POINTER_MOTION => {
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: a as f32,
                        y: b as f32,
                    },
                    window,
                );
            }
            EVENT_POINTER_LEAVE => {
                self.app.handle_event(PlatformEvent::PointerLeft, window);
            }
            EVENT_POINTER_BUTTON => {
                let button_id = (c & 0xffff) as u16;
                let flags = c >> 16;
                let button = match button_id {
                    1 => PointerButton::Primary,
                    2 => PointerButton::Secondary,
                    3 => PointerButton::Middle,
                    other => PointerButton::Other(other),
                };
                let state = if flags & u32::from(INPUT_FLAG_PRESS) != 0 {
                    if !self.pressed_buttons.contains(&button_id) {
                        self.pressed_buttons.push(button_id);
                    }
                    ButtonState::Pressed
                } else if flags & u32::from(INPUT_FLAG_RELEASE) != 0 {
                    if let Some(pos) = self
                        .pressed_buttons
                        .iter()
                        .position(|pressed| *pressed == button_id)
                    {
                        self.pressed_buttons.swap_remove(pos);
                    }
                    ButtonState::Released
                } else {
                    self.toggle_button_state(button_id)
                };
                self.app
                    .handle_event(PlatformEvent::PointerButton { button, state }, window);
            }
            EVENT_KEY => {
                if c & 1 != 0 {
                    if let Some(event) = self.key_event(a as u16, b as u32) {
                        self.app.handle_event(event, window);
                    }
                }
            }
            EVENT_FOCUS_GAINED => {
                self.app.handle_event(PlatformEvent::Focused(true), window);
            }
            EVENT_FOCUS_LOST => {
                self.app.handle_event(PlatformEvent::Focused(false), window);
            }
            EVENT_FRAME_DONE => {}
            _ => {}
        }

        Ok(())
    }

    fn key_event(&self, keycode: u16, codepoint: u32) -> Option<PlatformEvent> {
        if let Some(text) = char::from_u32(codepoint)
            && !text.is_control()
        {
            return Some(PlatformEvent::TextInput {
                text: text.to_string(),
            });
        }
        Some(match keycode {
            KEY_BACKSPACE => PlatformEvent::Backspace,
            KEY_TAB => PlatformEvent::TextInput {
                text: String::from("\t"),
            },
            KEY_ENTER => PlatformEvent::TextInput {
                text: String::from("\n"),
            },
            KEY_SPACE => PlatformEvent::TextInput {
                text: String::from(" "),
            },
            KEY_DELETE => PlatformEvent::Delete,
            KEY_HOME => PlatformEvent::Home,
            KEY_END => PlatformEvent::End,
            KEY_LEFT => PlatformEvent::ArrowLeft,
            KEY_RIGHT => PlatformEvent::ArrowRight,
            KEY_PAGE_UP => PlatformEvent::SelectHome,
            KEY_PAGE_DOWN => PlatformEvent::SelectEnd,
            _ => return None,
        })
    }

    fn toggle_button_state(&mut self, button_id: u16) -> ButtonState {
        if let Some(pos) = self
            .pressed_buttons
            .iter()
            .position(|pressed| *pressed == button_id)
        {
            self.pressed_buttons.swap_remove(pos);
            ButtonState::Released
        } else {
            self.pressed_buttons.push(button_id);
            ButtonState::Pressed
        }
    }
}

struct MochiOsWindow {
    viewport: Viewport,
    redraw_requested: Cell<bool>,
}

impl MochiOsWindow {
    fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            redraw_requested: Cell::new(false),
        }
    }

    const fn width(&self) -> u32 {
        self.viewport.physical_width
    }

    const fn height(&self) -> u32 {
        self.viewport.physical_height
    }

    fn take_redraw_requested(&self) -> bool {
        self.redraw_requested.replace(false)
    }
}

impl PlatformWindow for MochiOsWindow {
    fn request_redraw(&self) {
        self.redraw_requested.set(true);
    }

    fn set_title(&self, title: &str) {
        let _ = title;
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn set_cursor(&self, cursor: CursorIcon) {
        let _ = cursor;
    }
}

fn checked_surface_size(size: crate::geometry::Size) -> Result<(u32, u32), MochiOsBackendError> {
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }

    let width = size.width.round();
    let height = size.height.round();

    if width < 1.0
        || height < 1.0
        || width > MAX_SURFACE_EXTENT as f32
        || height > MAX_SURFACE_EXTENT as f32
    {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }

    Ok((width as u32, height as u32))
}

fn syscall_result<T>(result: syscall::SysResult<T>) -> Result<T, MochiOsBackendError> {
    result.map_err(|err| MochiOsBackendError::Syscall(err.errno().unwrap_or(5)))
}

fn perf_log(line: &str) {
    let _ = syscall::call3(
        syscall::SyscallNumber::Write,
        2,
        line.as_ptr() as u64,
        line.len() as u64,
    );
}

fn perf_counter() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: rdtsc is a userspace-readable counter on the current x86_64 target.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        perf_tick()
    }
}

fn perf_counter_elapsed(start: u64) -> u64 {
    perf_counter().saturating_sub(start)
}

fn perf_tick() -> u64 {
    syscall::call0(syscall::SyscallNumber::TimeNow).unwrap_or(0)
}

fn perf_tick_elapsed(start: u64) -> u64 {
    perf_tick().saturating_sub(start)
}

fn create_event_endpoint() -> Result<u64, MochiOsBackendError> {
    syscall_result(syscall::call2(syscall::SyscallNumber::IpcCreate, 0, 0))
}

fn find_compositor() -> Result<u64, MochiOsBackendError> {
    let name = COMPOSITOR_SERVICE_NAME.as_bytes();
    for _ in 0..64 {
        let tid = syscall_result(syscall::call2(
            syscall::SyscallNumber::FindProcessByName,
            name.as_ptr() as u64,
            name.len() as u64,
        ))?;
        if tid != 0 {
            return Ok(tid);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
    Err(MochiOsBackendError::CompositorNotFound)
}

fn find_display_driver() -> Result<u64, MochiOsBackendError> {
    let name = DISPLAY_SERVICE_NAME.as_bytes();
    for _ in 0..64 {
        let tid = syscall_result(syscall::call2(
            syscall::SyscallNumber::FindProcessByName,
            name.as_ptr() as u64,
            name.len() as u64,
        ))?;
        if tid != 0 {
            return Ok(tid);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
    Err(MochiOsBackendError::InvalidReply)
}

fn find_input_service() -> Result<u64, MochiOsBackendError> {
    let name = INPUT_SERVICE_NAME.as_bytes();
    for _ in 0..64 {
        let tid = syscall_result(syscall::call2(
            syscall::SyscallNumber::FindProcessByName,
            name.as_ptr() as u64,
            name.len() as u64,
        ))?;
        if tid != 0 {
            return Ok(tid);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
    Err(MochiOsBackendError::InvalidReply)
}

fn subscribe_input_events(endpoint: u64) -> bool {
    let Ok(input) = find_input_service() else {
        return false;
    };
    let request = core::ptr::addr_of_mut!(INPUT_SUBSCRIBE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(INPUT_SUBSCRIBE_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 16);
        put_u32_raw(request, 0, INPUT_SUBSCRIBE_OPCODE);
        put_u64_raw(request, 8, endpoint);
        zero_raw(reply, 1);
    }
    matches!(ipc_call_raw(input, request, 16, reply, 1), Ok(1))
}

fn require_window_overlay_capability() -> Result<(), MochiOsBackendError> {
    if query_capability(WINDOW_OVERLAY_CAPABILITY) {
        return Ok(());
    }
    Err(MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))
}

fn query_capability(capability: &str) -> bool {
    let bytes = capability.as_bytes();
    matches!(
        syscall::call2(
            syscall::SyscallNumber::CapQuery,
            bytes.as_ptr() as u64,
            bytes.len() as u64,
        ),
        Ok(1)
    )
}

fn display_surface_size() -> Option<crate::geometry::Size> {
    let display = find_display_driver().ok()?;
    let request = core::ptr::addr_of_mut!(DISPLAY_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(DISPLAY_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 20);
        zero_raw(reply, 32);
        put_u32_raw(request, 0, DISPLAY_GET_INFO_OPCODE);
    }
    let len = ipc_call_raw(display, request, 20, reply, 32).ok()?;
    if len < 20 {
        return None;
    }
    let status = unsafe { read_u32_raw(reply.cast_const(), 0) };
    if status != 0 {
        return None;
    }
    let width = unsafe { read_u32_raw(reply.cast_const(), 4) };
    let height = unsafe { read_u32_raw(reply.cast_const(), 8) };
    match (width, height) {
        (w, h) if w > 0 && h > 0 => Some(crate::geometry::Size::new(w as f32, h as f32)),
        _ => None,
    }
}

fn load_cursor_image() -> Option<ImageData> {
    let svg = SvgData::from_path(CURSOR_SVG_PATH).ok()?;
    ImageData::from_svg(&svg, CURSOR_WIDTH, CURSOR_HEIGHT).ok()
}

fn ipc_call_raw(
    dest: u64,
    req_ptr: *const u8,
    req_len: usize,
    reply_ptr: *mut u8,
    reply_len: usize,
) -> Result<usize, MochiOsBackendError> {
    let msg = syscall_result(syscall::call5(
        syscall::SyscallNumber::IpcCall,
        dest,
        req_ptr as u64,
        req_len as u64,
        reply_ptr as u64,
        reply_len as u64,
    ))?;
    Ok((msg & 0xffff_ffff) as usize)
}

fn ipc_wait_raw(
    endpoint: u64,
    buf_ptr: *mut u8,
    buf_len: usize,
) -> Result<usize, MochiOsBackendError> {
    let msg = syscall_result(syscall::call3(
        syscall::SyscallNumber::IpcWait,
        buf_ptr as u64,
        buf_len as u64,
        endpoint,
    ))?;
    Ok((msg & 0xffff_ffff) as usize)
}

fn alloc_shared_page_count(page_count: usize) -> Result<u64, MochiOsBackendError> {
    let virt = syscall_result(syscall::call4(
        syscall::SyscallNumber::AllocSharedPages,
        page_count as u64,
        0,
        0,
        0,
    ))?;
    if virt == 0 || (virt & (PAGE_SIZE as u64 - 1)) != 0 {
        return Err(MochiOsBackendError::Syscall(5));
    }
    Ok(virt)
}

fn send_pages(dest: u64, page_count: usize, local_base: u64) -> Result<(), MochiOsBackendError> {
    syscall_result(syscall::call4(
        syscall::SyscallNumber::IpcSendPages,
        dest,
        0,
        page_count as u64,
        local_base,
    ))?;
    Ok(())
}

struct SharedBuffer {
    virt: u64,
    byte_capacity: usize,
    sent_pages: bool,
    attached: bool,
}

#[derive(Clone, Copy)]
struct PhysicalDirtyRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct CursorBlit<'a> {
    image: &'a ImageData,
    x: f32,
    y: f32,
}

impl SharedBuffer {
    fn new(width: usize, height: usize) -> Result<Self, MochiOsBackendError> {
        let pixel_count = width
            .checked_mul(height)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let byte_len = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let page_count = byte_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let page_count = page_count.max(1);
        let byte_capacity = page_count
            .checked_mul(PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let virt = alloc_shared_page_count(page_count)?;

        Ok(Self {
            virt,
            byte_capacity,
            sent_pages: false,
            attached: false,
        })
    }

    fn send_pixmap_to(
        &mut self,
        compositor: u64,
        pixmap: &Pixmap,
        background: Color,
        dirty_rect: PhysicalDirtyRect,
        cursor: Option<CursorBlit<'_>>,
    ) -> Result<(), MochiOsBackendError> {
        let pixel_count = (pixmap.width() as usize)
            .checked_mul(pixmap.height() as usize)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let bytes_len = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        if bytes_len > self.byte_capacity {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let dst =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u8, self.byte_capacity) };

        let pixmap_width = pixmap.width() as usize;
        let pixmap_height = pixmap.height() as usize;
        let copy_rect = if self.sent_pages {
            dirty_rect
        } else {
            PhysicalDirtyRect {
                x: 0,
                y: 0,
                width: pixmap_width,
                height: pixmap_height,
            }
        };
        let right = copy_rect
            .x
            .saturating_add(copy_rect.width)
            .min(pixmap_width);
        let bottom = copy_rect
            .y
            .saturating_add(copy_rect.height)
            .min(pixmap_height);
        let src = pixmap.data();
        for y in copy_rect.y..bottom {
            let Some(row_start) = y.checked_mul(pixmap_width) else {
                return Err(MochiOsBackendError::ArithmeticOverflow);
            };
            for x in copy_rect.x..right {
                let Some(pixel_index) = row_start.checked_add(x) else {
                    return Err(MochiOsBackendError::ArithmeticOverflow);
                };
                let Some(byte_index) = pixel_index.checked_mul(4) else {
                    return Err(MochiOsBackendError::ArithmeticOverflow);
                };
                let Some(pixel) = src.get(byte_index..byte_index + 4) else {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                };
                let Some(out) = dst.get_mut(byte_index..byte_index + 4) else {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                };
                let mut value = flatten_premultiplied_pixel(pixel, background);
                if let Some(cursor) = cursor {
                    value = blend_cursor_pixel(value, cursor, x, y);
                }
                out.copy_from_slice(&value.to_le_bytes());
            }
        }
        let page_count = bytes_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        if self.sent_pages {
            return Ok(());
        }
        send_pages(compositor, page_count, self.virt)?;
        self.sent_pages = true;
        Ok(())
    }

    fn is_attached(&self) -> bool {
        self.attached
    }

    fn mark_attached(&mut self) {
        self.attached = true;
    }
}

fn flatten_premultiplied_pixel(pixel: &[u8], background: Color) -> u32 {
    let alpha = pixel[3] as u32;
    let inv_alpha = 255_u32.saturating_sub(alpha);

    // tiny-skia stores premultiplied RGBA. The compositor surface is XRGB,
    // so each pixel is flattened into the configured clear color.
    let red = pixel[0] as u32 + (background.red as u32 * inv_alpha + 127) / 255;
    let green = pixel[1] as u32 + (background.green as u32 * inv_alpha + 127) / 255;
    let blue = pixel[2] as u32 + (background.blue as u32 * inv_alpha + 127) / 255;

    0xff00_0000 | (red.min(255) << 16) | (green.min(255) << 8) | blue.min(255)
}

fn blend_cursor_pixel(base: u32, cursor: CursorBlit<'_>, x: usize, y: usize) -> u32 {
    let cursor_x = x as i32 - cursor.x.round() as i32;
    let cursor_y = y as i32 - cursor.y.round() as i32;
    if cursor_x < 0
        || cursor_y < 0
        || cursor_x >= cursor.image.width() as i32
        || cursor_y >= cursor.image.height() as i32
    {
        return base;
    }

    let cursor_x = cursor_x as usize;
    let cursor_y = cursor_y as usize;
    let width = cursor.image.width() as usize;
    let Some(pixel_index) = cursor_y
        .checked_mul(width)
        .and_then(|row| row.checked_add(cursor_x))
    else {
        return base;
    };
    let Some(byte_index) = pixel_index.checked_mul(4) else {
        return base;
    };
    let Some(pixel) = cursor
        .image
        .premultiplied_rgba8()
        .get(byte_index..byte_index + 4)
    else {
        return base;
    };

    let alpha = pixel[3] as u32;
    if alpha == 0 {
        return base;
    }

    let inv_alpha = 255_u32.saturating_sub(alpha);
    let base_red = (base >> 16) & 0xff;
    let base_green = (base >> 8) & 0xff;
    let base_blue = base & 0xff;
    let red = pixel[0] as u32 + (base_red * inv_alpha + 127) / 255;
    let green = pixel[1] as u32 + (base_green * inv_alpha + 127) / 255;
    let blue = pixel[2] as u32 + (base_blue * inv_alpha + 127) / 255;

    0xff00_0000 | (red.min(255) << 16) | (green.min(255) << 8) | blue.min(255)
}

unsafe fn zero_raw(ptr: *mut u8, len: usize) {
    unsafe {
        core::ptr::write_bytes(ptr, 0, len);
    }
}

unsafe fn put_u32_raw(ptr: *mut u8, offset: usize, value: u32) {
    unsafe {
        core::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(offset), 4);
    }
}

unsafe fn read_i32_raw(ptr: *const u8, offset: usize) -> i32 {
    let mut bytes = [0u8; 4];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 4);
    }
    i32::from_le_bytes(bytes)
}

unsafe fn put_u64_raw(ptr: *mut u8, offset: usize, value: u64) {
    unsafe {
        core::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(offset), 8);
    }
}

unsafe fn read_u32_raw(ptr: *const u8, offset: usize) -> u32 {
    let mut bytes = [0u8; 4];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 4);
    }
    u32::from_le_bytes(bytes)
}

unsafe fn read_u64_raw(ptr: *const u8, offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 8);
    }
    u64::from_le_bytes(bytes)
}

fn status_from_raw(ptr: *const u8, len: usize) -> Result<(), MochiOsBackendError> {
    if len < 4 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    let status = unsafe { read_u32_raw(ptr, 0) };
    if status == 0 {
        Ok(())
    } else {
        Err(MochiOsBackendError::Syscall(status as u64))
    }
}

fn try_recv_event() -> Result<Option<(usize, [u8; 32])>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = match ipc_wait_raw(0, event, 32) {
        Ok(len) => len,
        Err(MochiOsBackendError::Syscall(ERRNO_EAGAIN)) => return Ok(None),
        Err(err) => return Err(err),
    };
    if len < 16 {
        return Err(MochiOsBackendError::InvalidEvent);
    }
    let mut out = [0u8; 32];
    let copy_len = len.min(out.len());
    unsafe {
        core::ptr::copy_nonoverlapping(event, out.as_mut_ptr(), copy_len);
    }
    Ok(Some((len, out)))
}

fn wait_for_event<A: PlatformApplication>(
    endpoint: u64,
    window: &MochiOsWindow,
    backend: &mut MochiOsBackend<A>,
) -> Result<(), MochiOsBackendError> {
    if let Some((len, event)) = read_event_blocking(endpoint)? {
        backend.handle_or_queue_event_message(len, event, window)?;
        backend.flush_pending_pointer_motion(window);
    }
    Ok(())
}

fn wait_until_deadline<A: PlatformApplication>(
    deadline: Instant,
    window: &MochiOsWindow,
    backend: &mut MochiOsBackend<A>,
) -> Result<bool, MochiOsBackendError> {
    loop {
        if let Some((len, event)) = try_recv_event()? {
            backend.handle_or_queue_event_message(len, event, window)?;
            backend.flush_pending_pointer_motion(window);
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
}

fn read_event_blocking(endpoint: u64) -> Result<Option<(usize, [u8; 32])>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = match ipc_wait_raw(endpoint, event, 32) {
        Ok(len) => len,
        Err(MochiOsBackendError::Syscall(ERRNO_EAGAIN)) => return Ok(None),
        Err(err) => return Err(err),
    };
    if len < 16 {
        return Err(MochiOsBackendError::InvalidEvent);
    }
    let mut out = [0u8; 32];
    let copy_len = len.min(out.len());
    unsafe {
        core::ptr::copy_nonoverlapping(event, out.as_mut_ptr(), copy_len);
    }
    Ok(Some((len, out)))
}

fn create_surface(
    compositor: u64,
    event_endpoint: u64,
    role: u32,
    width: u32,
    height: u32,
) -> Result<u64, MochiOsBackendError> {
    let request = core::ptr::addr_of_mut!(CREATE_SURFACE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 24);
        put_u32_raw(request, 0, OP_CREATE_SURFACE);
        put_u32_raw(request, 4, role);
        put_u32_raw(request, 8, width);
        put_u32_raw(request, 12, height);
        put_u64_raw(request, 16, event_endpoint);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 24, reply, 16)?;
    if len < 12 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    status_from_raw(reply, len)?;
    Ok(unsafe { read_u64_raw(reply, 4) })
}

fn attach_buffer(
    compositor: u64,
    token: u64,
    width: usize,
    height: usize,
    pixmap: &Pixmap,
    background: Color,
    shared_buffer: &mut SharedBuffer,
    viewport: Viewport,
    dirty_bounds: Rect,
    cursor: Option<CursorBlit<'_>>,
) -> Result<(), MochiOsBackendError> {
    let pixel_count = width
        .checked_mul(height)
        .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
    let pixmap_pixel_count = (pixmap.width() as usize)
        .checked_mul(pixmap.height() as usize)
        .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
    if pixmap.width() as usize != width || pixmap.height() as usize != height {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }
    if pixmap_pixel_count < pixel_count {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }
    if !shared_buffer.is_attached() {
        let request = core::ptr::addr_of_mut!(ATTACH_BUFFER_REQ).cast::<u8>();
        let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
        unsafe {
            zero_raw(request, 28);
            put_u32_raw(request, 0, OP_ATTACH_BUFFER);
            put_u64_raw(request, 4, token);
            put_u32_raw(request, 12, width as u32);
            put_u32_raw(request, 16, height as u32);
            put_u32_raw(request, 20, width as u32);
            put_u32_raw(request, 24, PIXEL_FORMAT_XRGB8888);
            zero_raw(reply, 16);
        }
        let len = ipc_call_raw(compositor, request, 28, reply, 16)?;
        status_from_raw(reply, len)?;
        shared_buffer.mark_attached();
    }
    let dirty_rect = physical_dirty_rect(viewport, dirty_bounds);
    shared_buffer.send_pixmap_to(compositor, pixmap, background, dirty_rect, cursor)
}

fn simple_token_request(
    compositor: u64,
    opcode: u32,
    token: u64,
) -> Result<(), MochiOsBackendError> {
    let request = core::ptr::addr_of_mut!(TOKEN_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 12);
        put_u32_raw(request, 0, opcode);
        put_u64_raw(request, 4, token);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 12, reply, 16)?;
    status_from_raw(reply, len)
}

fn physical_dirty_rect(viewport: Viewport, dirty_bounds: Rect) -> PhysicalDirtyRect {
    let viewport_bounds = viewport.logical_bounds();
    let dirty = dirty_bounds
        .intersection(viewport_bounds)
        .unwrap_or(viewport_bounds);
    let scale = valid_scale_factor(viewport.scale_factor);
    let x = (dirty.origin.x * scale).floor().max(0.0);
    let y = (dirty.origin.y * scale).floor().max(0.0);
    let right = ((dirty.origin.x + dirty.size.width) * scale)
        .ceil()
        .min(viewport.physical_width as f32);
    let bottom = ((dirty.origin.y + dirty.size.height) * scale)
        .ceil()
        .min(viewport.physical_height as f32);
    let width = (right - x).max(1.0);
    let height = (bottom - y).max(1.0);

    PhysicalDirtyRect {
        x: x as usize,
        y: y as usize,
        width: width as usize,
        height: height as usize,
    }
}

fn damage_token_request(
    compositor: u64,
    token: u64,
    viewport: Viewport,
    dirty_bounds: Rect,
) -> Result<(), MochiOsBackendError> {
    let dirty = physical_dirty_rect(viewport, dirty_bounds);

    let request = core::ptr::addr_of_mut!(DAMAGE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 28);
        put_u32_raw(request, 0, OP_DAMAGE);
        put_u64_raw(request, 4, token);
        put_u32_raw(request, 12, dirty.x as u32);
        put_u32_raw(request, 16, dirty.y as u32);
        put_u32_raw(request, 20, dirty.width as u32);
        put_u32_raw(request, 24, dirty.height as u32);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 28, reply, 16)?;
    status_from_raw(reply, len)
}

fn render_display_list(
    viewport: Viewport,
    dirty_bounds: Rect,
    display_list: &DisplayList,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text_layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    pixmap: &mut Option<Pixmap>,
) -> Result<Color, MochiOsBackendError> {
    let width = viewport.physical_width;
    let height = viewport.physical_height;
    let pixmap = reusable_pixmap(pixmap, width, height)?;
    let mut clear_color = Color::BLACK;

    let scale = valid_scale_factor(viewport.scale_factor);
    let transform = Transform::from_scale(scale, scale);
    let bounds = viewport.logical_bounds();
    let dirty_bounds = dirty_bounds.intersection(bounds).unwrap_or(bounds);
    let mut clip_stack = vec![create_clip_mask(
        dirty_bounds,
        None,
        width,
        height,
        transform,
    )?];

    for command in display_list.commands() {
        match command {
            DrawCommand::Clear { color } => {
                clear_color = *color;
                if let Some(rect) = to_skia_rect(dirty_bounds) {
                    let paint = solid_paint(*color);
                    pixmap.fill_rect(rect, &paint, transform, clip_stack.last());
                }
            }
            DrawCommand::FillRect { rect, color } => {
                if rect.intersection(dirty_bounds).is_none() {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let paint = solid_paint(*color);
                pixmap.fill_rect(rect, &paint, transform, clip_stack.last());
            }
            DrawCommand::FillRoundedRect {
                rect,
                radius,
                color,
            } => {
                if rect.intersection(dirty_bounds).is_none() {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = rounded_rect_path(rect, *radius);
                let paint = solid_paint(*color);
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    transform,
                    clip_stack.last(),
                );
            }
            DrawCommand::FillEllipse { rect, color } => {
                if rect.intersection(dirty_bounds).is_none() {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = ellipse_path(rect);
                let paint = solid_paint(*color);
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    transform,
                    clip_stack.last(),
                );
            }
            DrawCommand::StrokeRect {
                rect,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                if rect
                    .expanded(*stroke_width * 0.5 + 1.0)
                    .intersection(dirty_bounds)
                    .is_none()
                {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = PathBuilder::from_rect(rect);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
            }
            DrawCommand::StrokeRoundedRect {
                rect,
                radius,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                if rect
                    .expanded(*stroke_width * 0.5 + 1.0)
                    .intersection(dirty_bounds)
                    .is_none()
                {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = rounded_rect_path(rect, *radius);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
            }
            DrawCommand::StrokeEllipse {
                rect,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                if rect
                    .expanded(*stroke_width * 0.5 + 1.0)
                    .intersection(dirty_bounds)
                    .is_none()
                {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = ellipse_path(rect);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
            }
            DrawCommand::PushClip { rect } => {
                let mask = create_clip_mask(*rect, clip_stack.last(), width, height, transform)?;
                clip_stack.push(mask);
            }
            DrawCommand::PushRoundedClip { rect, radius } => {
                let mask = create_rounded_clip_mask(
                    *rect,
                    *radius,
                    clip_stack.last(),
                    width,
                    height,
                    transform,
                )?;
                clip_stack.push(mask);
            }
            DrawCommand::PopClip => {
                if clip_stack.len() > 1 {
                    clip_stack.pop();
                }
            }
            DrawCommand::DrawText { command } => {
                if command.bounds.intersection(dirty_bounds).is_none() {
                    continue;
                }
                draw_text_command(
                    &mut *pixmap,
                    font_system,
                    swash_cache,
                    text_layout_cache,
                    command,
                    scale,
                    clip_stack.last(),
                );
            }
            DrawCommand::DrawSvg { command } => {
                if command.bounds.intersection(dirty_bounds).is_none() {
                    continue;
                }
                draw_svg_command(pixmap, command, scale, clip_stack.last())?;
            }
            DrawCommand::DrawImage { command } => {
                if command.bounds.intersection(dirty_bounds).is_none() {
                    continue;
                }
                draw_image_command(pixmap, command, scale, clip_stack.last())?;
            }
        }
    }

    Ok(clear_color)
}

fn reusable_pixmap(
    pixmap: &mut Option<Pixmap>,
    width: u32,
    height: u32,
) -> Result<&mut Pixmap, MochiOsBackendError> {
    let needs_allocate = pixmap
        .as_ref()
        .is_none_or(|current| current.width() != width || current.height() != height);
    if needs_allocate {
        *pixmap = Some(Pixmap::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?);
    }
    pixmap
        .as_mut()
        .ok_or(MochiOsBackendError::InvalidWindowSize)
}

fn valid_scale_factor(scale_factor: f64) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor as f32
    } else {
        1.0
    }
}

fn draw_image_command(
    target: &mut Pixmap,
    command: &ImageCommand,
    display_scale: f32,
    clip: Option<&Mask>,
) -> Result<(), MochiOsBackendError> {
    let bounds = command.bounds;
    if !is_valid_image_bounds(bounds) {
        return Ok(());
    }

    let image_width = command.image.width();
    let image_height = command.image.height();
    if image_width == 0 || image_height == 0 {
        return Ok(());
    }

    let Some(source) = PixmapRef::from_bytes(
        command.image.premultiplied_rgba8(),
        image_width,
        image_height,
    ) else {
        return Ok(());
    };

    let destination_width = bounds.size.width * display_scale;
    let destination_height = bounds.size.height * display_scale;
    let translate_x = bounds.origin.x * display_scale;
    let translate_y = bounds.origin.y * display_scale;
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || !translate_x.is_finite()
        || !translate_y.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return Ok(());
    }

    let scale_x = destination_width / image_width as f32;
    let scale_y = destination_height / image_height as f32;
    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
        return Ok(());
    }

    let transform = Transform::from_row(scale_x, 0.0, 0.0, scale_y, translate_x, translate_y);
    let paint = PixmapPaint {
        opacity: sanitize_image_opacity(command.opacity),
        quality: image_filter_quality(command.sampling),
        ..PixmapPaint::default()
    };
    target.draw_pixmap(0, 0, source, &paint, transform, clip);
    Ok(())
}

fn image_filter_quality(sampling: ImageSampling) -> FilterQuality {
    match sampling {
        ImageSampling::Nearest => FilterQuality::Nearest,
        ImageSampling::Bilinear => FilterQuality::Bilinear,
        ImageSampling::Bicubic => FilterQuality::Bicubic,
    }
}

fn draw_svg_command(
    target: &mut Pixmap,
    command: &SvgCommand,
    display_scale: f32,
    clip: Option<&Mask>,
) -> Result<(), MochiOsBackendError> {
    let bounds = command.bounds;
    if !is_valid_image_bounds(bounds) {
        return Ok(());
    }

    let svg_width = command.svg.width();
    let svg_height = command.svg.height();
    if !svg_width.is_finite() || !svg_height.is_finite() || svg_width <= 0.0 || svg_height <= 0.0 {
        return Ok(());
    }

    let destination_width = bounds.size.width * display_scale;
    let destination_height = bounds.size.height * display_scale;
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return Ok(());
    }

    let raster_width = destination_width.ceil() as u32;
    let raster_height = destination_height.ceil() as u32;
    if raster_width == 0 || raster_height == 0 {
        return Ok(());
    }

    let mut raster =
        Pixmap::new(raster_width, raster_height).ok_or(MochiOsBackendError::InvalidWindowSize)?;
    let render_transform = Transform::from_scale(
        raster_width as f32 / svg_width,
        raster_height as f32 / svg_height,
    );
    resvg::render(command.svg.tree(), render_transform, &mut raster.as_mut());

    if let Some(tint) = command.tint {
        tint_svg_pixmap(&mut raster, tint);
    }

    let translate_x = bounds.origin.x * display_scale;
    let translate_y = bounds.origin.y * display_scale;
    if !translate_x.is_finite() || !translate_y.is_finite() {
        return Ok(());
    }

    let paint = PixmapPaint {
        opacity: sanitize_image_opacity(command.opacity),
        quality: FilterQuality::Bicubic,
        ..PixmapPaint::default()
    };
    target.draw_pixmap(
        translate_x.round() as i32,
        translate_y.round() as i32,
        raster.as_ref(),
        &paint,
        Transform::identity(),
        clip,
    );

    Ok(())
}

fn svg_supersample_scale(destination_width: f32, destination_height: f32) -> f32 {
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return 1.0;
    }

    if destination_width.max(destination_height) <= SVG_SMALL_RENDER_LIMIT {
        SVG_SMALL_RENDER_SUPERSAMPLE
    } else {
        1.0
    }
}

fn is_valid_image_bounds(bounds: Rect) -> bool {
    bounds.origin.x.is_finite()
        && bounds.origin.y.is_finite()
        && bounds.size.width.is_finite()
        && bounds.size.height.is_finite()
        && bounds.size.width > 0.0
        && bounds.size.height > 0.0
}

fn sanitize_image_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn tint_svg_pixmap(pixmap: &mut Pixmap, tint: Color) {
    for pixel in pixmap.data_mut().chunks_exact_mut(4) {
        let alpha = multiply_channel(pixel[3], tint.alpha);

        pixel[0] = multiply_channel(tint.red, alpha);
        pixel[1] = multiply_channel(tint.green, alpha);
        pixel[2] = multiply_channel(tint.blue, alpha);
        pixel[3] = alpha;
    }
}

fn multiply_channel(first: u8, second: u8) -> u8 {
    let value = u16::from(first) * u16::from(second);

    ((value + 127) / 255) as u8
}

fn draw_text_command(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    command: &TextCommand,
    scale: f32,
    clip: Option<&Mask>,
) {
    if command.text.is_empty()
        || command.bounds.size.width <= 0.0
        || command.bounds.size.height <= 0.0
    {
        return;
    }

    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    let font_size = (command.font_size * scale).max(1.0);
    let line_height = (command.line_height * scale).max(font_size);
    let width = (command.bounds.size.width * scale).max(0.0);
    let height = (command.bounds.size.height * scale).max(0.0);
    let origin_x = (command.bounds.origin.x * scale).round();
    let origin_y = command.bounds.origin.y * scale;
    let key = TextLayoutKey::new(command, scale);

    if !layout_cache.contains_key(&key) {
        if layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            layout_cache.clear();
        }

        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(font_system, metrics);
        {
            let mut buffer_with_font_system = buffer.borrow_with(font_system);
            buffer_with_font_system.set_size(Some(width), Some(height));

            let attrs = Attrs::new()
                .family(Family::Name(command.font_family.as_str()))
                .weight(Weight(command.weight.clamp(1, 1000)));

            buffer_with_font_system.set_text(
                command.text.as_str(),
                &attrs,
                Shaping::Advanced,
                command.alignment.to_cosmic(),
            );
        }

        layout_cache.insert(key.clone(), buffer);
    }

    let Some(buffer) = layout_cache.get_mut(&key) else {
        return;
    };
    let mut buffer = buffer.borrow_with(font_system);
    let text_color = CosmicColor::rgba(
        command.color.red,
        command.color.green,
        command.color.blue,
        command.color.alpha,
    );
    let Some(text_clip) = SkiaRect::from_xywh(origin_x, origin_y, width, height) else {
        return;
    };

    let mut physical_glyphs = Vec::new();
    for run in buffer.layout_runs() {
        let baseline_y = (origin_y + run.line_y).round();
        for glyph in run.glyphs {
            physical_glyphs.push(glyph.physical((origin_x, baseline_y), 1.0));
        }
    }
    drop(buffer);

    for physical_glyph in physical_glyphs {
        swash_cache.with_pixels(
            font_system,
            physical_glyph.cache_key,
            text_color,
            |x, y, color| {
                let draw_x = physical_glyph.x + x;
                let draw_y = physical_glyph.y + y;
                let Some(pixel_rect) = SkiaRect::from_xywh(draw_x as f32, draw_y as f32, 1.0, 1.0)
                else {
                    return;
                };
                let Some(rect) = intersect_rect(pixel_rect, text_clip) else {
                    return;
                };
                let (red, green, blue, alpha) = color.as_rgba_tuple();
                if alpha == 0 {
                    return;
                }
                let mut paint = Paint::default();
                paint.set_color_rgba8(red, green, blue, alpha);
                paint.anti_alias = false;
                pixmap.fill_rect(rect, &paint, Transform::identity(), clip);
            },
        );
    }
}

fn intersect_rect(first: SkiaRect, second: SkiaRect) -> Option<SkiaRect> {
    let left = first.left().max(second.left());
    let top = first.top().max(second.top());
    let right = first.right().min(second.right());
    let bottom = first.bottom().min(second.bottom());
    if right <= left || bottom <= top {
        return None;
    }
    SkiaRect::from_xywh(left, top, right - left, bottom - top)
}

fn to_skia_rect(rect: Rect) -> Option<SkiaRect> {
    if !rect.origin.x.is_finite()
        || !rect.origin.y.is_finite()
        || !rect.size.width.is_finite()
        || !rect.size.height.is_finite()
        || rect.size.width < 0.0
        || rect.size.height < 0.0
    {
        return None;
    }
    SkiaRect::from_xywh(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    )
}

fn rounded_rect_path(rect: SkiaRect, radius: f32) -> Path {
    let radius = if radius.is_finite() {
        radius.max(0.0).min(rect.width().min(rect.height()) / 2.0)
    } else {
        0.0
    };
    if radius == 0.0 {
        return PathBuilder::from_rect(rect);
    }

    let left = rect.left();
    let top = rect.top();
    let right = rect.right();
    let bottom = rect.bottom();
    let mut builder = PathBuilder::new();
    builder.move_to(left + radius, top);
    builder.line_to(right - radius, top);
    builder.quad_to(right, top, right, top + radius);
    builder.line_to(right, bottom - radius);
    builder.quad_to(right, bottom, right - radius, bottom);
    builder.line_to(left + radius, bottom);
    builder.quad_to(left, bottom, left, bottom - radius);
    builder.line_to(left, top + radius);
    builder.quad_to(left, top, left + radius, top);
    builder.close();
    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn ellipse_path(rect: SkiaRect) -> Path {
    const KAPPA: f32 = 0.552_284_8;

    let center_x = (rect.left() + rect.right()) / 2.0;
    let center_y = (rect.top() + rect.bottom()) / 2.0;
    let radius_x = rect.width() / 2.0;
    let radius_y = rect.height() / 2.0;
    let control_x = radius_x * KAPPA;
    let control_y = radius_y * KAPPA;

    let mut builder = PathBuilder::new();
    builder.move_to(center_x + radius_x, center_y);
    builder.cubic_to(
        center_x + radius_x,
        center_y + control_y,
        center_x + control_x,
        center_y + radius_y,
        center_x,
        center_y + radius_y,
    );
    builder.cubic_to(
        center_x - control_x,
        center_y + radius_y,
        center_x - radius_x,
        center_y + control_y,
        center_x - radius_x,
        center_y,
    );
    builder.cubic_to(
        center_x - radius_x,
        center_y - control_y,
        center_x - control_x,
        center_y - radius_y,
        center_x,
        center_y - radius_y,
    );
    builder.cubic_to(
        center_x + control_x,
        center_y - radius_y,
        center_x + radius_x,
        center_y - control_y,
        center_x + radius_x,
        center_y,
    );
    builder.close();
    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn create_clip_mask(
    rect: Rect,
    previous: Option<&Mask>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<Mask, MochiOsBackendError> {
    let path = to_skia_rect(rect).map(PathBuilder::from_rect);
    create_path_clip_mask(path, previous, width, height, transform)
}

fn create_rounded_clip_mask(
    rect: Rect,
    radius: f32,
    previous: Option<&Mask>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<Mask, MochiOsBackendError> {
    let path = to_skia_rect(rect).map(|rect| rounded_rect_path(rect, radius));
    create_path_clip_mask(path, previous, width, height, transform)
}

fn create_path_clip_mask(
    path: Option<Path>,
    previous: Option<&Mask>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<Mask, MochiOsBackendError> {
    let has_previous = previous.is_some();
    let mut mask = match previous {
        Some(previous) => previous.clone(),
        None => Mask::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?,
    };

    let Some(path) = path else {
        mask.clear();
        return Ok(mask);
    };

    if has_previous {
        mask.intersect_path(&path, FillRule::Winding, true, transform);
    } else {
        mask.clear();
        mask.fill_path(&path, FillRule::Winding, true, transform);
    }

    Ok(mask)
}

fn solid_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);
    paint.anti_alias = true;
    paint
}
