//! Lossless storage of trusted embedded fonts; the shaping bytes stay identical.
use std::io::{self, Write};

const MAGIC: &[u8; 4] = b"FTZ1";
const MAX_BYTES: usize = 16 * 1024 * 1024;

#[allow(dead_code)]
pub(crate) fn encode(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(io::Error::other("embedded font size exceeds limit"));
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    encoder.write_all(bytes)?;
    let mut result = Vec::from(MAGIC.as_slice());
    result.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    result.extend_from_slice(&encoder.finish()?);
    Ok(result)
}

#[allow(dead_code)]
pub(crate) fn decode(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let invalid = || io::Error::other("invalid embedded font");
    if bytes.len() < 8 || &bytes[..4] != MAGIC {
        return Err(invalid());
    }
    let length = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| invalid())?) as usize;
    if length == 0 || length > MAX_BYTES {
        return Err(invalid());
    }
    let mut decoder = flate2::Decompress::new(true);
    let mut result = Vec::with_capacity(length + 1);
    let status = decoder
        .decompress_vec(&bytes[8..], &mut result, flate2::FlushDecompress::Finish)
        .map_err(|_| invalid())?;
    if status != flate2::Status::StreamEnd
        || result.len() != length
        || decoder.total_in() as usize != bytes.len() - 8
    {
        return Err(invalid());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_preserves_every_byte() {
        let bytes: Vec<_> = (0..250_000).map(|i| (i % 251) as u8).collect();
        let packed = encode(&bytes).unwrap();
        assert!(packed.len() < bytes.len());
        assert_eq!(decode(&packed).unwrap(), bytes);
    }
    #[test]
    fn shipped_fonts_round_trip_without_changing_glyph_or_weight_data() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../fonts/out/fonts");
        for name in [
            "InterVariable.ttf",
            "UDEVGothic-Regular.ttf",
            "IBMPlexSansJP-Regular.ttf",
            "IBMPlexSansJP-Medium.ttf",
            "IBMPlexSansJP-SemiBold.ttf",
        ] {
            let original = std::fs::read(root.join(name)).expect("shipped test font");
            let packed = encode(&original).unwrap();
            assert!(packed.len() < original.len(), "{name}");
            assert_eq!(decode(&packed).unwrap(), original, "{name}");
        }
    }
    #[test]
    fn rejects_corrupt_truncated_appended_and_oversized_streams() {
        let packed = encode(&[42; 8192]).unwrap();
        assert!(decode(&packed[..packed.len() - 1]).is_err());
        let mut corrupt = packed.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(decode(&corrupt).is_err());
        let mut appended = packed.clone();
        appended.push(0);
        assert!(decode(&appended).is_err());
        let mut wrong_length = packed.clone();
        wrong_length[4..8].copy_from_slice(&8191u32.to_le_bytes());
        assert!(decode(&wrong_length).is_err());
        let mut oversized = packed;
        oversized[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&oversized).is_err());
        assert!(encode(&[]).is_err());
    }
}
