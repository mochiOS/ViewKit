//! Flat UTF-8 C entry points used by language bindings.

use super::{VkRuntime, VkString};

unsafe fn borrowed_string(pointer: *const u8, length: usize) -> VkString {
    VkString { pointer, length }
}

/// Sets a string state value from borrowed UTF-8 bytes.
///
/// # Safety
///
/// `pointer` must address `length` readable bytes for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vk_state_set_string_utf8(
    runtime: *mut VkRuntime,
    state_id: u64,
    pointer: *const u8,
    length: usize,
) -> i32 {
    super::vk_state_set_string(runtime, state_id, unsafe {
        borrowed_string(pointer, length)
    })
}

/// Runs a ViewKit window whose title is supplied as borrowed UTF-8 bytes.
///
/// # Safety
///
/// `title_pointer` must address `title_length` readable bytes for the duration
/// of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vk_runtime_run_window_utf8(
    runtime: *mut VkRuntime,
    title_pointer: *const u8,
    title_length: usize,
    width: f32,
    height: f32,
    resizable: u8,
) -> i32 {
    super::vk_runtime_run_window(
        runtime,
        unsafe { borrowed_string(title_pointer, title_length) },
        width,
        height,
        resizable,
    )
}

/// Pushes a text node whose content is supplied as borrowed UTF-8 bytes.
///
/// # Safety
///
/// `content_pointer` must address `content_length` readable bytes for the
/// duration of the call.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_text_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    content_pointer: *const u8,
    content_length: usize,
    font_size: f32,
    line_height: f32,
    weight: u16,
    alignment: u32,
    color: u32,
) -> i32 {
    super::vk_push_text(
        runtime,
        node_id,
        unsafe { borrowed_string(content_pointer, content_length) },
        font_size,
        line_height,
        weight,
        alignment,
        color,
    )
}

/// Pushes a semantic text node from borrowed UTF-8 bytes.
///
/// # Safety
///
/// `content_pointer` must address `content_length` readable bytes for the
/// duration of the call.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_text_role_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    content_pointer: *const u8,
    content_length: usize,
    role: u32,
    tone: u32,
    alignment: u32,
) -> i32 {
    super::vk_push_text_role(
        runtime,
        node_id,
        unsafe { borrowed_string(content_pointer, content_length) },
        role,
        tone,
        alignment,
    )
}

/// Pushes a button whose title is supplied as borrowed UTF-8 bytes.
///
/// # Safety
///
/// `title_pointer` must address `title_length` readable bytes for the duration
/// of the call.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_button_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    title_pointer: *const u8,
    title_length: usize,
    color: u32,
    radius: f32,
    action_id: u64,
) -> i32 {
    super::vk_push_button(
        runtime,
        node_id,
        unsafe { borrowed_string(title_pointer, title_length) },
        color,
        radius,
        action_id,
    )
}

/// Pushes a semantic button whose title is supplied as borrowed UTF-8 bytes.
///
/// # Safety
///
/// `title_pointer` must address `title_length` readable bytes for the duration
/// of the call.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_button_semantic_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    title_pointer: *const u8,
    title_length: usize,
    style: u32,
    size: u32,
    action_id: u64,
) -> i32 {
    super::vk_push_button_semantic(
        runtime,
        node_id,
        unsafe { borrowed_string(title_pointer, title_length) },
        style,
        size,
        action_id,
    )
}

/// Pushes a text field using borrowed UTF-8 value and placeholder bytes.
///
/// # Safety
///
/// Each non-null string pointer must address its corresponding readable byte
/// length for the duration of the call.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_text_field_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    state_id: u64,
    value_pointer: *const u8,
    value_length: usize,
    placeholder_pointer: *const u8,
    placeholder_length: usize,
    size: u32,
    radius: f32,
    enabled: u8,
    invalid: u8,
) -> i32 {
    super::vk_push_text_field(
        runtime,
        node_id,
        state_id,
        unsafe { borrowed_string(value_pointer, value_length) },
        unsafe { borrowed_string(placeholder_pointer, placeholder_length) },
        size,
        radius,
        enabled,
        invalid,
    )
}
