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
use gfx_hal::Instance;
use winit;

const DIMS: Extent2D = Extent2D {
    width: 1024,
    height: 768,
};

fn main() {
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
    let (mut adapters, mut surface) = {
        let window = {
            let builder =
                back::config_context(back::glutin::ContextBuilder::new(), ColorFormat::SELF, None)
                    .with_vsync(true);
            back::glutin::GlWindow::new(wb, builder, &events_loop).unwrap()
        };

        let surface = back::Surface::from_window(window);
        let adapters = surface.enumerate_adapters();
        (adapters, surface)
    };

    let mut draw = Draw::new(&mut surface);
    let mut tri = draw.create_static_white_2d_triangle(&[-0.5, 0.5, -0.5, -0.5, 0.5, 0.0]);
    loop {
        for i in 0..100 {
            let image = draw.acquire_swapchain_image();
            // println!["{:?}", image];
            if let Some(image) = image {
                draw.clear(image, i as f32 / 100f32 );
                // let triangle = Triangle {
                //     points: [[-0.5, (i as f32 / 100.0f32)], [-0.5, -0.5], [0.0, 0.0]],
                // };
                // draw.draw_triangle(image, triangle);
                tri.draw(&mut draw, image);
                // draw.render(image);
                draw.swap_it(image);
            }
        }
    }

    // The ideal API:
    // Draw::draw_texture(surface, filename, matrix)
    // This is too slow, as we'd need to do a hashmap lookup for the filename to bind the
    // correct vertex buffer. Bad.

    // The semi-ideal, optimal API
    // let mut draw = Draw::new(surface);
    // let sprite = draw.create_sprite(filename, scale);
    // draw.draw_sprite(&sprite, matrix);

    // Or, when instanced:
    // draw.draw_sprite_many(&sprite, &[matrix]);
}
