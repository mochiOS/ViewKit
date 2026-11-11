//! Default window elevation, mirrored in compositor without a source dependency.
//! scripts/dev/check-ui-boundaries.py verifies that both copies match.
//! This file intentionally has no renderer, allocation, or OS dependencies.

/// One neutral-black elevation layer, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    /// Coverage at the centre of the shadow.
    pub alpha: u8,
    /// Vertical displacement from the window boundary.
    pub offset_y: i32,
    /// Outward falloff extent.
    pub blur_radius: u32,
    /// Constant outward displacement before falloff.
    pub spread: u32,
}
/// Contact and ambient layers for the working window; contained in the existing 20px damage margin.
pub const ACTIVE: [Layer; 2] = [
    Layer {
        alpha: 20,
        offset_y: 1,
        blur_radius: 3,
        spread: 0,
    },
    Layer {
        alpha: 32,
        offset_y: 5,
        blur_radius: 14,
        spread: 0,
    },
];
/// Lower elevation for inactive windows.
pub const INACTIVE: [Layer; 1] = [Layer {
    alpha: 10,
    offset_y: 2,
    blur_radius: 7,
    spread: 0,
}];
/// Outer edge coverage for the default light window material.
pub const BORDER_ALPHA: u8 = 24;
/// Inactive windows retain a quieter edge against adjacent content.
pub const INACTIVE_BORDER_ALPHA: u8 = 16;

/// Shared desktop surface corner setback, in logical pixels.
pub const CORNER_RADIUS: f32 = 26.5;
