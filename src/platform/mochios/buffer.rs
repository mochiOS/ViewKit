use super::*;

use super::connection::{alloc_shared_page_count, send_pages};

pub(super) struct SharedBuffer {
    virt: u64,
    byte_capacity: usize,
    sent_pages: bool,
    attached: bool,
}

#[derive(Clone, Copy)]
pub(super) struct PhysicalDirtyRect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl SharedBuffer {
    pub(super) fn new(width: usize, height: usize) -> Result<Self, MochiOsBackendError> {
        let pixel_count = width
            .checked_mul(height)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let byte_len = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let page_count = byte_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let page_count = page_count.max(1);
        let byte_capacity = page_count
            .checked_mul(PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let virt = alloc_shared_page_count(page_count)?;

        Ok(Self {
            virt,
            byte_capacity,
            sent_pages: false,
            attached: false,
        })
    }

    pub(super) fn send_pixmap_to(
        &mut self,
        compositor: u64,
        pixmap: &Pixmap,
        background: Color,
        dirty_rect: PhysicalDirtyRect,
        format: u32,
    ) -> Result<(), MochiOsBackendError> {
        let pixel_count = (pixmap.width() as usize)
            .checked_mul(pixmap.height() as usize)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let bytes_len = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        if bytes_len > self.byte_capacity {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let dst =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u8, self.byte_capacity) };

        let pixmap_width = pixmap.width() as usize;
        let pixmap_height = pixmap.height() as usize;
        let copy_rect = if self.sent_pages {
            dirty_rect
        } else {
            PhysicalDirtyRect {
                x: 0,
                y: 0,
                width: pixmap_width,
                height: pixmap_height,
            }
        };
        let right = copy_rect
            .x
            .saturating_add(copy_rect.width)
            .min(pixmap_width);
        let bottom = copy_rect
            .y
            .saturating_add(copy_rect.height)
            .min(pixmap_height);
        let src = pixmap.data();
        for y in copy_rect.y..bottom {
            let Some(row_start) = y.checked_mul(pixmap_width) else {
                return Err(MochiOsBackendError::ArithmeticOverflow);
            };
            for x in copy_rect.x..right {
                let Some(pixel_index) = row_start.checked_add(x) else {
                    return Err(MochiOsBackendError::ArithmeticOverflow);
                };
                let Some(byte_index) = pixel_index.checked_mul(4) else {
                    return Err(MochiOsBackendError::ArithmeticOverflow);
                };
                let Some(pixel) = src.get(byte_index..byte_index + 4) else {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                };
                let Some(out) = dst.get_mut(byte_index..byte_index + 4) else {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                };
                let value = if format == PIXEL_FORMAT_ARGB8888_PREMULTIPLIED {
                    premultiplied_pixel(pixel)
                } else {
                    flatten_premultiplied_pixel(pixel, background)
                };
                out.copy_from_slice(&value.to_le_bytes());
            }
        }
        let page_count = bytes_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        if self.sent_pages {
            return Ok(());
        }
        send_pages(compositor, page_count, self.virt)?;
        self.sent_pages = true;
        Ok(())
    }

    pub(super) fn is_attached(&self) -> bool {
        self.attached
    }

    pub(super) fn mark_attached(&mut self) {
        self.attached = true;
    }
}

pub(super) fn flatten_premultiplied_pixel(pixel: &[u8], background: Color) -> u32 {
    let alpha = pixel[3] as u32;
    let inv_alpha = 255_u32.saturating_sub(alpha);

    // tiny-skia stores premultiplied RGBA. The compositor surface is XRGB,
    // so each pixel is flattened into the configured clear color.
    let red = pixel[0] as u32 + (background.red as u32 * inv_alpha + 127) / 255;
    let green = pixel[1] as u32 + (background.green as u32 * inv_alpha + 127) / 255;
    let blue = pixel[2] as u32 + (background.blue as u32 * inv_alpha + 127) / 255;

    0xff00_0000 | (red.min(255) << 16) | (green.min(255) << 8) | blue.min(255)
}

fn premultiplied_pixel(pixel: &[u8]) -> u32 {
    (u32::from(pixel[3]) << 24)
        | (u32::from(pixel[0]) << 16)
        | (u32::from(pixel[1]) << 8)
        | u32::from(pixel[2])
}
