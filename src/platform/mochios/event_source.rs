//! Window-protocol events are accepted only from the connected compositor.
//! Application protocols retain their own sender/Handle admission policy.
pub(super) fn compositor_event(bytes: &[u8]) -> bool {
    let Some(kind) = bytes.get(..4) else {
        return false;
    };
    matches!(u32::from_le_bytes(kind.try_into().unwrap()), 2..=15)
}
pub(super) fn accepts(sender: u64, compositor: u64, bytes: &[u8]) -> bool {
    !compositor_event(bytes) || (sender != 0 && sender == compositor)
}

/// Copy only received bytes: old 24-byte messages must never inherit an input
/// serial from unused bytes of the reusable receive buffer.
pub(super) fn core_event(bytes: &[u8]) -> [u8; 32] {
    let mut event = [0u8; 32];
    let copied = bytes.len().min(event.len());
    event[..copied].copy_from_slice(&bytes[..copied]);
    event
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_connected_compositor_can_issue_window_state_events() {
        let bytes = 15u32.to_le_bytes();
        assert!(!accepts(2, 1, &bytes));
        assert!(accepts(1, 1, &bytes));
    }
    #[test]
    fn legacy_pointer_events_have_no_interaction_serial() {
        let mut receive_buffer = [255u8; 64];
        receive_buffer[..4].copy_from_slice(&5u32.to_le_bytes());
        let old = core_event(&receive_buffer[..24]);
        assert_eq!(&old[24..32], &[0; 8]);
        let current = core_event(&receive_buffer[..32]);
        assert_eq!(&current[24..32], &[255; 8]);
    }
}
