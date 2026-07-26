use super::*;

pub(super) fn syscall_result<T>(result: syscall::SysResult<T>) -> Result<T, MochiOsBackendError> {
    result.map_err(|err| MochiOsBackendError::Syscall(err.errno().unwrap_or(5)))
}

pub(super) fn perf_log(line: &str) {
    let _ = syscall::call3(
        syscall::SyscallNumber::Write,
        2,
        line.as_ptr() as u64,
        line.len() as u64,
    );
}

pub(super) fn perf_counter() -> u64 {
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

pub(super) fn perf_counter_elapsed(start: u64) -> u64 {
    perf_counter().saturating_sub(start)
}

pub(super) fn perf_tick() -> u64 {
    syscall::call0(syscall::SyscallNumber::TimeNow).unwrap_or(0)
}

pub(super) fn perf_tick_elapsed(start: u64) -> u64 {
    perf_tick().saturating_sub(start)
}

pub(super) fn create_event_endpoint() -> Result<u64, MochiOsBackendError> {
    syscall_result(syscall::call2(syscall::SyscallNumber::IpcCreate, 0, 0))
}

pub(super) fn find_compositor() -> Result<u64, MochiOsBackendError> {
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

pub(super) fn find_display_driver() -> Result<u64, MochiOsBackendError> {
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

pub(super) fn find_input_service() -> Result<u64, MochiOsBackendError> {
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

pub(super) fn subscribe_input_events(endpoint: u64) -> bool {
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

pub(super) fn require_window_overlay_capability() -> Result<(), MochiOsBackendError> {
    if query_capability(WINDOW_OVERLAY_CAPABILITY) {
        return Ok(());
    }
    Err(MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))
}

pub(super) fn query_capability(capability: &str) -> bool {
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

pub(super) fn display_surface_size() -> Option<crate::geometry::Size> {
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

pub(super) fn load_cursor_image() -> Option<ImageData> {
    let svg = SvgData::from_path(CURSOR_SVG_PATH).ok()?;
    ImageData::from_svg(&svg, CURSOR_WIDTH, CURSOR_HEIGHT).ok()
}

pub(super) fn ipc_call_raw(
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

pub(super) fn ipc_wait_raw(
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

pub(super) fn alloc_shared_page_count(page_count: usize) -> Result<u64, MochiOsBackendError> {
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

pub(super) fn send_pages(
    dest: u64,
    page_count: usize,
    local_base: u64,
) -> Result<(), MochiOsBackendError> {
    syscall_result(syscall::call4(
        syscall::SyscallNumber::IpcSendPages,
        dest,
        0,
        page_count as u64,
        local_base,
    ))?;
    Ok(())
}

pub(super) unsafe fn zero_raw(ptr: *mut u8, len: usize) {
    unsafe {
        core::ptr::write_bytes(ptr, 0, len);
    }
}

pub(super) unsafe fn put_u32_raw(ptr: *mut u8, offset: usize, value: u32) {
    unsafe {
        core::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(offset), 4);
    }
}

pub(super) unsafe fn read_i32_raw(ptr: *const u8, offset: usize) -> i32 {
    let mut bytes = [0u8; 4];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 4);
    }
    i32::from_le_bytes(bytes)
}

pub(super) unsafe fn put_u64_raw(ptr: *mut u8, offset: usize, value: u64) {
    unsafe {
        core::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(offset), 8);
    }
}

pub(super) unsafe fn read_u32_raw(ptr: *const u8, offset: usize) -> u32 {
    let mut bytes = [0u8; 4];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 4);
    }
    u32::from_le_bytes(bytes)
}

pub(super) unsafe fn read_u64_raw(ptr: *const u8, offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 8);
    }
    u64::from_le_bytes(bytes)
}

pub(super) fn status_from_raw(ptr: *const u8, len: usize) -> Result<(), MochiOsBackendError> {
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

pub(super) fn try_recv_event()
-> Result<Option<(usize, [u8; EVENT_BUFFER_SIZE])>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = match ipc_wait_raw(0, event, EVENT_BUFFER_SIZE) {
        Ok(len) => len,
        Err(MochiOsBackendError::Syscall(ERRNO_EAGAIN)) => return Ok(None),
        Err(err) => return Err(err),
    };
    if len < 16 {
        return Err(MochiOsBackendError::InvalidEvent);
    }
    let mut out = [0u8; EVENT_BUFFER_SIZE];
    let copy_len = len.min(out.len());
    unsafe {
        core::ptr::copy_nonoverlapping(event, out.as_mut_ptr(), copy_len);
    }
    Ok(Some((len, out)))
}

pub(super) fn wait_for_event<A: PlatformApplication>(
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

pub(super) fn wait_until_deadline<A: PlatformApplication>(
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

pub(super) fn read_event_blocking(
    endpoint: u64,
) -> Result<Option<(usize, [u8; EVENT_BUFFER_SIZE])>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = match ipc_wait_raw(endpoint, event, EVENT_BUFFER_SIZE) {
        Ok(len) => len,
        Err(MochiOsBackendError::Syscall(ERRNO_EAGAIN)) => return Ok(None),
        Err(err) => return Err(err),
    };
    if len < 16 {
        return Err(MochiOsBackendError::InvalidEvent);
    }
    let mut out = [0u8; EVENT_BUFFER_SIZE];
    let copy_len = len.min(out.len());
    unsafe {
        core::ptr::copy_nonoverlapping(event, out.as_mut_ptr(), copy_len);
    }
    Ok(Some((len, out)))
}
