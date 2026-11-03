//! Window-protocol events are accepted only from the connected compositor.
//! Application protocols retain their own sender/Handle admission policy.
pub(super) fn compositor_event(bytes: &[u8]) -> bool {
    let Some(kind) = bytes.get(..4) else {
        return false;
    };
    matches!(u32::from_le_bytes(kind.try_into().unwrap()), 2..=14)
}
pub(super) fn accepts(sender: u64, compositor: u64, bytes: &[u8]) -> bool {
    !compositor_event(bytes) || (sender != 0 && sender == compositor)
}
