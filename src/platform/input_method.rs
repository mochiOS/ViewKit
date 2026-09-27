#[cfg(target_os = "mochios")]
const CONVERT_OPCODE: u32 = u32::from_le_bytes(*b"IMEC");
#[cfg(target_os = "mochios")]
const STATUS_OPCODE: u32 = u32::from_le_bytes(*b"IMES");
#[cfg(target_os = "mochios")]
const TOGGLE_OPCODE: u32 = u32::from_le_bytes(*b"IMET");
#[cfg(target_os = "mochios")]
const HEADER_LEN: usize = 12;
const REPLY_HEADER_LEN: usize = 8;
#[cfg(target_os = "mochios")]
const MAX_MESSAGE_LEN: usize = 4096;
const MAX_CANDIDATES: usize = 20;

#[cfg(target_os = "mochios")]
pub fn candidates(reading: &str) -> Vec<String> {
    use mochi_user_platform as platform;

    let bytes = reading.as_bytes();
    let Ok(reading_len) = u16::try_from(bytes.len()) else {
        return Vec::new();
    };
    if bytes.is_empty() || HEADER_LEN + bytes.len() > MAX_MESSAGE_LEN {
        return Vec::new();
    }
    let Ok(input) = platform::process::find_by_name("input.service") else {
        return Vec::new();
    };
    if input == 0 {
        return Vec::new();
    }

    let mut request = Vec::with_capacity(HEADER_LEN + bytes.len());
    request.extend_from_slice(&CONVERT_OPCODE.to_le_bytes());
    request.extend_from_slice(&reading_len.to_le_bytes());
    request.extend_from_slice(&(MAX_CANDIDATES as u16).to_le_bytes());
    request.extend_from_slice(&[0; 4]);
    request.extend_from_slice(bytes);
    let mut reply = vec![0; MAX_MESSAGE_LEN];
    let Ok(message) = platform::ipc::call(input, &request, &mut reply) else {
        return Vec::new();
    };
    decode_reply(&reply[..((message & 0xffff_ffff) as usize).min(reply.len())])
}

#[cfg(not(target_os = "mochios"))]
pub fn candidates(_reading: &str) -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "mochios")]
pub fn enabled() -> Option<bool> {
    state_call(STATUS_OPCODE)
}

#[cfg(not(target_os = "mochios"))]
pub fn enabled() -> Option<bool> {
    None
}

#[cfg(target_os = "mochios")]
pub fn toggle() -> Option<bool> {
    state_call(TOGGLE_OPCODE)
}

#[cfg(not(target_os = "mochios"))]
pub fn toggle() -> Option<bool> {
    None
}

#[cfg(target_os = "mochios")]
fn state_call(opcode: u32) -> Option<bool> {
    use mochi_user_platform as platform;

    let input = platform::process::find_by_name("input.service").ok()?;
    if input == 0 {
        return None;
    }
    let mut reply = [0u8; 1];
    let message = platform::ipc::call(input, &opcode.to_le_bytes(), &mut reply).ok()?;
    ((message & 0xffff_ffff) == 1).then_some(reply[0] != 0)
}

fn decode_reply(reply: &[u8]) -> Vec<String> {
    if reply.len() < REPLY_HEADER_LEN
        || i32::from_le_bytes(reply[0..4].try_into().unwrap_or_default()) != 0
    {
        return Vec::new();
    }
    let count = u16::from_le_bytes(reply[4..6].try_into().unwrap_or_default()) as usize;
    let mut offset = REPLY_HEADER_LEN;
    let mut result = Vec::with_capacity(count.min(MAX_CANDIDATES));
    for _ in 0..count.min(MAX_CANDIDATES) {
        let Some(length_bytes) = reply.get(offset..offset + 2) else {
            return Vec::new();
        };
        let length = u16::from_le_bytes(length_bytes.try_into().unwrap_or_default()) as usize;
        offset += 2;
        let Some(bytes) = reply.get(offset..offset + length) else {
            return Vec::new();
        };
        let Ok(candidate) = std::str::from_utf8(bytes) else {
            return Vec::new();
        };
        result.push(candidate.to_owned());
        offset += length;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_length_prefixed_candidates() {
        let mut reply = vec![0; REPLY_HEADER_LEN];
        reply[4..6].copy_from_slice(&2u16.to_le_bytes());
        for candidate in ["今日", "京"] {
            reply.extend_from_slice(&(candidate.len() as u16).to_le_bytes());
            reply.extend_from_slice(candidate.as_bytes());
        }
        assert_eq!(decode_reply(&reply), ["今日", "京"]);
    }

    #[test]
    fn rejects_truncated_reply() {
        let mut reply = vec![0; REPLY_HEADER_LEN];
        reply[4..6].copy_from_slice(&1u16.to_le_bytes());
        reply.extend_from_slice(&9u16.to_le_bytes());
        assert!(decode_reply(&reply).is_empty());
    }
}
