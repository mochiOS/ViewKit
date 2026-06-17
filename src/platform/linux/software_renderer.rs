//! softbufferを使用したLinux向けソフトウェアレンダラー

use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{
    Context,
    SoftBufferError,
    Surface,
};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use crate::draw_command::{
    DisplayList,
    DrawCommand,
};
use crate::renderer::{
    Renderer,
    Viewport,
};
use crate::theme::Color;

#[derive(Debug, thiserror::Error)]
pub enum SoftwareRendererError {
    #[error("softbufferの処理に失敗しました: {0}")]
    SoftBuffer(#[from] SoftBufferError),
}

pub struct SoftwareRenderer {
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    viewport: Viewport,
}

impl SoftwareRenderer {
    pub fn new(
        context: &Context<OwnedDisplayHandle>,
        window: Rc<Window>,
        viewport: Viewport,
    ) -> Result<Self, SoftwareRendererError> {
        let surface = Surface::new(context, window)?;

        let mut renderer = Self {
            surface,
            viewport,
        };

        renderer.resize_surface(viewport)?;

        Ok(renderer)
    }

    fn resize_surface(
        &mut self,
        viewport: Viewport,
    ) -> Result<(), SoftwareRendererError> {
        self.viewport = viewport;

        let Some(width) =
            NonZeroU32::new(viewport.physical_width)
        else {
            return Ok(());
        };

        let Some(height) =
            NonZeroU32::new(viewport.physical_height)
        else {
            return Ok(());
        };

        self.surface.resize(width, height)?;

        Ok(())
    }
}

impl Renderer for SoftwareRenderer {
    type Error = SoftwareRendererError;

    fn resize(
        &mut self,
        viewport: Viewport,
    ) -> Result<(), Self::Error> {
        self.resize_surface(viewport)
    }

    fn render(
        &mut self,
        display_list: &DisplayList,
    ) -> Result<(), Self::Error> {
        if self.viewport.physical_width == 0
            || self.viewport.physical_height == 0
        {
            return Ok(());
        }

        let mut buffer = self.surface.buffer_mut()?;

        for command in display_list.commands() {
            match command {
                DrawCommand::Clear { color } => {
                    buffer.fill(encode_color(*color));
                }

                _ => {
                    todo!("他の命令はtiny-skiaにつなげる")
                }
            }
        }

        buffer.present()?;

        Ok(())
    }
}

fn encode_color(color: Color) -> u32 {
    u32::from(color.blue)
        | (u32::from(color.green) << 8)
        | (u32::from(color.red) << 16)
}