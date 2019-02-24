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
    let mut tri2 = draw.create_static_white_2d_triangle(&[0.5, -0.5, 0.5, 0.5, -0.5, 0.0]);
    let mut tex = draw.create_static_texture_2d_rectangle();
    loop {
        for i in 0..100 {
            let mut canvas = draw.prepare_canvas();

            tri.draw(&mut canvas);
            tri2.draw(&mut canvas);
            tex.draw(&mut canvas);

            // Swap the frame
            let frame = canvas.frame;
            std::mem::drop(canvas);
            draw.swap_it(frame);
        }
    }
}
