use crate::glocals::{Log, Windowing};
#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;
// use gfx_hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
use gfx_hal::{Backend, Instance};
use logger::{info, InDebug, Logger};
use winit::{Event, EventsLoop, Window};

// ---

pub fn init_window_with_vulkan(log: &mut Logger<Log>) -> Windowing {
    let events_loop = EventsLoop::new();
    let window = Window::new(&events_loop).unwrap();
    let version = 1;
    let vk_inst = back::Instance::create("renderer", version);
    let surf: <back::Backend as Backend>::Surface = vk_inst.create_surface(&window);
    let adapters = vk_inst.enumerate_adapters();
    let len = adapters.len();
    info![log, "vxdraw", "Adapters found"; "count" => len];
    for (idx, adap) in adapters.iter().enumerate() {
        let info = adap.info.clone();
        info![log, "vxdraw", "Adapter found"; "idx" => idx, "info" => InDebug(&info)];
    }
    Windowing {
        surf,
        vk_inst,
        events_loop,
        window,
    }
}

pub fn collect_input(windowing: &mut Windowing) -> Vec<Event> {
    let mut inputs = vec![];
    windowing.events_loop.poll_events(|evt| {
        inputs.push(evt);
    });
    inputs
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_window_and_get_input() {
        let mut logger = Logger::spawn();
        let mut windowing = init_window_with_vulkan(&mut logger);
        collect_input(&mut windowing);
    }
}
