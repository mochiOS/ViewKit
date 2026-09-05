use super::*;

use super::connection::{alloc_shared_page_count, send_pages};

pub(super) struct SharedBuffer {
    virt: u64,
    byte_capacity: usize,
    sent_pages: bool,
    attached: bool,
}

impl SharedBuffer {
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
        let virt = alloc_shared_page_count(page_count)?;

        Ok(Self {
            virt,
            byte_capacity,
            sent_pages: false,
            attached: false,
        })
    }

    pub(super) fn send_scene_to(
        &mut self,
        compositor: u64,
        scene: &[u8],
    ) -> Result<(), MochiOsBackendError> {
        if scene.len() > self.byte_capacity {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let destination =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u8, self.byte_capacity) };
        destination[..scene.len()].copy_from_slice(scene);
        if !self.sent_pages {
            let page_count = self.byte_capacity / PAGE_SIZE;
            send_pages(compositor, page_count, self.virt)?;
            self.sent_pages = true;
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
