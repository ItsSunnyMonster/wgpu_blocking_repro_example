use std::time::{Duration, Instant, SystemTime};
use env_logger::Env;
use log::{error, info, trace};
use wgpu::{Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Instance, InstanceDescriptor, PresentMode, Queue, RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError, TextureViewDescriptor};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    // Logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Set up winit
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Set up wgpu
    let instance = Instance::new(InstanceDescriptor::default());

    let surface = instance.create_surface(&window).unwrap();

    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: Default::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) =
        pollster::block_on(adapter.request_device(&DeviceDescriptor::default(), None)).unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter()
        .copied()
        .filter(|f| f.is_srgb())
        .next()
        .unwrap_or(surface_caps.formats[0]);
    let mut config = SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run(|event, target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    target.exit();
                }
                WindowEvent::Resized(size) => {
                    resize_surface(&surface, size, &mut config, &device);
                }
                _ => {}
            },
            Event::AboutToWait => {
                match render(&surface, &device, &queue) {
                    Ok(_) => {}
                    Err(SurfaceError::Lost) => resize_surface(&surface, &window.inner_size(), &mut config, &device),
                    Err(SurfaceError::OutOfMemory) => target.exit(),
                    Err(e) => error!("Surface error {:?}", e),
                }
            }
            _ => {}
        })
        .unwrap();
}

fn resize_surface(surface: &Surface, size: &PhysicalSize<u32>, config: &mut SurfaceConfiguration, device: &Device) {
    if size.width > 0 && size.height > 0 {
        config.width = size.width;
        config.height = size.height;
        surface.configure(device, config);
        info!("Resized {} {}", config.width, config.height);
    }
}

fn render(surface: &Surface, device: &Device, queue: &Queue) -> Result<(), SurfaceError>{

    let timeout = Duration::from_millis(500);

    let timer_start = Instant::now();
    let output = surface.get_current_texture()?;
    if timer_start.elapsed() > timeout {
        error!("Get current texture took {}ms", timer_start.elapsed().as_millis());
    }

    let view = output.texture.create_view(&TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    {
        let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.6509803921568628,
                        g: 0.8901960784313725,
                        b: 0.6313725490196078,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    queue.submit(std::iter::once(encoder.finish()));
    trace!("Present");
    output.present();
    
    Ok(())
}
