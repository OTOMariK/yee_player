use zerocopy::{AsBytes, FromBytes};

#[derive(Debug, Clone, Copy, AsBytes, FromBytes)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

const VERTEX: [Vertex; 4] = [
    Vertex {
        position: [0.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0],
        uv: [1.0, 0.0],
    },
];

const INDECES: [u16; 6] = [1, 2, 0, 0, 2, 3];

#[derive(Debug, Clone, Copy, AsBytes, FromBytes)]
#[repr(C)]
pub struct Transform {
    pub location: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            location: [0.0; 2],
            size: [1.0, 1.0],
            color: [1.0; 3],
        }
    }
}
use std::path::PathBuf;
use wgpu::util::DeviceExt;

pub struct PiplineSetting {
    pub shader_path: PathBuf,
}

pub struct Renderer {
    surface: wgpu::Surface,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn init<W: raw_window_handle::HasRawWindowHandle>(
        window: &W,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };

        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .ok_or("no adapter found!")?;

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .map_err(|e| format!("can not request device: {}", e))?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[],
        });

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &surface_config);

        let (vertex_buffer, index_buffer) = {
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: VERTEX.as_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: INDECES.as_bytes(),
                usage: wgpu::BufferUsages::INDEX,
            });
            (vb, ib)
        };

        Ok(Renderer {
            surface,
            _adapter: adapter,
            device,
            queue,
            surface_config,
            vertex_buffer,
            index_buffer,
            bind_group,
            bind_group_layout,
        })
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn create_render_pipline(
        &self,
        setting: &PiplineSetting,
    ) -> Result<wgpu::RenderPipeline, String> {
        let shader_module = {
            let source_string =
                std::fs::read_to_string(&setting.shader_path).map_err(|e| e.to_string())?;
            self.device
                .create_shader_module(&wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&source_string)),
                })
        };

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&self.bind_group_layout],
                push_constant_ranges: &[],
            });
        Ok(self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0,
                                    format: wgpu::VertexFormat::Float32x2,
                                    shader_location: 0,
                                },
                                wgpu::VertexAttribute {
                                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                                    format: wgpu::VertexFormat::Float32x2,
                                    shader_location: 1,
                                },
                            ],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Transform>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0,
                                    format: wgpu::VertexFormat::Float32x2,
                                    shader_location: 2,
                                },
                                wgpu::VertexAttribute {
                                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                                    format: wgpu::VertexFormat::Float32x2,
                                    shader_location: 3,
                                },
                                wgpu::VertexAttribute {
                                    offset: (std::mem::size_of::<[f32; 2]>()
                                        + std::mem::size_of::<[f32; 2]>())
                                        as wgpu::BufferAddress,
                                    format: wgpu::VertexFormat::Float32x3,
                                    shader_location: 4,
                                },
                            ],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[self.surface_config.format.into()],
                }),
                multiview: None,
            }))
    }

    pub fn render(
        &mut self,
        transforms: &[Transform],
        render_pipeline: &wgpu::RenderPipeline,
    ) -> Result<(), String> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("{:?}", e))?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let transform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transforms buffer"),
                contents: transforms.as_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            });

        {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, transform_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..INDECES.len() as u32, 0, 0..transforms.len() as u32);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}
