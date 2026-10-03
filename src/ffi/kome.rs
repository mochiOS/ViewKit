//! Flat UTF-8 C entry points used by language bindings.

use super::tree::{FfiBuiltView, FfiNode, FfiViewFactory, expect_no_children};
use super::{VkRuntime, VkStatus, VkString};
use std::cell::RefCell;

type KomeClosureOwnership = unsafe extern "C" fn(u64);

/// Stable prefix of a Kome closure passed through the C ABI.
#[repr(C)]
pub struct KomeClosure {
    pub code: u64,
    pub environment: u64,
    pub retain: KomeClosureOwnership,
    pub release: KomeClosureOwnership,
}

impl KomeClosure {
    fn validate(&self) -> Result<(), VkStatus> {
        if self.code == 0 {
            Err(VkStatus::InvalidValue)
        } else {
            Ok(())
        }
    }

    unsafe fn invoke(&self) {
        let function: unsafe extern "C" fn(u64) =
            unsafe { std::mem::transmute(self.code as usize) };
        unsafe { function(self.environment) };
    }

    unsafe fn invoke_null(&self) {
        let function: unsafe extern "C" fn(u64) -> u8 =
            unsafe { std::mem::transmute(self.code as usize) };
        let _ = unsafe { function(self.environment) };
    }
}

#[derive(Clone, Copy)]
struct KomeBuildState {
    runtime: *mut VkRuntime,
    next_node_id: u64,
    status: i32,
}

thread_local! {
    static KOME_BUILD_STATE: RefCell<Option<KomeBuildState>> = const { RefCell::new(None) };
}

struct KomeBuildGuard;

impl KomeBuildGuard {
    fn begin(runtime: *mut VkRuntime) -> Result<Self, VkStatus> {
        KOME_BUILD_STATE.with(|slot| {
            let mut state = slot.borrow_mut();
            if state.is_some() {
                return Err(VkStatus::BuilderAlreadyActive);
            }
            *state = Some(KomeBuildState {
                runtime,
                next_node_id: 2,
                status: VkStatus::Ok as i32,
            });
            Ok(Self)
        })
    }

    fn finish(self) -> KomeBuildState {
        KOME_BUILD_STATE.with(|slot| slot.borrow_mut().take().expect("Kome build state exists"))
    }
}

impl Drop for KomeBuildGuard {
    fn drop(&mut self) {
        KOME_BUILD_STATE.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}

fn remember_kome_status(status: i32) -> i32 {
    if status != VkStatus::Ok as i32 {
        KOME_BUILD_STATE.with(|slot| {
            if let Some(state) = slot.borrow_mut().as_mut()
                && state.status == VkStatus::Ok as i32
            {
                state.status = status;
            }
        });
    }
    status
}

fn with_kome_node(operation: impl FnOnce(*mut VkRuntime, u64) -> i32) -> i32 {
    let Some((runtime, node_id, status)) = KOME_BUILD_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let state = state.as_mut()?;
        let node_id = state.next_node_id;
        state.next_node_id = state.next_node_id.saturating_add(1);
        Some((state.runtime, node_id, state.status))
    }) else {
        return VkStatus::NoActiveBuilder as i32;
    };
    if status != VkStatus::Ok as i32 {
        return status;
    }
    remember_kome_status(operation(runtime, node_id))
}

fn with_kome_runtime(operation: impl FnOnce(*mut VkRuntime) -> i32) -> i32 {
    let Some((runtime, status)) = KOME_BUILD_STATE.with(|slot| {
        let state = slot.borrow();
        let state = state.as_ref()?;
        Some((state.runtime, state.status))
    }) else {
        return VkStatus::NoActiveBuilder as i32;
    };
    if status != VkStatus::Ok as i32 {
        return status;
    }
    remember_kome_status(operation(runtime))
}

fn status_result(status: i32) -> Result<(), VkStatus> {
    if status == VkStatus::Ok as i32 {
        Ok(())
    } else {
        Err(match status {
            value if value == VkStatus::NullPointer as i32 => VkStatus::NullPointer,
            value if value == VkStatus::InvalidUtf8 as i32 => VkStatus::InvalidUtf8,
            value if value == VkStatus::BuilderAlreadyActive as i32 => {
                VkStatus::BuilderAlreadyActive
            }
            value if value == VkStatus::NoActiveBuilder as i32 => VkStatus::NoActiveBuilder,
            value if value == VkStatus::NoOpenNode as i32 => VkStatus::NoOpenNode,
            value if value == VkStatus::UnclosedNodes as i32 => VkStatus::UnclosedNodes,
            value if value == VkStatus::MultipleRoots as i32 => VkStatus::MultipleRoots,
            value if value == VkStatus::MissingRoot as i32 => VkStatus::MissingRoot,
            value if value == VkStatus::InvalidEnumValue as i32 => VkStatus::InvalidEnumValue,
            value if value == VkStatus::UnsupportedEvent as i32 => VkStatus::UnsupportedEvent,
            value if value == VkStatus::PlatformError as i32 => VkStatus::PlatformError,
            value if value == VkStatus::UnsupportedPlatform as i32 => VkStatus::UnsupportedPlatform,
            value if value == VkStatus::InvalidChildCount as i32 => VkStatus::InvalidChildCount,
            value if value == VkStatus::InvalidTreeNode as i32 => VkStatus::InvalidTreeNode,
            value if value == VkStatus::StateNotFound as i32 => VkStatus::StateNotFound,
            value if value == VkStatus::StateTypeMismatch as i32 => VkStatus::StateTypeMismatch,
            value if value == VkStatus::BufferTooSmall as i32 => VkStatus::BufferTooSmall,
            value if value == VkStatus::InvalidValue as i32 => VkStatus::InvalidValue,
            _ => VkStatus::Panic,
        })
    }
}

