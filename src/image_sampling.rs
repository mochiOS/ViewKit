//! Shared, bounded preparation for raster images. Filtering always operates on
//! premultiplied, encoded RGBA8, matching the current surface blend contract.
use crate::{draw_command::ImageSampling, image::ImageData};
use std::sync::Mutex;

const BUDGET: usize = 64 * 1024 * 1024;
const ENTRIES: usize = 32;
struct Entry {
    source: ImageData,
    size: (u32, u32),
    sampling: ImageSampling,
    image: ImageData,
}
static CACHE: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

/// Resamples once for a physical destination size. Returns `None` rather than
/// allocating an unbounded intermediate; callers retain a direct sampling path.
pub(crate) fn prepare(
    source: &ImageData,
    width: u32,
    height: u32,
    sampling: ImageSampling,
) -> Option<ImageData> {
    if width == 0 || height == 0 {
        return None;
    }
    if source.dimensions() == (width, height) {
        return Some(source.clone());
    }
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if bytes > 32 * 1024 * 1024 {
        return None;
    }
    {
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(index) = cache.iter().position(|e| {
            e.source == *source && e.size == (width, height) && e.sampling == sampling
        }) {
            let entry = cache.remove(index);
            let image = entry.image.clone();
            cache.push(entry);
            return Some(image);
        }
    }
    let source_pixels = ::image::ImageBuffer::<::image::Rgba<u8>, &[u8]>::from_raw(
        source.width(),
        source.height(),
        source.premultiplied_rgba8(),
    )?;
    let filter = match sampling {
        ImageSampling::Nearest => ::image::imageops::FilterType::Nearest,
        ImageSampling::Bilinear => ::image::imageops::FilterType::Triangle,
        ImageSampling::Bicubic => ::image::imageops::FilterType::CatmullRom,
    };
    let mut pixels = ::image::imageops::resize(&source_pixels, width, height, filter).into_raw();
    // Negative cubic lobes must not make an RGB channel larger than alpha.
    for pixel in pixels.chunks_exact_mut(4) {
        for c in 0..3 {
            pixel[c] = pixel[c].min(pixel[3]);
        }
    }
    let image = ImageData::from_premultiplied_rgba8(width, height, pixels).ok()?;
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    // Account for retained source allocations too; identical sources count
    // conservatively more than once, keeping the real resident bound below it.
    let retained = |e: &Entry| {
        e.image
            .premultiplied_rgba8()
            .len()
            .saturating_add(e.source.premultiplied_rgba8().len())
    };
    let cost = bytes.saturating_add(source.premultiplied_rgba8().len());
    if cost <= BUDGET {
        let mut used: usize = cache.iter().map(retained).sum();
        while !cache.is_empty() && (cache.len() >= ENTRIES || used.saturating_add(cost) > BUDGET) {
            used = used.saturating_sub(retained(&cache.remove(0)));
        }
        cache.push(Entry {
            source: source.clone(),
            size: (width, height),
            sampling,
            image: image.clone(),
        });
    }
    Some(image)
}

/// Catmull–Rom interpolation for destinations too large for the prepared cache.
pub(crate) fn bicubic(source: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
    if width == 0 || height == 0 {
        return [0; 4];
    }
    let weight = |v: f32| {
        let a = v.abs();
        if a < 1.0 {
            1.5 * a * a * a - 2.5 * a * a + 1.0
        } else if a < 2.0 {
            -0.5 * a * a * a + 2.5 * a * a - 4.0 * a + 2.0
        } else {
            0.0
        }
    };
    let (ix, iy) = (x.floor() as isize, y.floor() as isize);
    let mut sum = [0.0f32; 4];
    for dy in -1..=2 {
        for dx in -1..=2 {
            let sx = (ix + dx).clamp(0, width as isize - 1) as usize;
            let sy = (iy + dy).clamp(0, height as isize - 1) as usize;
            let w = weight(x - (ix + dx) as f32) * weight(y - (iy + dy) as f32);
            if let Some(p) = source.get((sy * width + sx) * 4..(sy * width + sx) * 4 + 4) {
                for c in 0..4 {
                    sum[c] += p[c] as f32 * w;
                }
            }
        }
    }
    let mut pixel = sum.map(|c| c.round().clamp(0.0, 255.0) as u8);
    for c in 0..3 {
        pixel[c] = pixel[c].min(pixel[3]);
    }
    pixel
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cubic_is_distinct_and_keeps_transparency_premultiplied() {
        let source = ImageData::from_rgba8(
            4,
            1,
            vec![0, 0, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 0, 0, 0, 0],
        )
        .unwrap();
        let cubic = prepare(&source, 9, 1, ImageSampling::Bicubic).unwrap();
        let linear = prepare(&source, 9, 1, ImageSampling::Bilinear).unwrap();
        assert_ne!(cubic.premultiplied_rgba8(), linear.premultiplied_rgba8());
        for p in cubic.premultiplied_rgba8().chunks_exact(4) {
            assert!(p[0] <= p[3]);
        }
        assert_eq!(
            cubic,
            prepare(&source, 9, 1, ImageSampling::Bicubic).unwrap()
        );
        assert!(prepare(&source, 32768, 32768, ImageSampling::Bicubic).is_none());
        for x in [0.0, 0.25, 1.5, 2.75, 3.0] {
            let p = bicubic(source.premultiplied_rgba8(), 4, 1, x, 0.0);
            assert!(p[0] <= p[3]);
        }
    }
}
