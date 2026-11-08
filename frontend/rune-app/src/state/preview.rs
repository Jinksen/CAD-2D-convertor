//! Viewport state shared by the 2D drawing and 3D preview.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewportMode {
    #[default]
    TwoD,
    ThreeD,
}

/// Placeholder orbit camera for the 3D preview agent to fill in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            yaw: 0.6,
            pitch: 0.4,
            distance: 1.0,
        }
    }
}

#[derive(Default)]
pub struct PreviewState {
    pub mode: ViewportMode,
    #[allow(dead_code)] // used by the 3D preview and section features in phase 3
    pub camera: Camera,
}
