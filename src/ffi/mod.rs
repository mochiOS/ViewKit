//! KomeからViewKit Runtimeを操作するためのC関数

use crate::components::{
    BorderStyle, ButtonColor, ImageContentMode, RectangleColor, ScrollAxis, ScrollBarVisibility,
    SvgContentMode, TextFieldSize, ZStackAlignment,
};
use crate::draw_command::{DisplayList, DrawCommand, ImageSampling};
use crate::event::{EventContext, EventDispatcher};
use crate::geometry::{Rect, Size};
use crate::layout::{LayoutLength, StackAlignment, StackDistribution, StackGap};
use crate::platform::linux::LinuxBackend;
use crate::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use crate::renderer::Viewport;
use crate::theme::{Color, CornerRadius, Theme};
use crate::typography::{TextAlignment, TextMeasurer, Typography};
use crate::view::{PaintContext, RedrawSchedule, View};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::rc::Rc;
use std::slice;
use std::str;
use std::time::Instant;

mod generated_components;
mod tree;

use crate::ffi::tree::{
    FfiBuildContext, FfiStateStore, FfiTreeBuilder, FfiTreeBuilderError, SharedActionQueue,
    SharedStateStore,
};
use crate::image::ImageData;
use crate::svg::SvgData;
pub use generated_components::*;

pub const VK_Z_ALIGNMENT_TOP_LEADING: u32 = 0;
pub const VK_Z_ALIGNMENT_TOP: u32 = 1;
pub const VK_Z_ALIGNMENT_TOP_TRAILING: u32 = 2;
pub const VK_Z_ALIGNMENT_LEADING: u32 = 3;
pub const VK_Z_ALIGNMENT_CENTER: u32 = 4;
pub const VK_Z_ALIGNMENT_TRAILING: u32 = 5;
pub const VK_Z_ALIGNMENT_BOTTOM_LEADING: u32 = 6;
pub const VK_Z_ALIGNMENT_BOTTOM: u32 = 7;
pub const VK_Z_ALIGNMENT_BOTTOM_TRAILING: u32 = 8;

pub const VK_ABI_VERSION_MAJOR: u32 = 1;
pub const VK_ABI_VERSION_MINOR: u32 = 0;
pub const VK_ABI_VERSION_PATCH: u32 = 0;

pub const VK_IMAGE_CONTENT_MODE_FIT: u32 = 0;
pub const VK_IMAGE_CONTENT_MODE_FILL: u32 = 1;
pub const VK_IMAGE_CONTENT_MODE_STRETCH: u32 = 2;

pub const VK_IMAGE_SAMPLING_NEAREST: u32 = 0;
pub const VK_IMAGE_SAMPLING_BILINEAR: u32 = 1;
pub const VK_IMAGE_SAMPLING_BICUBIC: u32 = 2;

pub const VK_SVG_CONTENT_MODE_FIT: u32 = 0;
pub const VK_SVG_CONTENT_MODE_FILL: u32 = 1;
pub const VK_SVG_CONTENT_MODE_STRETCH: u32 = 2;

/*
 * 0x00MMmmpp
 *
 * MM: major
 * mm: minor
 * pp: patch
 *
 * 1.0.0は0x00010000になります。
 */
