#[cfg(feature = "dx12")]
use gfx_backend_dx12 as back;
#[cfg(feature = "gl")]
use gfx_backend_gl as back;
#[cfg(feature = "metal")]
use gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
use gfx_backend_vulkan as back;

use gfx_hal::format::{AsFormat, ChannelType, Rgba8Srgb as ColorFormat, Swizzle};
use gfx_hal::{window::Extent2D, Backend, Instance};

// ---

pub fn init_vulkan() {
    let instance = back::Instance::create("renderer", 1);
}

// ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_not_crash() {
        init_vulkan();
    }
}
