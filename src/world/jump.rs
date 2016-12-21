use geometry::polygon::Polygon;

// This little struct makes me realize we need two things in the game:
// - force rather than acceleration
// - 'delta time' sent to all update functions.


pub struct Jump {
    progress: u32,
    // config
    frames: u32,  // Number of frames to apply
    force: f32,   // Force to apply every frame (for now just acceleration)

}

impl Jump {
    pub fn new(frames: u32, force: f32) -> Jump {
        Jump {
            progress: 0,
            frames: frames,
            force: force,
        }
    }

    /// Returns acceleration upward for this frame
    /// Returns None if it's done.
    pub fn tick(&mut self) -> Option<f32> {
        self.progress += 1;
        if self.progress <= self.frames {
            Some(self.force)
        } else {
            None
        }
    }

    pub fn get_progress(&self) -> u32 {
        self.progress
    }
}