unsafe fn build_kome_tree(action: *const KomeClosure) -> Result<Box<VkRuntime>, VkStatus> {
    let action = unsafe { action.as_ref() }.ok_or(VkStatus::NullPointer)?;
    action.validate()?;
    let mut runtime = Box::new(VkRuntime::new(1));
    let runtime_pointer = runtime.as_mut() as *mut VkRuntime;
    status_result(super::vk_tree_begin(runtime_pointer, 1))?;
    let guard = KomeBuildGuard::begin(runtime_pointer)?;
    unsafe { action.invoke_null() };
    let state = guard.finish();
    status_result(state.status)?;
    status_result(super::vk_tree_commit(runtime_pointer))?;
    Ok(runtime)
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
        unsafe { closure.invoke() };
    }
}

/// Runs a window built by a Kome closure without exposing runtime or node identifiers.
///
/// # Safety
///
/// `title_pointer` must address `title_length` readable bytes for the duration of the call.
/// `content` must point to a live Kome closure with the signature `() -> Null`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vk_kome_run_window_action_utf8(
    title_pointer: *const u8,
    title_length: usize,
    width: f32,
    height: f32,
    resizable: u8,
    content: *const KomeClosure,
) -> i32 {
    super::ffi_status(|| {
        let mut runtime = unsafe { build_kome_tree(content) }?;
        status_result(unsafe {
            vk_runtime_run_window_utf8(
                runtime.as_mut(),
                title_pointer,
                title_length,
                width,
                height,
                resizable,
            )
        })
    })
}

/// Begins a vertical stack in the active Kome view build.
#[unsafe(no_mangle)]
pub extern "C" fn vk_kome_begin_vstack(gap: u32, alignment: u32, distribution: u32) -> i32 {
    with_kome_node(|runtime, node_id| {
        super::vk_begin_vstack(runtime, node_id, gap, alignment, distribution)
    })
}

/// Begins a horizontal stack in the active Kome view build.
#[unsafe(no_mangle)]
pub extern "C" fn vk_kome_begin_hstack(gap: u32, alignment: u32, distribution: u32) -> i32 {
    with_kome_node(|runtime, node_id| {
        super::vk_begin_hstack(runtime, node_id, gap, alignment, distribution)
    })
}

/// Ends the current container in the active Kome view build.
#[unsafe(no_mangle)]
pub extern "C" fn vk_kome_end_node() -> i32 {
    with_kome_runtime(|runtime| super::vk_end_node(runtime))
}

/// Pushes semantic text into the active Kome view build.
///
/// # Safety
///
/// `content_pointer` must address `content_length` readable bytes for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vk_kome_push_text_role_utf8(
    content_pointer: *const u8,
    content_length: usize,
    role: u32,
    tone: u32,
    alignment: u32,
) -> i32 {
    with_kome_node(|runtime, node_id| unsafe {
        vk_push_text_role_utf8(
            runtime,
            node_id,
            content_pointer,
            content_length,
            role,
            tone,
            alignment,
        )
    })
}

/// Pushes a closure-backed button into the active Kome view build.
///
/// # Safety
///
/// `title_pointer` must address `title_length` readable bytes for the duration of the call.
/// `action` must point to a live Kome closure with the signature `() -> Void`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vk_kome_push_button_action_utf8(
    title_pointer: *const u8,
    title_length: usize,
    color: u32,
    radius: f32,
    action: *const KomeClosure,
) -> i32 {
    with_kome_node(|runtime, node_id| unsafe {
        vk_push_button_action_utf8(
            runtime,
            node_id,
            title_pointer,
            title_length,
            color,
            radius,
            action,
        )
    })
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
    use super::{
        KomeClosure, OwnedKomeAction, build_kome_tree, vk_kome_begin_vstack, vk_kome_end_node,
        vk_kome_push_text_role_utf8,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    static INVOCATIONS: AtomicUsize = AtomicUsize::new(0);
    static RETAINS: AtomicUsize = AtomicUsize::new(0);
    static RELEASES: AtomicUsize = AtomicUsize::new(0);
    static BUILD_ERRORS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn invoke(environment: u64) {
        INVOCATIONS.fetch_add(environment as usize, Ordering::SeqCst);
    }

    unsafe extern "C" fn retain(_closure: u64) {
        RETAINS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn release(_closure: u64) {
        RELEASES.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn build_view(_environment: u64) -> u8 {
        let mut errors = 0;
        if vk_kome_begin_vstack(2, 1, 0) != 0 {
            errors += 1;
        }
        let content = "Kome";
        if unsafe { vk_kome_push_text_role_utf8(content.as_ptr(), content.len(), 5, 0, 0) } != 0 {
            errors += 1;
        }
        if vk_kome_end_node() != 0 {
            errors += 1;
        }
        BUILD_ERRORS.store(errors, Ordering::SeqCst);
        0
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

    #[test]
    fn builds_a_kome_view_without_explicit_runtime_or_node_ids() {
        BUILD_ERRORS.store(0, Ordering::SeqCst);
        let closure = KomeClosure {
            code: build_view as *const () as usize as u64,
            environment: 0,
            retain,
            release,
        };

        let runtime = unsafe { build_kome_tree(&closure) }.unwrap();

        assert_eq!(BUILD_ERRORS.load(Ordering::SeqCst), 0);
        assert!(runtime.root.is_some());
        assert!(runtime.builder.is_none());
    }
}
