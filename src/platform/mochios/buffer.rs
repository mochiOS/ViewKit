use super::*;

use super::connection::syscall_result;
use memory_object::{MemoryMapping, MemoryObject};

pub(super) struct SharedBuffer {
    mapping: MemoryMapping,
    object: MemoryObject,
    byte_capacity: usize,
    offered_endpoint: Option<u64>,
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
        Self::new_with_minimum_capacity(width, height, 0)
    }

    pub(super) fn new_gpu_scene(width: usize, height: usize) -> Result<Self, MochiOsBackendError> {
        use mochios_viewkit_gpu_protocol::{
            ATLAS_HEIGHT, ATLAS_WIDTH, HEADER_LEN, MAX_VERTICES, VERTEX_STRIDE,
        };

        let vertex_bytes = (MAX_VERTICES as usize)
            .checked_mul(VERTEX_STRIDE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let atlas_bytes = (ATLAS_WIDTH as usize)
            .checked_mul(ATLAS_HEIGHT as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let scene_capacity = HEADER_LEN
            .checked_add(vertex_bytes)
            .and_then(|bytes| bytes.checked_add(atlas_bytes))
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;

        Self::new_with_minimum_capacity(width, height, scene_capacity)
    }

    fn new_with_minimum_capacity(
        width: usize,
        height: usize,
        minimum_capacity: usize,
    ) -> Result<Self, MochiOsBackendError> {
        let pixel_count = width
            .checked_mul(height)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let pixel_bytes = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let byte_len = pixel_bytes.max(minimum_capacity);
        let page_count = byte_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let page_count = page_count.max(1);
        let byte_capacity = page_count
            .checked_mul(PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let object = syscall_result(MemoryObject::create(byte_capacity as u64))?;
        let mapping = syscall_result(object.map(
            0,
            byte_capacity as u64,
            syscall::MEMORY_ACCESS_READ | syscall::MEMORY_ACCESS_WRITE,
        ))?;

        Ok(Self {
            mapping,
            object,
            byte_capacity,
            offered_endpoint: None,
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
        // SAFETY: the owned mapping covers byte_capacity; this process is the
        // only writer and every transferred handle is restricted to READ|MAP.
        let dst =
            unsafe { std::slice::from_raw_parts_mut(self.mapping.as_mut_ptr(), self.byte_capacity) };

        let pixmap_width = pixmap.width() as usize;
        let pixmap_height = pixmap.height() as usize;
        let copy_rect = if self.offered_endpoint == Some(compositor) {
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
        self.offer(compositor)
    }

    pub(super) fn send_scene_to(
        &mut self,
        compositor: u64,
        scene: &[u8],
    ) -> Result<(), MochiOsBackendError> {
        if scene.len() > self.byte_capacity {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        // SAFETY: scene length was checked against the retained mapping and
        // peers receive read-only access, never a writable handle.
        let destination =
            unsafe { std::slice::from_raw_parts_mut(self.mapping.as_mut_ptr(), self.byte_capacity) };
        destination[..scene.len()].copy_from_slice(scene);
        self.offer(compositor)
    }

    fn offer(&mut self, endpoint: u64) -> Result<(), MochiOsBackendError> {
        if endpoint == 0 {
            return Err(MochiOsBackendError::InvalidReply);
        }
        if self.offered_endpoint != Some(endpoint) {
            syscall_result(self.object.send_readonly(endpoint, 0, self.byte_capacity as u64))?;
            self.offered_endpoint = Some(endpoint);
        }
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
