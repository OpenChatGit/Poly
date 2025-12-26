//! GPU Renderer implementation using wgpu

use wgpu::*;
use crate::core::context::Color;
use crate::render::{RenderList, Primitive};

/// Vertex for 2D rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x4,  // color
        2 => Float32x2,  // uv
    ];

    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// GPU state and resources
pub struct GpuRenderer {
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub size: (u32, u32),
    pub clear_color: Color,
}

impl GpuRenderer {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();
        
        // Create wgpu instance
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        
        // Create surface
        let surface = instance.create_surface(window).unwrap();
        
        // Request adapter
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find GPU adapter");
        
        // Request device
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Poly UI Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        
        // Create shader
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Poly UI Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        
        // Create pipeline
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Poly UI Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Poly UI Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        
        // Create buffers
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1024 * 1024, // 1MB
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Index Buffer"),
            size: 256 * 1024, // 256KB
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            vertex_buffer,
            index_buffer,
            size: (size.width, size.height),
            clear_color: Color::rgb(18, 18, 18),
        }
    }
    
    pub fn resize(&mut self, new_size: (u32, u32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.size = new_size;
            self.config.width = new_size.0;
            self.config.height = new_size.1;
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    pub fn render(&mut self, render_list: &RenderList) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        
        // Build vertices from render list
        let (vertices, indices) = self.build_geometry(render_list);
        
        // Upload to GPU
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: self.clear_color.r as f64,
                            g: self.clear_color.g as f64,
                            b: self.clear_color.b as f64,
                            a: self.clear_color.a as f64,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
    
    fn build_geometry(&self, render_list: &RenderList) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let w = self.size.0 as f32;
        let h = self.size.1 as f32;
        
        for primitive in &render_list.primitives {
            match primitive {
                Primitive::Rect { x, y, width, height, color, .. } => {
                    let base = vertices.len() as u32;
                    
                    // Convert to NDC (-1 to 1)
                    let x1 = (x / w) * 2.0 - 1.0;
                    let y1 = 1.0 - (y / h) * 2.0;
                    let x2 = ((x + width) / w) * 2.0 - 1.0;
                    let y2 = 1.0 - ((y + height) / h) * 2.0;
                    
                    let c = color.to_array();
                    
                    vertices.extend_from_slice(&[
                        Vertex { position: [x1, y1], color: c, uv: [0.0, 0.0] },
                        Vertex { position: [x2, y1], color: c, uv: [1.0, 0.0] },
                        Vertex { position: [x2, y2], color: c, uv: [1.0, 1.0] },
                        Vertex { position: [x1, y2], color: c, uv: [0.0, 1.0] },
                    ]);
                    
                    indices.extend_from_slice(&[
                        base, base + 1, base + 2,
                        base, base + 2, base + 3,
                    ]);
                }
                Primitive::Circle { cx, cy, radius, color } => {
                    let base = vertices.len() as u32;
                    let segments = 32;
                    let c = color.to_array();
                    
                    // Center vertex
                    let cx_ndc = (cx / w) * 2.0 - 1.0;
                    let cy_ndc = 1.0 - (cy / h) * 2.0;
                    vertices.push(Vertex { position: [cx_ndc, cy_ndc], color: c, uv: [0.5, 0.5] });
                    
                    // Circle vertices
                    for i in 0..segments {
                        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                        let px = cx + angle.cos() * radius;
                        let py = cy + angle.sin() * radius;
                        let px_ndc = (px / w) * 2.0 - 1.0;
                        let py_ndc = 1.0 - (py / h) * 2.0;
                        vertices.push(Vertex { position: [px_ndc, py_ndc], color: c, uv: [0.0, 0.0] });
                    }
                    
                    // Triangle fan indices
                    for i in 0..segments {
                        indices.push(base);
                        indices.push(base + 1 + i);
                        indices.push(base + 1 + (i + 1) % segments);
                    }
                }
                _ => {} // Text and Image need texture support
            }
        }
        
        (vertices, indices)
    }
}
