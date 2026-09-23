use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

use cosmic_text::{Buffer, FontSystem, SwashCache};
use mochios_viewkit_gpu_protocol::{ATLAS_HEIGHT, ATLAS_WIDTH, VERTEX_STRIDE, decode};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use super::gpu_scene::{GpuSceneError, GpuSceneRenderer, TextLayoutKey};
use crate::draw_command::DisplayList;
use crate::font::create_font_system;
use crate::geometry::Rect;
use crate::renderer::{Renderer, Viewport};

const SHADER: &str = include_str!("../../shaders/desktop_scene.wgsl");
const INITIAL_VERTEX_CAPACITY: usize = 6;

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
    #[error("Failed to build the GPU scene: {0}")]
    Scene(#[from] GpuSceneError),
    #[error("The GPU scene encoder produced an invalid scene")]
    InvalidScene,
}

pub struct GpuRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    viewport: Viewport,
    scene_renderer: GpuSceneRenderer,
    scene_bytes: Vec<u8>,
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_layout_cache: HashMap<TextLayoutKey, Buffer>,
}

impl GpuRenderer {
    pub fn new(
        window: Rc<Window>,
        viewport: Viewport,
        display: OwnedDisplayHandle,
    ) -> Result<Self, GpuRendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_bits_retain(
                wgpu::Backends::VULKAN.bits() | wgpu::Backends::GL.bits(),
            ),
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display))
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
                "[ViewKit] Linux GPU-only renderer: {} ({:?}, {:?})",
                info.name, info.backend, info.device_type
            );
        }
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("ViewKit Linux GPU-only device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            }))?;
        let capabilities = surface.get_capabilities(&adapter);
        // Scene colors and atlas texels are both encoded as UNORM bytes by the
        // mochiOS GPU scene protocol. An sRGB atlas would decode only texels,
        // making a tinted SVG visibly darker than an adjacent solid fill made
        // from the exact same theme color.
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
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

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ViewKit GPU scene atlas layout"),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ViewKit GPU scene shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ViewKit GPU scene pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ViewKit GPU scene pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_STRIDE as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: (mem::size_of::<[f32; 3]>()
                                + mem::size_of::<[f32; 2]>())
                                as wgpu::BufferAddress,
                            shader_location: 2,
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
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ViewKit GPU scene atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ViewKit GPU scene atlas sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ViewKit GPU scene atlas bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ViewKit GPU scene vertices"),
            size: (INITIAL_VERTEX_CAPACITY * VERTEX_STRIDE) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            atlas_texture,
            atlas_bind_group,
            vertex_buffer,
            vertex_capacity: INITIAL_VERTEX_CAPACITY,
            viewport,
            scene_renderer: GpuSceneRenderer::new(),
            scene_bytes: Vec::new(),
            font_system: create_font_system(),
            swash_cache: SwashCache::new(),
            text_layout_cache: HashMap::new(),
        })
    }

    fn ensure_vertex_capacity(&mut self, vertex_count: usize) {
        if vertex_count <= self.vertex_capacity {
            return;
        }
        self.vertex_capacity = vertex_count.next_power_of_two();
        self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ViewKit GPU scene vertices"),
            size: (self.vertex_capacity * VERTEX_STRIDE) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
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
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    fn render(
        &mut self,
        display_list: &DisplayList,
        _dirty_bounds: Rect,
    ) -> Result<(), Self::Error> {
        // The swapchain does not preserve prior contents. Rebuild the entire
        // frame on the GPU until retained render targets are introduced.
        let full_frame = self.viewport.logical_bounds();
        self.scene_renderer.render(
            self.viewport,
            full_frame,
            display_list,
            &mut self.font_system,
            &mut self.swash_cache,
            &mut self.text_layout_cache,
            false,
            &mut self.scene_bytes,
        )?;
        let vertex_count = decode(&self.scene_bytes)
            .map_err(|_| GpuRendererError::InvalidScene)?
            .vertex_count() as usize;
        self.ensure_vertex_capacity(vertex_count);
        let scene = decode(&self.scene_bytes).map_err(|_| GpuRendererError::InvalidScene)?;
        self.queue
            .write_buffer(&self.vertex_buffer, 0, scene.vertices);
        if scene.atlas_data_height > 0 {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: scene.atlas_data_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                scene.atlas,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(scene.atlas_width * 4),
                    rows_per_image: Some(scene.atlas_data_height),
                },
                wgpu::Extent3d {
                    width: scene.atlas_width,
                    height: scene.atlas_data_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let Some((frame, view)) = self.acquire_frame()? else {
            return Ok(());
        };
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ViewKit GPU scene encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ViewKit GPU scene pass"),
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.atlas_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..vertex_count as u32, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}
