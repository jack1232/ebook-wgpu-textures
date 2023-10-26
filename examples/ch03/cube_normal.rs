use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::{Matrix, Matrix4, SquareMatrix};
use std::{iter, mem};
use wgpu::{util::DeviceExt, VertexBufferLayout};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use wgpu_simplified as ws;
use wgpu_textures::vertex_data as vd;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    tangent: [f32; 3],
    bitangent: [f32; 3],
}

fn create_vertices(ul: f32, vl: f32) -> (Vec<Vertex>, Vec<u16>) {
    let (pos, _, normal, uv, ind, _) = vd::create_cube_data(3.0);
    let tan = vd::create_tangent_data(&pos, &normal, &uv, &ind);

    let mut data: Vec<Vertex> = vec![];
    for i in 0..pos.len() {
        data.push(Vertex {
            position: pos[i],
            normal: normal[i],
            uv: [uv[i][0] * ul, uv[i][1] * vl],
            tangent: tan[i].tang,
            bitangent: tan[i].bitang,
        });
    }

    (data.to_vec(), ind)
}

const ADDRESS_MODE: wgpu::AddressMode = wgpu::AddressMode::MirrorRepeat;

struct State {
    init: ws::IWgpuInit,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_bind_groups: Vec<wgpu::BindGroup>,
    uniform_buffers: Vec<wgpu::Buffer>,
    view_mat: Matrix4<f32>,
    project_mat: Matrix4<f32>,
    msaa_texture_view: wgpu::TextureView,
    depth_texture_view: wgpu::TextureView,
    animation_speed: f32,
    indices_len: u32,
    u_len: f32,
    v_len: f32,
    update_buffers: bool,
    gamma_correction: u32,
}

