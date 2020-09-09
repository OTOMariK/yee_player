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
    pub vertex_shader_path: PathBuf,
    pub fragment_shader_path: PathBuf,
}

pub struct Renderer {
    surface: wgpu::Surface,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
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
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };

        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: None,
            }))
            .ok_or("no adapter found!")?;

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
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

        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);
        let (vertex_buffer, index_buffer) = {
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: VERTEX.as_bytes(),
                usage: wgpu::BufferUsage::VERTEX,
            });

            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: INDECES.as_bytes(),
                usage: wgpu::BufferUsage::INDEX,
            });
            (vb, ib)
        };

        Ok(Renderer {
            surface,
            _adapter: adapter,
            device,
            queue,
            swap_chain_desc,
            swap_chain,
            vertex_buffer,
            index_buffer,
            bind_group,
            bind_group_layout,
        })
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.swap_chain_desc.width = size.width;
        self.swap_chain_desc.height = size.height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_desc);
    }

    pub fn create_render_pipline(
        &self,
        setting: &PiplineSetting,
    ) -> Result<wgpu::RenderPipeline, String> {
        let (vs_module, fs_module) = {
            let vs_file = std::fs::read(&setting.vertex_shader_path).map_err(|e| e.to_string())?;
            let vs = wgpu::util::make_spirv(&vs_file);
            let vs_module = self.device.create_shader_module(vs);

            let fs_file =
                std::fs::read(&setting.fragment_shader_path).map_err(|e| e.to_string())?;
            let fs = wgpu::util::make_spirv(&fs_file);
            let fs_module = self.device.create_shader_module(fs);
            (vs_module, fs_module)
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
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    clamp_depth: false,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[
                        wgpu::VertexBufferDescriptor {
                            stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                            step_mode: wgpu::InputStepMode::Vertex,
                            attributes: &[
                                wgpu::VertexAttributeDescriptor {
                                    offset: 0,
                                    format: wgpu::VertexFormat::Float2,
                                    shader_location: 0,
                                },
                                wgpu::VertexAttributeDescriptor {
                                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                                    format: wgpu::VertexFormat::Float2,
                                    shader_location: 1,
                                },
                            ],
                        },
                        wgpu::VertexBufferDescriptor {
                            stride: std::mem::size_of::<Transform>() as wgpu::BufferAddress,
                            step_mode: wgpu::InputStepMode::Instance,
                            attributes: &[
                                wgpu::VertexAttributeDescriptor {
                                    offset: 0,
                                    format: wgpu::VertexFormat::Float2,
                                    shader_location: 2,
                                },
                                wgpu::VertexAttributeDescriptor {
                                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                                    format: wgpu::VertexFormat::Float2,
                                    shader_location: 3,
                                },
                                wgpu::VertexAttributeDescriptor {
                                    offset: (std::mem::size_of::<[f32; 2]>()
                                        + std::mem::size_of::<[f32; 2]>())
                                        as wgpu::BufferAddress,
                                    format: wgpu::VertexFormat::Float3,
                                    shader_location: 4,
                                },
                            ],
                        },
                    ],
                },
                depth_stencil_state: None,
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            }))
    }

    pub fn render(
        &mut self,
        transforms: &[Transform],
        render_pipeline: &wgpu::RenderPipeline,
    ) -> Result<(), String> {
        let frame = self
            .swap_chain
            .get_current_frame()
            .map_err(|e| format!("{:?}", e))?
            .output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let transform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transforms buffer"),
                contents: transforms.as_bytes(),
                usage: wgpu::BufferUsage::VERTEX,
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
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
            render_pass.set_index_buffer(self.index_buffer.slice(..));
            render_pass.draw_indexed(0..INDECES.len() as u32, 0, 0..transforms.len() as u32);
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
}
