//! Flat UTF-8 C entry points used by language bindings.

use super::tree::{FfiBuiltView, FfiNode, FfiViewFactory, expect_no_children};
use super::{VkRuntime, VkStatus, VkString};

type KomeClosureOwnership = unsafe extern "C" fn(u64);

/// Stable prefix of a Kome closure passed through the C ABI.
#[repr(C)]
pub struct KomeClosure {
    pub code: u64,
    pub environment: u64,
    pub retain: KomeClosureOwnership,
    pub release: KomeClosureOwnership,
}

struct OwnedKomeAction {
    closure: *const KomeClosure,
}

impl OwnedKomeAction {
    unsafe fn retain(closure: *const KomeClosure) -> Result<Self, VkStatus> {
        if closure.is_null() {
            return Err(VkStatus::NullPointer);
        }
        let value = unsafe { &*closure };
        if value.code == 0 {
            return Err(VkStatus::InvalidValue);
        }
        unsafe { (value.retain)(closure as u64) };
        Ok(Self { closure })
    }

    fn invoke(&mut self) {
        let closure = unsafe { &*self.closure };
        let function: unsafe extern "C" fn(u64) =
            unsafe { std::mem::transmute(closure.code as usize) };
        unsafe { function(closure.environment) };
    }
}

impl Drop for OwnedKomeAction {
    fn drop(&mut self) {
        let closure = unsafe { &*self.closure };
        unsafe { (closure.release)(self.closure as u64) };
    }
}

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

/// Pushes a button that owns and invokes a Kome closure when clicked.
///
/// # Safety
///
/// `title_pointer` must address `title_length` readable bytes for the duration
/// of the call. `action` must point to a live Kome closure with the signature
/// `() -> Void`. ViewKit retains it before returning.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_button_action_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    title_pointer: *const u8,
    title_length: usize,
    color: u32,
    radius: f32,
    action: *const KomeClosure,
) -> i32 {
    super::ffi_status(|| {
        let title = super::copy_string(unsafe { borrowed_string(title_pointer, title_length) })?;
        let color = super::decode_button_color(color)?;
        let radius = super::sanitize_length(radius);
        let action = unsafe { OwnedKomeAction::retain(action) }?;
        let factory: FfiViewFactory = Box::new(move |_node_id, children, _context| {
            expect_no_children(children)?;
            let mut action = action;
            let button = crate::components::Button::new(title)
                .color(color)
                .radius(crate::theme::CornerRadius::Custom(radius))
                .on_click(move || action.invoke());
            Ok(FfiBuiltView::View(Box::new(button)))
        });
        let runtime = super::runtime_mut(runtime)?;
        let builder = super::active_builder(runtime)?;
        let node = FfiNode::component(node_id, factory);
        builder.leaf(node);
        Ok(())
    })
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

/// Pushes a menu item using borrowed UTF-8 label and shortcut bytes.
///
/// # Safety
///
/// Each non-null string pointer must address its corresponding readable byte
/// length for the duration of the call.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn vk_push_menu_item_utf8(
    runtime: *mut VkRuntime,
    node_id: u64,
    label_pointer: *const u8,
    label_length: usize,
    shortcut_pointer: *const u8,
    shortcut_length: usize,
    enabled: u8,
    danger: u8,
    action_id: u64,
) -> i32 {
    super::vk_push_menu_item(
        runtime,
        node_id,
        unsafe { borrowed_string(label_pointer, label_length) },
        unsafe { borrowed_string(shortcut_pointer, shortcut_length) },
        enabled,
        danger,
        action_id,
    )
}

#[cfg(test)]
mod tests {
    use super::{KomeClosure, OwnedKomeAction};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static INVOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static RETAINS: AtomicUsize = AtomicUsize::new(0);
    static RELEASES: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn invoke(environment: u64) {
        INVOCATIONS.fetch_add(environment as usize, Ordering::SeqCst);
    }

    unsafe extern "C" fn retain(_closure: u64) {
        RETAINS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn release(_closure: u64) {
        RELEASES.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn retains_invokes_and_releases_a_kome_action() {
        INVOCATIONS.store(0, Ordering::SeqCst);
        RETAINS.store(0, Ordering::SeqCst);
        RELEASES.store(0, Ordering::SeqCst);
        let closure = KomeClosure {
            code: invoke as *const () as usize as u64,
            environment: 3,
            retain,
            release,
        };

        let mut action = unsafe { OwnedKomeAction::retain(&closure) }.unwrap();
        action.invoke();
        drop(action);

        assert_eq!(INVOCATIONS.load(Ordering::SeqCst), 3);
        assert_eq!(RETAINS.load(Ordering::SeqCst), 1);
        assert_eq!(RELEASES.load(Ordering::SeqCst), 1);
    }
}