impl State {
    async fn new(window: &Window, sample_count: u32) -> Self {
        let init = ws::IWgpuInit::new(&window, sample_count, None).await;

        let vs_shader = init
            .device
            .create_shader_module(wgpu::include_wgsl!("normal_vert.wgsl"));
        let fs_shader = init
            .device
            .create_shader_module(wgpu::include_wgsl!("normal_frag.wgsl"));

        // uniform data
        let camera_position = (2.0, 3.0, 4.0).into();
        let look_direction = (0.0, 0.0, 0.0).into();
        let up_direction = cgmath::Vector3::unit_y();

        let (view_mat, project_mat, _) = ws::create_vp_mat(
            camera_position,
            look_direction,
            up_direction,
            init.config.width as f32 / init.config.height as f32,
        );

        // create vertex uniform buffers

        // model_mat and vp_mat will be stored in vertex_uniform_buffer inside the update function
        let vert_uniform_buffer = init.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Uniform Buffer"),
            size: 192,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // create light uniform buffer. here we set eye_position = camera_position and light_position = eye_position
        let vert_light_uniform_buffer = init.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Light Uniform Buffer"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let eye_position: &[f32; 3] = camera_position.as_ref();
        let light_position = eye_position;
        init.queue
            .write_buffer(&vert_light_uniform_buffer, 0, cast_slice(light_position));
        init.queue
            .write_buffer(&vert_light_uniform_buffer, 16, cast_slice(eye_position));

        // fragment light unifrom buffer
        let light_uniform_buffer = init.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fragment Light Uniform Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // set specular light color to white
        let specular_color: [f32; 3] = [1.0, 1.0, 1.0];
        init.queue.write_buffer(
            &light_uniform_buffer,
            0,
            cast_slice(specular_color.as_ref()),
        );

        // material uniform buffer
        let material_uniform_buffer = init.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Material Uniform Buffer"),
            size: 20,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // set default material parameters
        let material = [0.1 as f32, 0.4, 0.1, 5.0, 0.0];
        init.queue
            .write_buffer(&material_uniform_buffer, 0, cast_slice(&material));

        // uniform bind group for vertex shader
        let (vert_bind_group_layout, vert_bind_group) = ws::create_bind_group(
            &init.device,
            vec![wgpu::ShaderStages::VERTEX, wgpu::ShaderStages::VERTEX],
            &[
                vert_uniform_buffer.as_entire_binding(),
                vert_light_uniform_buffer.as_entire_binding(),
            ],
        );

        // uniform bind group for fragment shader
        let (frag_bind_group_layout, frag_bind_group) = ws::create_bind_group(
            &init.device,
            vec![wgpu::ShaderStages::FRAGMENT, wgpu::ShaderStages::FRAGMENT],
            &[
                light_uniform_buffer.as_entire_binding(),
                material_uniform_buffer.as_entire_binding(),
            ],
        );

        // create image texture and image texture bind group
        let img_files = vec!["assets/brick.png", "assets/brick-normal.png"];
        let (texture_bind_group_layout, texture_bind_group) = ws::create_texture_bind_group(
            &init.device,
            &init.queue,
            img_files,
            ADDRESS_MODE,
            ADDRESS_MODE,
        );

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, // position
                1 => Float32x3, // normal
                2 => Float32x2, // uv
                3 => Float32x3, // tangent
                4 => Float32x3, // bitangent
            ],
        };

        let pipeline_layout = init
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &vert_bind_group_layout,
                    &frag_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let mut ppl = ws::IRenderPipeline {
            vs_shader: Some(&vs_shader),
            fs_shader: Some(&fs_shader),
            pipeline_layout: Some(&pipeline_layout),
            vertex_buffer_layout: &[vertex_buffer_layout],
            ..Default::default()
        };
        let pipeline = ppl.new(&init);

        let msaa_texture_view = ws::create_msaa_texture_view(&init);
        let depth_texture_view = ws::create_depth_view(&init);

        let (vertex_data, index_data) = create_vertices(1.0, 1.0);

        let vertex_buffer = init
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = init
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            init,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_bind_groups: vec![vert_bind_group, frag_bind_group, texture_bind_group],
            uniform_buffers: vec![
                vert_uniform_buffer,
                light_uniform_buffer,
                material_uniform_buffer,
                vert_light_uniform_buffer,
            ],
            view_mat,
            project_mat,
            msaa_texture_view,
            depth_texture_view,
            animation_speed: 1.0,
            indices_len: index_data.len() as u32,
            u_len: 1.0,
            v_len: 1.0,
            update_buffers: false,
            gamma_correction: 0,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.init.size = new_size;
            self.init.config.width = new_size.width;
            self.init.config.height = new_size.height;
            self.init
                .surface
                .configure(&self.init.device, &self.init.config);

            self.project_mat =
                ws::create_projection_mat(new_size.width as f32 / new_size.height as f32, true);
            self.depth_texture_view = ws::create_depth_view(&self.init);
            if self.init.sample_count > 1 {
                self.msaa_texture_view = ws::create_msaa_texture_view(&self.init);
            }
        }
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(keycode),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match keycode {
                VirtualKeyCode::Space => {
                    self.gamma_correction = (self.gamma_correction + 1) % 2;
                    let material = [0.1 as f32, 0.4, 0.1, 5.0, self.gamma_correction as f32];
                    self.init.queue.write_buffer(
                        &self.uniform_buffers[2],
                        0,
                        cast_slice(&material),
                    );
                    true
                }
                VirtualKeyCode::Q => {
                    self.u_len += 0.1;
                    self.update_buffers = true;
                    true
                }
                VirtualKeyCode::A => {
                    self.u_len -= 0.1;
                    if self.u_len < 0.1 {
                        self.u_len = 0.1;
                    }
                    self.update_buffers = true;
                    true
                }
                VirtualKeyCode::W => {
                    self.v_len += 0.1;
                    self.update_buffers = true;
                    true
                }
                VirtualKeyCode::S => {
                    self.v_len -= 0.1;
                    if self.v_len < 0.1 {
                        self.v_len = 0.1;
                    }
                    self.update_buffers = true;
                    true
                }
                VirtualKeyCode::E => {
                    self.animation_speed += 0.1;
                    true
                }
                VirtualKeyCode::D => {
                    self.animation_speed -= 0.1;
                    if self.animation_speed < 0.0 {
                        self.animation_speed = 0.0;
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        // update uniform buffer
        let dt = self.animation_speed * dt.as_secs_f32();
        let model_mat = ws::create_model_mat([0.0, 0.5, 0.0], [0.0, dt, 0.0], [1.0, 1.0, 1.0]);
        let view_project_mat = self.project_mat * self.view_mat;

        let normal_mat = (model_mat.invert().unwrap()).transpose();

        let model_ref: &[f32; 16] = model_mat.as_ref();
        let view_projection_ref: &[f32; 16] = view_project_mat.as_ref();
        let normal_ref: &[f32; 16] = normal_mat.as_ref();

        self.init.queue.write_buffer(
            &self.uniform_buffers[0],
            0,
            bytemuck::cast_slice(view_projection_ref),
        );
        self.init.queue.write_buffer(
            &self.uniform_buffers[0],
            64,
            bytemuck::cast_slice(model_ref),
        );
        self.init.queue.write_buffer(
            &self.uniform_buffers[0],
            128,
            bytemuck::cast_slice(normal_ref),
        );

        // update vertex buffer
        if self.update_buffers {
            let (vertex_data, _) = create_vertices(self.u_len, self.v_len);
            self.init
                .queue
                .write_buffer(&self.vertex_buffer, 0, cast_slice(&vertex_data));
            self.update_buffers = false;
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        //let output = self.init.surface.get_current_frame()?.output;
        let output = self.init.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.init
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let color_attach = ws::create_color_attachment(&view);
            let msaa_attach = ws::create_msaa_color_attachment(&view, &self.msaa_texture_view);
            let color_attachment = if self.init.sample_count == 1 {
                color_attach
            } else {
                msaa_attach
            };
            let depth_attachment = ws::create_depth_stencil_attachment(&self.depth_texture_view);

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(color_attachment)],
                depth_stencil_attachment: Some(depth_attachment),
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, &self.uniform_bind_groups[0], &[]);
            render_pass.set_bind_group(1, &self.uniform_bind_groups[1], &[]);
            render_pass.set_bind_group(2, &self.uniform_bind_groups[2], &[]);
            render_pass.draw_indexed(0..self.indices_len, 0, 0..1);
        }

        self.init.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    window.set_title(&*format!("{}", "cube_normal"));

    let mut state = pollster::block_on(State::new(&window, 8));
    let render_start_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        Event::RedrawRequested(_) => {
            let now = std::time::Instant::now();
            let dt = now - render_start_time;
            state.update(dt);

            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.init.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    });
}