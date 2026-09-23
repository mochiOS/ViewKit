use std::mem;
use std::rc::Rc;

use crate::draw_command::{DisplayList, DrawCommand};
use crate::geometry::Rect;
use crate::renderer::{Renderer, Viewport};
use crate::theme::Color;

use winit::window::Window;
#[cfg(target_os = "linux")]
use winit::event_loop::OwnedDisplayHandle;

const SHADER: &str = include_str!("../../shaders/windows_gpu.wgsl");
const BLIT_SHADER: &str = include_str!("../../shaders/desktop_blit.wgsl");

#[cfg(target_os = "windows")]
const GPU_BACKENDS: wgpu::Backends = wgpu::Backends::DX12;

#[cfg(target_os = "linux")]
const GPU_BACKENDS: wgpu::Backends = wgpu::Backends::from_bits_retain(
    wgpu::Backends::VULKAN.bits() | wgpu::Backends::GL.bits(),
);

#[cfg(target_os = "windows")]
const GPU_DEVICE_LABEL: &str = "ViewKit Windows GPU device";

#[cfg(target_os = "linux")]
const GPU_DEVICE_LABEL: &str = "ViewKit Linux GPU device";

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[derive(Debug, thiserror::Error)]
pub enum GpuRendererError {
    #[error("Failed to create the GPU surface: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),

    #[error("Failed to get the GPU surface handle: {0}")]
    Handle(#[from] raw_window_handle::HandleError),

    #[error("Failed to request a compatible GPU adapter: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),

    #[error("Failed to create the GPU device: {0}")]
    Device(#[from] wgpu::RequestDeviceError),

    #[error("The GPU surface does not expose a compatible texture format")]
    SurfaceFormatUnavailable,

    #[error("Failed to acquire the next GPU frame: {0}")]
    SurfaceTexture(&'static str),

    #[error("The display list contains a command that the GPU renderer does not support yet")]
    UnsupportedCommand,
}

pub struct GpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    blit_texture: wgpu::Texture,
    blit_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    viewport: Viewport,
}

impl GpuRenderer {
    pub fn new(
        window: Rc<Window>,
        viewport: Viewport,
        #[cfg(target_os = "linux")] display: OwnedDisplayHandle,
    ) -> Result<Self, GpuRendererError> {
        #[cfg(target_os = "windows")]
        let instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        #[cfg(target_os = "linux")]
        let instance_descriptor =
            wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: GPU_BACKENDS,
            ..instance_descriptor
        });

        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::from_window(window.as_ref())?;
            instance.create_surface_unsafe(target)?
        };

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))?;

        if std::env::var_os("VIEWKIT_RENDERER_DIAGNOSTICS").is_some() {
            let info = adapter.get_info();
            eprintln!(
                "[ViewKit] GPU renderer: {} ({:?}, {:?})",
                info.name, info.backend, info.device_type
            );
        }

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some(GPU_DEVICE_LABEL),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            }))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(GpuRendererError::SurfaceFormatUnavailable)?;

        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::AutoVsync)
            .unwrap_or(wgpu::PresentMode::Fifo);

        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: viewport.physical_width.max(1),
            height: viewport.physical_height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ViewKit Windows GPU shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ViewKit Windows GPU pipeline layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ViewKit Windows GPU pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ViewKit desktop blit bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let blit_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ViewKit desktop blit pipeline layout"),
                bind_group_layouts: &[Some(&blit_bind_group_layout)],
                immediate_size: 0,
            });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ViewKit desktop blit shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ViewKit desktop blit pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ViewKit desktop blit sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let (blit_texture, blit_bind_group) = create_blit_resources(
            &device,
            &blit_bind_group_layout,
            &blit_sampler,
            config.width,
            config.height,
        );

        let vertex_capacity = 6;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ViewKit Windows GPU vertices"),
            size: (vertex_capacity * mem::size_of::<Vertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            blit_pipeline,
            blit_texture,
            blit_bind_group,
            vertex_buffer,
            vertex_capacity,
            viewport,
        })
    }

    fn ensure_vertex_capacity(&mut self, vertex_count: usize) {
        if vertex_count <= self.vertex_capacity {
            return;
        }

        self.vertex_capacity = vertex_count.next_power_of_two();
        self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ViewKit Windows GPU vertices"),
            size: (self.vertex_capacity * mem::size_of::<Vertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }

    /// Presents a complete premultiplied RGBA8 frame through the GPU
    /// swapchain. This preserves GPU presentation for display-list commands
    /// which still use the complete software rasterizer.
    pub fn present_rgba8(&mut self, pixels: &[u8]) -> Result<(), GpuRendererError> {
        let expected = self.config.width as usize * self.config.height as usize * 4;
        if pixels.len() != expected {
            return Err(GpuRendererError::SurfaceTexture(
                "rasterized frame size does not match the GPU surface",
            ));
        }
        self.queue.write_texture(
            self.blit_texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.config.width * 4),
                rows_per_image: Some(self.config.height),
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );
        let Some((frame, view)) = self.acquire_frame()? else {
            return Ok(());
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ViewKit desktop blit encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ViewKit desktop blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &self.blit_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }

    fn acquire_frame(
        &self,
    ) -> Result<Option<(wgpu::SurfaceTexture, wgpu::TextureView)>, GpuRendererError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(GpuRendererError::SurfaceTexture(
                    "surface texture validation failed",
                ));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Some((frame, view)))
    }
}

