use draw::{Draw, Triangle};
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
use gfx_hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
use gfx_hal::window::Extent2D;
use gfx_hal::Backend;
use gfx_hal::Instance;
use winit;

const DIMS: Extent2D = Extent2D {
    width: 1024,
    height: 768,
};

fn main() {
    // let instance = <back::Backend as Backend>::Instance;
    // for (idx, adapter) in gfx_hal::Instance::enumerate_adapters(&instance).iter().enumerate() {
    //     println!("Adapter {}: {:?}", idx, adapter.info);
    // }

    let mut events_loop = winit::EventsLoop::new();

    let wb = winit::WindowBuilder::new()
        .with_dimensions(winit::dpi::LogicalSize::new(
            DIMS.width as _,
            DIMS.height as _,
        ))
        .with_title("quad".to_string());
    // instantiate backend
    #[cfg(not(feature = "gl"))]
    let (_window, _instance, mut adapters, mut surface) = {
        let window = wb.build(&events_loop).unwrap();
        let instance = back::Instance::create("gfx-rs quad", 1);
        let surface = instance.create_surface(&window);
        let adapters = instance.enumerate_adapters();
        (window, instance, adapters, surface)
    };
    #[cfg(feature = "gl")]
    let mut surface = {
        let window = {
            let builder =
                back::config_context(back::glutin::ContextBuilder::new(), ColorFormat::SELF, None)
                    .with_vsync(true);
            back::glutin::GlWindow::new(wb, builder, &events_loop).unwrap()
        };

        let surface = back::Surface::from_window(window);
        // let adapters = surface.enumerate_adapters();
        surface
    };

    let (device, queue_group, adapter) = Draw::open_device(&mut surface, &mut adapters);
    let mut draw = Draw::new(&mut surface, &device, queue_group, adapter);
    let mut tri = draw.create_static_white_2d_triangle(&device, &[-0.5, 0.5, -0.5, -0.5, 0.5, 0.0]);
    let mut tri2 = draw.create_static_white_2d_triangle(&device, &[0.5, -0.5, 0.5, 0.5, -0.5, 0.0]);
    let mut tex = draw.create_static_texture_2d_rectangle(&device);
    let mut bullets = draw.create_bullets(&device, include_bytes!["data/logo.png"]);
    let mut bullets2 = draw.create_bullets(&device, include_bytes!["data/pagliacci.png"]);
    draw.create_dynamic_binary_texture(&device, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    bullets.upload(&[0.0, 0.0, 1.0, 0.5, 0.0, 0.3]);
    println!["COOL"];
    bullets.upload(&[-0.2, 0.5, 0.0, 0.0, 0.0, 1.0, 0.5, 0.0, 0.3]);
    use rand::distributions::uniform::UniformFloat;
    use rand::prelude::*;
    let mut rng = rand::thread_rng();
    let mut vec: Vec<f32> = vec![];
    for i in 0..300 {
        let ii = i as f32;
        vec.push(rng.gen::<f32>() * 2.0 - 1.0);
        vec.push(rng.gen::<f32>() * 2.0 - 1.0);
        vec.push(rng.gen::<f32>() * 3.14159);
    }
    bullets.upload(&vec[..]);
    use draw::Canvas;
    println!["OK"];
    loop {
        for i in 0..100 {
            println!["BEFORE"];
            let mut canvas = draw.prepare_canvas();
            println!["NICE GOT"];

            tri.draw(&mut canvas);
            tri2.draw(&mut canvas);
            bullets.draw(&mut canvas);
            // tex.draw(&mut canvas);
        }
    }
}
