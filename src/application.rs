use std::time::{Duration, Instant};
use wgpu::{Surface, Device, Queue, SurfaceConfiguration, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, PipelineLayoutDescriptor, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferSize, BindGroupDescriptor, PipelineLayout, ComputePipeline, ComputePassDescriptor, VertexAttribute};
use winit::window::Window;
use winit::event::{WindowEvent, KeyboardInput, VirtualKeyCode};
use crate::boid::Boid;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use bytemuck::{Contiguous, Zeroable, Pod};
use crate::camera::{Camera, CameraUniform, CameraController};
use arr_macro::arr;
#[derive(Clone, Debug)]
pub struct SimulationParams{
    pub(crate) separation_reach: f32,
    pub(crate) separation_scale: f32,
    pub(crate) alignement_reach: f32,
    pub(crate) alignement_scale: f32,
    pub(crate) cohesion_reach: f32,
    pub(crate) cohesion_scale: f32,
    pub(crate) color_mult: f32,
    pub(crate) step_mult:f32,
    pub(crate) center_attraction: f32
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimuUniforms {
    delta_time: f32,
    separation_reach: f32,
    separation_scale: f32,
    alignement_reach: f32,
    alignement_scale: f32,
    cohesion_reach: f32,
    cohesion_scale: f32,
    color_mult: f32,
    center_attraction: f32,
}

impl SimulationParams{
    fn create_uniforms(&self, delta_time: f32)-> SimuUniforms{
        SimuUniforms{
            delta_time,
            separation_reach: self.separation_reach,
            separation_scale: self.separation_scale,
            alignement_reach: self.alignement_reach,
            alignement_scale: self.alignement_scale,
            cohesion_reach: self.cohesion_reach,
            cohesion_scale: self.cohesion_scale,
            color_mult: self.color_mult,
            center_attraction: self.center_attraction
        }
    }
}

impl SimuUniforms{
    fn update(&mut self, delta_time: f32){
        self.delta_time = delta_time;
    }
}

const BOID_VERTICES: &[[f32; 2]] = &[
    [0.0, 0.1],
    [-0.045, -0.1],
    [0.0, -0.065],
    [0.045, -0.1],
];

// const BOID_VERTICES: &[f32] = &[-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];

const BOID_TRIANGLE: &[u16] = &[
    0,1,2,
    2,3,0
];


pub struct ApplicationState{
    // WGPU related fields
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    compute_pipeline: ComputePipeline,
    camera:Camera,
    camera_uniform:CameraUniform,
    camera_bind_group:wgpu::BindGroup,
    boid_bind_groups:Vec<wgpu::BindGroup>,
    simu_uniform:SimuUniforms,
    workgroup_count:u32,

    //Buffers
    boid_vertex_buffer: wgpu::Buffer,
    boid_triangle_buffer: wgpu::Buffer,
    boid_buffers: Vec<wgpu::Buffer>,
    camera_buffer:wgpu::Buffer,
    params_buffer:wgpu::Buffer,