pub const VK_ABI_VERSION: u32 =
    (VK_ABI_VERSION_MAJOR << 16) | (VK_ABI_VERSION_MINOR << 8) | VK_ABI_VERSION_PATCH;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VkStatus {
    Ok = 0,

    NullPointer = 1,
    InvalidUtf8 = 2,

    BuilderAlreadyActive = 3,
    NoActiveBuilder = 4,

    NoOpenNode = 5,
    UnclosedNodes = 6,
    MultipleRoots = 7,
    MissingRoot = 8,

    InvalidEnumValue = 9,
    UnsupportedEvent = 10,

    PlatformError = 11,
    UnsupportedPlatform = 12,

    InvalidChildCount = 13,
    InvalidTreeNode = 14,

    StateNotFound = 15,
    StateTypeMismatch = 16,
    BufferTooSmall = 17,
    InvalidValue = 18,

    Panic = 255,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkString {
    pub pointer: *const u8,
    pub length: usize,
}

impl VkString {
    pub fn from_str(value: &str) -> Self {
        Self {
            pointer: value.as_ptr(),

            length: value.len(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkBytes {
    pub pointer: *const u8,
    pub length: usize,
}

impl Default for VkBytes {
    fn default() -> Self {
        Self {
            pointer: ptr::null(),
            length: 0,
        }
    }
}

pub const VK_LENGTH_AUTO: u32 = 0;

pub const VK_LENGTH_FIXED: u32 = 1;

pub const VK_RECTANGLE_COLOR_BACKGROUND: u32 = 0;
pub const VK_RECTANGLE_COLOR_SURFACE: u32 = 1;
pub const VK_RECTANGLE_COLOR_ELEVATED_SURFACE: u32 = 2;
pub const VK_RECTANGLE_COLOR_ACCENT: u32 = 3;
pub const VK_RECTANGLE_COLOR_DESTRUCTIVE: u32 = 4;
pub const VK_RECTANGLE_COLOR_CUSTOM: u32 = 5;

pub const VK_CORNER_RADIUS_NONE: u32 = 0;
pub const VK_CORNER_RADIUS_SMALL: u32 = 1;
pub const VK_CORNER_RADIUS_MEDIUM: u32 = 2;
pub const VK_CORNER_RADIUS_LARGE: u32 = 3;
pub const VK_CORNER_RADIUS_EXTRA_LARGE: u32 = 4;
pub const VK_CORNER_RADIUS_CARD: u32 = 5;
pub const VK_CORNER_RADIUS_FULL: u32 = 6;
pub const VK_CORNER_RADIUS_CUSTOM: u32 = 7;

pub const VK_BORDER_NONE: u32 = 0;
pub const VK_BORDER_STANDARD: u32 = 1;
pub const VK_BORDER_STRONG: u32 = 2;
pub const VK_BORDER_CUSTOM: u32 = 3;

pub const VK_SCROLL_AXIS_HORIZONTAL: u32 = 0;
pub const VK_SCROLL_AXIS_VERTICAL: u32 = 1;
pub const VK_SCROLL_AXIS_BOTH: u32 = 2;

pub const VK_SCROLLBAR_HIDDEN: u32 = 0;
pub const VK_SCROLLBAR_AUTOMATIC: u32 = 1;
pub const VK_SCROLLBAR_ALWAYS: u32 = 2;

pub const VK_TEXT_FIELD_SIZE_SMALL: u32 = 0;
pub const VK_TEXT_FIELD_SIZE_MEDIUM: u32 = 1;
pub const VK_TEXT_FIELD_SIZE_LARGE: u32 = 2;

pub const VK_MENU_ENTRY_ITEM: u32 = 0;
pub const VK_MENU_ENTRY_SEPARATOR: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkSegmentedItem {
    pub value: u64,
    pub label: VkString,
    pub enabled: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkSegmentedItems {
    pub pointer: *const VkSegmentedItem,
    pub length: usize,
}

impl Default for VkSegmentedItems {
    fn default() -> Self {
        Self {
            pointer: ptr::null(),
            length: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkMenuEntry {
    pub kind: u32,

    pub label: VkString,
    pub shortcut: VkString,

    pub enabled: u8,
    pub danger: u8,

    pub action_id: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkMenuEntries {
    pub pointer: *const VkMenuEntry,
    pub length: usize,
}

impl Default for VkMenuEntries {
    fn default() -> Self {
        Self {
            pointer: ptr::null(),
            length: 0,
        }
    }
}

pub(crate) struct DecodedSegmentedItem {
    value: usize,
    label: String,
    enabled: bool,
}

pub(crate) enum DecodedMenuEntry {
    Item {
        label: String,
        shortcut: Option<String>,

        enabled: bool,
        danger: bool,

        action_id: u64,
    },

    Separator,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl VkColor {
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkRectangleStyle {
    pub color_kind: u32,
    pub custom_color: VkColor,

    pub radius_kind: u32,
    pub radius: f32,

    pub border_kind: u32,
    pub border_color: VkColor,
    pub border_width: f32,
}

impl Default for VkRectangleStyle {
    fn default() -> Self {
        Self {
            color_kind: VK_RECTANGLE_COLOR_SURFACE,
            custom_color: VkColor::transparent(),

            radius_kind: VK_CORNER_RADIUS_NONE,
            radius: 0.0,

            border_kind: VK_BORDER_NONE,
            border_color: VkColor::transparent(),
            border_width: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkLength {
    pub kind: u32,
    pub value: f32,
}

impl VkLength {
    pub const fn auto() -> Self {
        Self {
            kind: VK_LENGTH_AUTO,
            value: 0.0,
        }
    }

    pub const fn fixed(value: f32) -> Self {
        Self {
            kind: VK_LENGTH_FIXED,
            value,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkActionEvent {
    pub component_instance_id: u64,
    pub node_id: u64,
    pub action_id: u64,
    pub event_kind: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct DecodedRectangleStyle {
    color: RectangleColor,
    radius: CornerRadius,
    border: BorderStyle,
}

pub const VK_EVENT_BUTTON_CLICKED: u32 = 1;

pub const VK_STACK_GAP_NONE: u32 = 0;

pub const VK_STACK_GAP_EXTRA_SMALL: u32 = 1;

pub const VK_STACK_GAP_SMALL: u32 = 2;

pub const VK_STACK_GAP_MEDIUM: u32 = 3;

pub const VK_STACK_GAP_LARGE: u32 = 4;

pub const VK_STACK_GAP_EXTRA_LARGE: u32 = 5;

pub const VK_STACK_GAP_DOUBLE_EXTRA_LARGE: u32 = 6;

pub const VK_ALIGNMENT_START: u32 = 0;

pub const VK_ALIGNMENT_CENTER: u32 = 1;

pub const VK_ALIGNMENT_END: u32 = 2;

pub const VK_ALIGNMENT_STRETCH: u32 = 3;

pub const VK_DISTRIBUTION_START: u32 = 0;

pub const VK_DISTRIBUTION_CENTER: u32 = 1;

pub const VK_DISTRIBUTION_END: u32 = 2;

pub const VK_DISTRIBUTION_SPACE_BETWEEN: u32 = 3;

pub const VK_DISTRIBUTION_SPACE_AROUND: u32 = 4;

pub const VK_DISTRIBUTION_SPACE_EVENLY: u32 = 5;

pub const VK_TEXT_ALIGNMENT_START: u32 = 0;

pub const VK_TEXT_ALIGNMENT_CENTER: u32 = 1;

pub const VK_TEXT_ALIGNMENT_END: u32 = 2;

pub const VK_TEXT_ALIGNMENT_JUSTIFIED: u32 = 3;

pub const VK_TEXT_COLOR_BLACK: u32 = 0;

pub const VK_TEXT_COLOR_WHITE: u32 = 1;

pub const VK_BUTTON_COLOR_ACCENT: u32 = 0;

pub const VK_BUTTON_COLOR_DESTRUCTIVE: u32 = 1;

pub struct VkRuntime {
    component_instance_id: u64,

    root: Option<Box<dyn View>>,

    builder: Option<FfiTreeBuilder>,

    actions: SharedActionQueue,
    states: SharedStateStore,
}

impl VkRuntime {
    fn new(component_instance_id: u64) -> Self {
        Self {
            component_instance_id,
            root: None,
            builder: None,
            actions: Rc::new(RefCell::new(VecDeque::new())),
            states: Rc::new(RefCell::new(FfiStateStore::default())),
        }
    }
}

struct VkWindowApplication<'a> {
    runtime: &'a mut VkRuntime,

    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,

    event_dispatcher: EventDispatcher,
    redraw_schedule: RedrawSchedule,
}

impl<'a> VkWindowApplication<'a> {
    fn new(runtime: &'a mut VkRuntime) -> Self {
        Self {
            runtime,

            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),

            redraw_schedule: RedrawSchedule::new(),
        }
    }
}

impl PlatformApplication for VkWindowApplication<'_> {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
        match &event {
            PlatformEvent::Resumed { .. }
            | PlatformEvent::Resized { .. }
            | PlatformEvent::ScaleFactorChanged { .. } => {
                window.request_redraw();
                return;
            }

            PlatformEvent::RedrawRequested | PlatformEvent::CloseRequested => {
                return;
            }

            _ => {}
        }

        let Some(root) = self.runtime.root.as_ref() else {
            return;
        };

        let bounds = window.viewport().logical_bounds();

        let redraw_request = {
            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher
                .dispatch(root.as_ref(), bounds, &event, &mut context);

            context.redraw_request()
        };

        if redraw_request.is_requested() {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) -> Rect {
        let bounds = viewport.logical_bounds();

        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();

        if let Some(root) = self.runtime.root.as_ref() {
            let mut context = PaintContext::new(
                display_list,
                &self.theme,
                &self.typography,
                &mut self.text_measurer,
            )
            .with_redraw_schedule(&mut self.redraw_schedule);

            root.paint(bounds, &mut context);
        }

        bounds
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        self.redraw_schedule.deadline()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_create(component_instance_id: u64) -> *mut VkRuntime {
    catch_unwind(AssertUnwindSafe(|| {
        Box::into_raw(Box::new(VkRuntime::new(component_instance_id)))
    }))
    .unwrap_or_else(|_| ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_abi_version() -> u32 {
    VK_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_status_name(status: i32) -> VkString {
    VkString::from_str(status_name(status))
}

fn status_name(status: i32) -> &'static str {
    match status {
        value if value == VkStatus::Ok as i32 => "ok",

        value if value == VkStatus::NullPointer as i32 => "null_pointer",

        value if value == VkStatus::InvalidUtf8 as i32 => "invalid_utf8",

        value if value == VkStatus::BuilderAlreadyActive as i32 => "builder_already_active",

        value if value == VkStatus::NoActiveBuilder as i32 => "no_active_builder",

        value if value == VkStatus::NoOpenNode as i32 => "no_open_node",

        value if value == VkStatus::UnclosedNodes as i32 => "unclosed_nodes",

        value if value == VkStatus::MultipleRoots as i32 => "multiple_roots",

        value if value == VkStatus::MissingRoot as i32 => "missing_root",

        value if value == VkStatus::InvalidEnumValue as i32 => "invalid_enum_value",

        value if value == VkStatus::UnsupportedEvent as i32 => "unsupported_event",

        value if value == VkStatus::Panic as i32 => "panic",

        value if value == VkStatus::InvalidChildCount as i32 => "invalid_child_count",

        value if value == VkStatus::InvalidTreeNode as i32 => "invalid_tree_node",

        value if value == VkStatus::PlatformError as i32 => "platform_error",

        value if value == VkStatus::UnsupportedPlatform as i32 => "unsupported_platform",

        value if value == VkStatus::StateNotFound as i32 => "state_not_found",

        value if value == VkStatus::StateTypeMismatch as i32 => "state_type_mismatch",

        value if value == VkStatus::BufferTooSmall as i32 => "buffer_too_small",

        value if value == VkStatus::InvalidValue as i32 => "invalid_value",

        _ => "unknown_status",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_destroy(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        if runtime.is_null() {
            return Ok(());
        }

        unsafe {
            drop(Box::from_raw(runtime));
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_tree_begin(runtime: *mut VkRuntime, root_node_id: u64) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        if runtime.builder.is_some() {
            return Err(VkStatus::BuilderAlreadyActive);
        }

        runtime.builder = Some(FfiTreeBuilder::new(root_node_id));

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_tree_abort(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        runtime.builder = None;

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_end_node(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        let builder = active_builder(runtime)?;

        builder.end().map_err(map_builder_error)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_tree_commit(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        let mut builder = runtime.builder.take().ok_or(VkStatus::NoActiveBuilder)?;

        builder.end().map_err(map_builder_error)?;

        let tree = builder.finish().map_err(map_builder_error)?;

        runtime.actions.borrow_mut().clear();

        let mut context = FfiBuildContext::new(
            runtime.component_instance_id,
            Rc::clone(&runtime.actions),
            Rc::clone(&runtime.states),
        );

        let root = tree.build(&mut context)?;

        context.retain_active_states();

        runtime.root = Some(root);

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_collect_actions(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        runtime_mut(runtime)?;

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_poll_action(
    runtime: *mut VkRuntime,
    output: *mut VkActionEvent,
    has_action: *mut u8,
) -> i32 {
    ffi_status(|| {
        if output.is_null() || has_action.is_null() {
            return Err(VkStatus::NullPointer);
        }

        unsafe {
            *has_action = 0;
        }

        let runtime = runtime_mut(runtime)?;

        let action = runtime.actions.borrow_mut().pop_front();

        let Some(action) = action else {
            return Ok(());
        };

        unsafe {
            *output = action;
            *has_action = 1;
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_get_bool(
    runtime: *mut VkRuntime,
    state_id: u64,
    output: *mut u8,
) -> i32 {
    ffi_status(|| {
        if output.is_null() {
            return Err(VkStatus::NullPointer);
        }

        let runtime = runtime_mut(runtime)?;

        let value = runtime.states.borrow().get_bool(state_id)?;

        unsafe {
            *output = u8::from(value);
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_set_bool(runtime: *mut VkRuntime, state_id: u64, value: u8) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        runtime.states.borrow_mut().set_bool(state_id, value != 0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_get_f32(
    runtime: *mut VkRuntime,
    state_id: u64,
    output: *mut f32,
) -> i32 {
    ffi_status(|| {
        if output.is_null() {
            return Err(VkStatus::NullPointer);
        }

        let runtime = runtime_mut(runtime)?;

        let value = runtime.states.borrow().get_float(state_id)?;

        unsafe {
            *output = value;
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_set_f32(runtime: *mut VkRuntime, state_id: u64, value: f32) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        runtime.states.borrow_mut().set_float(state_id, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_get_u64(
    runtime: *mut VkRuntime,
    state_id: u64,
    output: *mut u64,
) -> i32 {
    ffi_status(|| {
        if output.is_null() {
            return Err(VkStatus::NullPointer);
        }

        let runtime = runtime_mut(runtime)?;

        let value = runtime.states.borrow().get_usize(state_id)?;

        let value = u64::try_from(value).map_err(|_| VkStatus::InvalidValue)?;

        unsafe {
            *output = value;
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_set_u64(runtime: *mut VkRuntime, state_id: u64, value: u64) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        let value = usize::try_from(value).map_err(|_| VkStatus::InvalidValue)?;

        runtime.states.borrow_mut().set_usize(state_id, value)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_string_length(
    runtime: *mut VkRuntime,
    state_id: u64,
    output: *mut usize,
) -> i32 {
    ffi_status(|| {
        if output.is_null() {
            return Err(VkStatus::NullPointer);
        }

        let runtime = runtime_mut(runtime)?;

        let value = runtime.states.borrow().get_string(state_id)?;

        unsafe {
            *output = value.len();
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_copy_string(
    runtime: *mut VkRuntime,
    state_id: u64,
    buffer: *mut u8,
    buffer_length: usize,
    output_length: *mut usize,
) -> i32 {
    ffi_status(|| {
        if output_length.is_null() {
            return Err(VkStatus::NullPointer);
        }

        let runtime = runtime_mut(runtime)?;

        let value = runtime.states.borrow().get_string(state_id)?;

        let bytes = value.as_bytes();
        let required_length = bytes.len();

        unsafe {
            *output_length = required_length;
        }

        if required_length == 0 {
            return Ok(());
        }

        if buffer.is_null() {
            return Err(VkStatus::NullPointer);
        }

        if buffer_length < required_length {
            return Err(VkStatus::BufferTooSmall);
        }

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, required_length);
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_state_set_string(
    runtime: *mut VkRuntime,
    state_id: u64,
    value: VkString,
) -> i32 {
    ffi_status(|| {
        let value = copy_string(value)?;
        let runtime = runtime_mut(runtime)?;

        runtime.states.borrow_mut().set_string(state_id, value)
    })
}

fn runtime_mut<'a>(runtime: *mut VkRuntime) -> Result<&'a mut VkRuntime, VkStatus> {
    if runtime.is_null() {
        return Err(VkStatus::NullPointer);
    }

    Ok(unsafe { &mut *runtime })
}

fn active_builder(runtime: &mut VkRuntime) -> Result<&mut FfiTreeBuilder, VkStatus> {
    runtime.builder.as_mut().ok_or(VkStatus::NoActiveBuilder)
}

fn copy_string(value: VkString) -> Result<String, VkStatus> {
    if value.length == 0 {
        return Ok(String::new());
    }

    if value.pointer.is_null() {
        return Err(VkStatus::NullPointer);
    }

    let bytes = unsafe { slice::from_raw_parts(value.pointer, value.length) };

    let value = str::from_utf8(bytes).map_err(|_| VkStatus::InvalidUtf8)?;

    Ok(value.to_owned())
}

fn decode_stack_gap(value: u32) -> Result<StackGap, VkStatus> {
    match value {
        VK_STACK_GAP_NONE => Ok(StackGap::None),

        VK_STACK_GAP_EXTRA_SMALL => Ok(StackGap::ExtraSmall),

        VK_STACK_GAP_SMALL => Ok(StackGap::Small),

        VK_STACK_GAP_MEDIUM => Ok(StackGap::Medium),

        VK_STACK_GAP_LARGE => Ok(StackGap::Large),

        VK_STACK_GAP_EXTRA_LARGE => Ok(StackGap::ExtraLarge),

        VK_STACK_GAP_DOUBLE_EXTRA_LARGE => Ok(StackGap::DoubleExtraLarge),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_stack_alignment(value: u32) -> Result<StackAlignment, VkStatus> {
    match value {
        VK_ALIGNMENT_START => Ok(StackAlignment::Start),

        VK_ALIGNMENT_CENTER => Ok(StackAlignment::Center),

        VK_ALIGNMENT_END => Ok(StackAlignment::End),

        VK_ALIGNMENT_STRETCH => Ok(StackAlignment::Stretch),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_stack_distribution(value: u32) -> Result<StackDistribution, VkStatus> {
    match value {
        VK_DISTRIBUTION_START => Ok(StackDistribution::Start),

        VK_DISTRIBUTION_CENTER => Ok(StackDistribution::Center),

        VK_DISTRIBUTION_END => Ok(StackDistribution::End),

        VK_DISTRIBUTION_SPACE_BETWEEN => Ok(StackDistribution::SpaceBetween),

        VK_DISTRIBUTION_SPACE_AROUND => Ok(StackDistribution::SpaceAround),

        VK_DISTRIBUTION_SPACE_EVENLY => Ok(StackDistribution::SpaceEvenly),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_text_alignment(value: u32) -> Result<TextAlignment, VkStatus> {
    match value {
        VK_TEXT_ALIGNMENT_START => Ok(TextAlignment::Start),

        VK_TEXT_ALIGNMENT_CENTER => Ok(TextAlignment::Center),

        VK_TEXT_ALIGNMENT_END => Ok(TextAlignment::End),

        VK_TEXT_ALIGNMENT_JUSTIFIED => Ok(TextAlignment::Justified),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_text_color(value: u32) -> Result<Color, VkStatus> {
    match value {
        VK_TEXT_COLOR_BLACK => Ok(Color::BLACK),

        VK_TEXT_COLOR_WHITE => Ok(Color::WHITE),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_button_color(value: u32) -> Result<ButtonColor, VkStatus> {
    match value {
        VK_BUTTON_COLOR_ACCENT => Ok(ButtonColor::Accent),

        VK_BUTTON_COLOR_DESTRUCTIVE => Ok(ButtonColor::Destructive),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_usize(value: u64) -> Result<usize, VkStatus> {
    usize::try_from(value).map_err(|_| VkStatus::InvalidEnumValue)
}

fn decode_scroll_axis(value: u32) -> Result<ScrollAxis, VkStatus> {
    match value {
        VK_SCROLL_AXIS_HORIZONTAL => Ok(ScrollAxis::Horizontal),

        VK_SCROLL_AXIS_VERTICAL => Ok(ScrollAxis::Vertical),

        VK_SCROLL_AXIS_BOTH => Ok(ScrollAxis::Both),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_scrollbar_visibility(value: u32) -> Result<ScrollBarVisibility, VkStatus> {
    match value {
        VK_SCROLLBAR_HIDDEN => Ok(ScrollBarVisibility::Hidden),

        VK_SCROLLBAR_AUTOMATIC => Ok(ScrollBarVisibility::Automatic),

        VK_SCROLLBAR_ALWAYS => Ok(ScrollBarVisibility::Always),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_text_field_size(value: u32) -> Result<TextFieldSize, VkStatus> {
    match value {
        VK_TEXT_FIELD_SIZE_SMALL => Ok(TextFieldSize::Small),

        VK_TEXT_FIELD_SIZE_MEDIUM => Ok(TextFieldSize::Medium),

        VK_TEXT_FIELD_SIZE_LARGE => Ok(TextFieldSize::Large),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn map_builder_error(error: FfiTreeBuilderError) -> VkStatus {
    match error {
        FfiTreeBuilderError::NoOpenNode => VkStatus::NoOpenNode,

        FfiTreeBuilderError::UnclosedNodes => VkStatus::UnclosedNodes,

        FfiTreeBuilderError::MultipleRoots => VkStatus::MultipleRoots,

        FfiTreeBuilderError::MissingRoot => VkStatus::MissingRoot,
    }
}

fn sanitize_length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn copy_segmented_items(items: VkSegmentedItems) -> Result<Vec<DecodedSegmentedItem>, VkStatus> {
    if items.length == 0 {
        return Ok(Vec::new());
    }

    if items.pointer.is_null() {
        return Err(VkStatus::NullPointer);
    }

    let items = unsafe { slice::from_raw_parts(items.pointer, items.length) };

    items
        .iter()
        .map(|item| {
            Ok(DecodedSegmentedItem {
                value: decode_usize(item.value)?,

                label: copy_string(item.label)?,

                enabled: item.enabled != 0,
            })
        })
        .collect()
}

fn copy_menu_entries(entries: VkMenuEntries) -> Result<Vec<DecodedMenuEntry>, VkStatus> {
    if entries.length == 0 {
        return Ok(Vec::new());
    }

    if entries.pointer.is_null() {
        return Err(VkStatus::NullPointer);
    }

    let entries = unsafe { slice::from_raw_parts(entries.pointer, entries.length) };

    entries
        .iter()
        .map(|entry| match entry.kind {
            VK_MENU_ENTRY_ITEM => {
                let label = copy_string(entry.label)?;

                let shortcut = copy_string(entry.shortcut)?;

                Ok(DecodedMenuEntry::Item {
                    label,

                    shortcut: if shortcut.is_empty() {
                        None
                    } else {
                        Some(shortcut)
                    },

                    enabled: entry.enabled != 0,

                    danger: entry.danger != 0,

                    action_id: entry.action_id,
                })
            }

            VK_MENU_ENTRY_SEPARATOR => Ok(DecodedMenuEntry::Separator),

            _ => Err(VkStatus::InvalidEnumValue),
        })
        .collect()
}

fn finite_or_default(value: f32, default: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        default
    }
}

fn ffi_status<F>(operation: F) -> i32
where
    F: FnOnce() -> Result<(), VkStatus>,
{
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => VkStatus::Ok as i32,

        Ok(Err(status)) => status as i32,

        Err(_) => VkStatus::Panic as i32,
    }
}

fn decode_zstack_alignment(value: u32) -> Result<ZStackAlignment, VkStatus> {
    match value {
        VK_Z_ALIGNMENT_TOP_LEADING => Ok(ZStackAlignment::TopLeading),

        VK_Z_ALIGNMENT_TOP => Ok(ZStackAlignment::Top),

        VK_Z_ALIGNMENT_TOP_TRAILING => Ok(ZStackAlignment::TopTrailing),

        VK_Z_ALIGNMENT_LEADING => Ok(ZStackAlignment::Leading),

        VK_Z_ALIGNMENT_CENTER => Ok(ZStackAlignment::Center),

        VK_Z_ALIGNMENT_TRAILING => Ok(ZStackAlignment::Trailing),

        VK_Z_ALIGNMENT_BOTTOM_LEADING => Ok(ZStackAlignment::BottomLeading),

        VK_Z_ALIGNMENT_BOTTOM => Ok(ZStackAlignment::Bottom),

        VK_Z_ALIGNMENT_BOTTOM_TRAILING => Ok(ZStackAlignment::BottomTrailing),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_layout_length(value: VkLength) -> Result<LayoutLength, VkStatus> {
    match value.kind {
        VK_LENGTH_AUTO => Ok(LayoutLength::Auto),

        VK_LENGTH_FIXED => Ok(LayoutLength::Fixed(sanitize_length(value.value))),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_rectangle_style(style: VkRectangleStyle) -> Result<DecodedRectangleStyle, VkStatus> {
    Ok(DecodedRectangleStyle {
        color: decode_rectangle_color(style.color_kind, style.custom_color)?,

        radius: decode_corner_radius(style.radius_kind, style.radius)?,

        border: decode_border_style(style.border_kind, style.border_color, style.border_width)?,
    })
}

fn build_rectangle(style: DecodedRectangleStyle) -> crate::components::Rectangle {
    crate::components::Rectangle::new()
        .color(style.color)
        .radius(style.radius)
        .border(style.border)
}

fn decode_rectangle_color(kind: u32, custom_color: VkColor) -> Result<RectangleColor, VkStatus> {
    match kind {
        VK_RECTANGLE_COLOR_BACKGROUND => Ok(RectangleColor::Background),

        VK_RECTANGLE_COLOR_SURFACE => Ok(RectangleColor::Surface),

        VK_RECTANGLE_COLOR_ELEVATED_SURFACE => Ok(RectangleColor::ElevatedSurface),

        VK_RECTANGLE_COLOR_ACCENT => Ok(RectangleColor::Accent),

        VK_RECTANGLE_COLOR_DESTRUCTIVE => Ok(RectangleColor::Destructive),

        VK_RECTANGLE_COLOR_CUSTOM => Ok(RectangleColor::Custom(decode_color(custom_color))),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_corner_radius(kind: u32, value: f32) -> Result<CornerRadius, VkStatus> {
    match kind {
        VK_CORNER_RADIUS_NONE => Ok(CornerRadius::None),
        VK_CORNER_RADIUS_SMALL => Ok(CornerRadius::Small),
        VK_CORNER_RADIUS_MEDIUM => Ok(CornerRadius::Medium),
        VK_CORNER_RADIUS_LARGE => Ok(CornerRadius::Large),
        VK_CORNER_RADIUS_EXTRA_LARGE => Ok(CornerRadius::ExtraLarge),
        VK_CORNER_RADIUS_CARD => Ok(CornerRadius::Card),
        VK_CORNER_RADIUS_FULL => Ok(CornerRadius::Full),

        VK_CORNER_RADIUS_CUSTOM => Ok(CornerRadius::Custom(sanitize_length(value))),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_border_style(kind: u32, color: VkColor, width: f32) -> Result<BorderStyle, VkStatus> {
    let width = sanitize_length(width);

    match kind {
        VK_BORDER_NONE => Ok(BorderStyle::None),

        VK_BORDER_STANDARD => Ok(BorderStyle::standard(width)),

        VK_BORDER_STRONG => Ok(BorderStyle::strong(width)),

        VK_BORDER_CUSTOM => Ok(BorderStyle::custom(decode_color(color), width)),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_color(color: VkColor) -> Color {
    Color::rgba(color.red, color.green, color.blue, color.alpha)
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_run_window(
    runtime: *mut VkRuntime,
    title: VkString,
    width: f32,
    height: f32,
    resizable: u8,
) -> i32 {
    ffi_status(|| {
        let title = copy_string(title)?;

        let width = finite_or_default(width, 800.0).max(1.0);
        let height = finite_or_default(height, 600.0).max(1.0);

        let runtime = runtime_mut(runtime)?;

        if runtime.builder.is_some() {
            return Err(VkStatus::BuilderAlreadyActive);
        }

        if runtime.root.is_none() {
            return Err(VkStatus::MissingRoot);
        }

        #[cfg(target_os = "linux")]
        {
            let application = VkWindowApplication::new(runtime);

            let backend = LinuxBackend::new(
                application,
                WindowConfig {
                    title,
                    size: Size::new(width, height),
                    resizable: resizable != 0,
                    fullscreen: false,
                },
            );

            backend.run().map_err(|_| VkStatus::PlatformError)?;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = title;
            let _ = width;
            let _ = height;
            let _ = resizable;

            Err(VkStatus::UnsupportedPlatform)
        }
    })
}

fn decode_image_data(value: VkBytes) -> Result<ImageData, VkStatus> {
    if value.length == 0 {
        return Err(VkStatus::InvalidValue);
    }

    if value.pointer.is_null() {
        return Err(VkStatus::NullPointer);
    }

    let bytes = unsafe { slice::from_raw_parts(value.pointer, value.length) };

    ImageData::decode(bytes).map_err(|_| VkStatus::InvalidValue)
}

fn decode_svg_data(value: VkBytes) -> Result<SvgData, VkStatus> {
    if value.length == 0 {
        return Err(VkStatus::InvalidValue);
    }

    if value.pointer.is_null() {
        return Err(VkStatus::NullPointer);
    }

    let bytes = unsafe { slice::from_raw_parts(value.pointer, value.length) };

    SvgData::decode(bytes).map_err(|_| VkStatus::InvalidValue)
}

fn decode_svg_content_mode(value: u32) -> Result<SvgContentMode, VkStatus> {
    match value {
        VK_SVG_CONTENT_MODE_FIT => Ok(SvgContentMode::Fit),

        VK_SVG_CONTENT_MODE_FILL => Ok(SvgContentMode::Fill),

        VK_SVG_CONTENT_MODE_STRETCH => Ok(SvgContentMode::Stretch),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_optional_color(enabled: bool, color: VkColor) -> Option<Color> {
    enabled.then(|| decode_color(color))
}

fn decode_image_content_mode(value: u32) -> Result<ImageContentMode, VkStatus> {
    match value {
        VK_IMAGE_CONTENT_MODE_FIT => Ok(ImageContentMode::Fit),

        VK_IMAGE_CONTENT_MODE_FILL => Ok(ImageContentMode::Fill),

        VK_IMAGE_CONTENT_MODE_STRETCH => Ok(ImageContentMode::Stretch),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_image_sampling(value: u32) -> Result<ImageSampling, VkStatus> {
    match value {
        VK_IMAGE_SAMPLING_NEAREST => Ok(ImageSampling::Nearest),

        VK_IMAGE_SAMPLING_BILINEAR => Ok(ImageSampling::Bilinear),

        VK_IMAGE_SAMPLING_BICUBIC => Ok(ImageSampling::Bicubic),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn sanitize_opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
