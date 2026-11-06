//! Release events carry a physical code, often with no Unicode codepoint.
//! Retain the logical key from its press instead of losing character releases.
pub(crate) struct PressedKeys<K> {
    keys: Vec<(u16, K)>,
}
impl<K: Copy> PressedKeys<K> {
    pub(crate) fn new() -> Self {
        Self { keys: Vec::new() }
    }
    pub(crate) fn transition(&mut self, code: u16, decoded: Option<K>, pressed: bool) -> Option<K> {
        let index = self.keys.iter().position(|(old, _)| *old == code);
        if pressed {
            if let Some(key) = decoded {
                if index.is_none() {
                    self.keys.push((code, key));
                }
            }
            decoded
        } else {
            index
                .map(|index| self.keys.swap_remove(index).1)
                .or(decoded)
        }
    }
    pub(crate) fn clear(&mut self) {
        self.keys.clear();
    }
}