impl Renderer for GpuRenderer {
    type Error = GpuRendererError;

    fn resize(&mut self, viewport: Viewport) -> Result<(), Self::Error> {
        self.viewport = viewport;
        self.config.width = viewport.physical_width.max(1);
        self.config.height = viewport.physical_height.max(1);
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ViewKit desktop blit sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let layout = self.blit_pipeline.get_bind_group_layout(0);
        (self.blit_texture, self.blit_bind_group) = create_blit_resources(
            &self.device,
            &layout,
            &sampler,
            self.config.width,
            self.config.height,
        );
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    fn render(
        &mut self,
        display_list: &DisplayList,
        _dirty_bounds: Rect,
    ) -> Result<(), Self::Error> {
        let mut clear = Color::TRANSPARENT;
        let mut vertices = Vec::new();

        for command in display_list.commands() {
            match command {
                DrawCommand::Clear { color } => clear = *color,
                DrawCommand::FillRect { rect, color } => {
                    push_rect(&mut vertices, *rect, *color, self.viewport);
                }
                DrawCommand::StrokeRect { rect, color, width } => {
                    push_stroke_rect(&mut vertices, *rect, *width, *color, self.viewport);
                }
                _ => return Err(GpuRendererError::UnsupportedCommand),
            }
        }

        self.ensure_vertex_capacity(vertices.len());
        if !vertices.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(GpuRendererError::SurfaceTexture(
                    "surface texture validation failed",
                ));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ViewKit Windows GPU encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ViewKit Windows GPU render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(to_wgpu_color(clear)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if !vertices.is_empty() {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);

        Ok(())
    }
}

fn create_blit_resources(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ViewKit raster fallback texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ViewKit desktop blit bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    (texture, bind_group)
}

fn push_stroke_rect(
    vertices: &mut Vec<Vertex>,
    rect: Rect,
    width: f32,
    color: Color,
    viewport: Viewport,
) {
    if !width.is_finite() || width <= 0.0 {
        return;
    }

    let half = width * 0.5;
    push_rect(
        vertices,
        Rect::new(
            rect.origin.x - half,
            rect.origin.y - half,
            rect.size.width + width,
            width,
        ),
        color,
        viewport,
    );
    push_rect(
        vertices,
        Rect::new(
            rect.origin.x - half,
            rect.origin.y + rect.size.height - half,
            rect.size.width + width,
            width,
        ),
        color,
        viewport,
    );
    push_rect(
        vertices,
        Rect::new(
            rect.origin.x - half,
            rect.origin.y + half,
            width,
            (rect.size.height - width).max(0.0),
        ),
        color,
        viewport,
    );
    push_rect(
        vertices,
        Rect::new(
            rect.origin.x + rect.size.width - half,
            rect.origin.y + half,
            width,
            (rect.size.height - width).max(0.0),
        ),
        color,
        viewport,
    );
}

fn push_rect(vertices: &mut Vec<Vertex>, rect: Rect, color: Color, viewport: Viewport) {
    let Some(rect) = rect.intersection(viewport.logical_bounds()) else {
        return;
    };

    if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
        return;
    }

    let left = rect.origin.x;
    let top = rect.origin.y;
    let right = rect.origin.x + rect.size.width;
    let bottom = rect.origin.y + rect.size.height;
    let color = premultiplied_color(color);

    let p = [
        to_ndc(left, top, viewport),
        to_ndc(right, top, viewport),
        to_ndc(right, bottom, viewport),
        to_ndc(left, bottom, viewport),
    ];

    vertices.extend_from_slice(&[
        Vertex {
            position: p[0],
            color,
        },
        Vertex {
            position: p[1],
            color,
        },
        Vertex {
            position: p[2],
            color,
        },
        Vertex {
            position: p[0],
            color,
        },
        Vertex {
            position: p[2],
            color,
        },
        Vertex {
            position: p[3],
            color,
        },
    ]);
}

fn to_ndc(x: f32, y: f32, viewport: Viewport) -> [f32; 2] {
    [
        x / viewport.logical_size.width.max(1.0) * 2.0 - 1.0,
        1.0 - y / viewport.logical_size.height.max(1.0) * 2.0,
    ]
}

fn premultiplied_color(color: Color) -> [f32; 4] {
    let alpha = f32::from(color.alpha) / 255.0;
    [
        f32::from(color.red) / 255.0 * alpha,
        f32::from(color.green) / 255.0 * alpha,
        f32::from(color.blue) / 255.0 * alpha,
        alpha,
    ]
}

fn to_wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(color.red) / 255.0,
        g: f64::from(color.green) / 255.0,
        b: f64::from(color.blue) / 255.0,
        a: f64::from(color.alpha) / 255.0,
    }
}