    // Application Related fields
    simulation_params: SimulationParams,
    boid_count: u32,
    camera_controller:CameraController,
    previous_update:Instant,
    frame:u32,

}

impl ApplicationState{
    pub async fn init(window:&Window, simulation_params :SimulationParams)->Self{
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(&ShaderModuleDescriptor{
            label: Some("RenderBoids"),
            source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into())
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("CameraBindGroup"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<CameraUniform>() as u64)
                    },
                    count: None
                }
            ]
        });

        let camera = Camera::new();
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, size);
        dbg!(camera_uniform);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );


        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("RenderPipelineLayout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[]
        });
        
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor{
            label: Some("RenderPipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module:&shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout{
                        array_stride: std::mem::size_of::<Boid>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![ 0=>Float32x2, 1=>Float32x2, 2=>Float32x3]
                    },
                    wgpu::VertexBufferLayout{
                        array_stride: 2 * 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![ 3=>Float32x2 ]
                    }
                ]
            },
            primitive:  wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState{
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState{
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                }]
            }),
            multiview: None
        });


        let boid_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(BOID_VERTICES),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        let boid_triangle_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(BOID_TRIANGLE),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            };
        );

        let initial_boid= &arr![Boid::rand_new();1000];
        let boid_count = initial_boid.len() as u32;


        let simu_uniform = simulation_params.create_uniforms(0.0);
        let params_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Simu params buffer"),
            contents: bytemuck::cast_slice(&[simu_uniform]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM
        });

        let mut boid_buffers = vec![];
        for _ in 0..2 {
            boid_buffers.push(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Boid Buffer"),
                    contents: bytemuck::cast_slice(initial_boid),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                }
            ));
        }

        let boid_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Boid Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<SimuUniforms>() as u64)
                    },
                    count: None
                },
                BindGroupLayoutEntry{
                    binding:1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Boid>() as u64 * boid_count as u64)
                    },
                    count: None
                },
                BindGroupLayoutEntry{
                    binding:2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Boid>() as u64 * boid_count as u64)
                    },
                    count: None
                }
            ]
        });


        let mut boid_bind_groups = vec![];
        for i in 0..2 {
            boid_bind_groups.push(device.create_bind_group(&BindGroupDescriptor{
                label: Some(&*format!("Boid binding group {}", i)),
                layout: &boid_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{ binding: 0, resource: params_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry{ binding: 1, resource: boid_buffers[i].as_entire_binding() },
                    wgpu::BindGroupEntry{ binding: 2, resource: boid_buffers[(i+1)%2].as_entire_binding() },
                ]
            }));
        }

        let camera_controller = CameraController::new(0.1, 0.05);

        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor{
            label: Some("Compute Pipeline Layout"),
            bind_group_layouts: &[&boid_bind_group_layout],
            push_constant_ranges: &[]
        });

        let compute_shader = device.create_shader_module(&ShaderModuleDescriptor{
            label: Some("StepBoids"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into())
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "step"
        });

        let workgroup_count = ((boid_count as f32) / (64 as f32)).ceil() as u32;


        Self {
            surface,
            device,
            queue,
            config,
            size,
            simulation_params,
            render_pipeline,
            compute_pipeline,
            camera,
            camera_uniform,
            camera_bind_group,
            boid_bind_groups,
            simu_uniform,
            workgroup_count,
            boid_vertex_buffer,
            boid_triangle_buffer,
            boid_buffers,
            boid_count,
            camera_buffer,
            camera_controller,
            params_buffer,
            previous_update: Instant::now(),
            frame:0
        }

    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            //Updating surface
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Updating the camera
            self.camera_uniform.update_view_proj(&self.camera,self.size);
            self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]))
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let frame = self.frame;
        let delta_time = (now-self.previous_update).as_secs_f32();
        self.previous_update = now;
        self.frame+=1;

        print!("\x1B[0K\x1B[GFrame : {}, Delta T : {:4}", frame, delta_time);

        if self.camera_controller.update_camera(&mut self.camera){
            self.camera_uniform.update_view_proj(&self.camera, self.size);
            self.queue.write_buffer(&self.camera_buffer,0, bytemuck::cast_slice(&[self.camera_uniform]))
        }

        self.simu_uniform.update(delta_time * 2.0);
        self.queue.write_buffer(&self.params_buffer, 0 , bytemuck::cast_slice(&[self.simu_uniform]));
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{
            label:Some("Compute Encoder")
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor{ label: None });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0,&self.boid_bind_groups[(frame % 2) as usize],&[]);
            compute_pass.dispatch(self.workgroup_count,1, 1)
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError>{
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.boid_buffers[0].slice(..));
            render_pass.set_vertex_buffer(1, self.boid_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.boid_triangle_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6,0,0..self.boid_count);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        std::thread::sleep(Duration::from_millis(16));

        Ok(())
    }
}