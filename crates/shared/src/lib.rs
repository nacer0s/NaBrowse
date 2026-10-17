//! Shared geometry and core types for NaBrowse.
//!
//! This crate contains the fundamental building blocks used across the
//! browser: points, sizes, rectangles, colors, edge insets, and URLs. No
//! external dependencies — pure Rust.

pub mod url;
pub use url::NabrowseUrl;

/// A 2D point with f32 precision.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// A 2D size with f32 precision.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// A rectangle defined by an origin point and a size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    pub fn zero() -> Self {
        Self::new(Point::zero(), Size::zero())
    }

    /// The minimum X (left) edge.
    pub fn min_x(&self) -> f32 {
        self.origin.x
    }

    /// The minimum Y (top) edge.
    pub fn min_y(&self) -> f32 {
        self.origin.y
    }

    /// The maximum X (right) edge.
    pub fn max_x(&self) -> f32 {
        self.origin.x + self.size.width
    }

    /// The maximum Y (bottom) edge.
    pub fn max_y(&self) -> f32 {
        self.origin.y + self.size.height
    }

    /// Returns `true` if the rectangle has zero area.
    pub fn is_empty(&self) -> bool {
        self.size.width <= 0.0 || self.size.height <= 0.0
    }

    /// Returns the intersection of two rectangles, or `None` if they do not
    /// overlap.
    pub fn intersection(&self, other: Rect) -> Option<Rect> {
        let min_x = self.min_x().max(other.min_x());
        let min_y = self.min_y().max(other.min_y());
        let max_x = self.max_x().min(other.max_x());
        let max_y = self.max_y().min(other.max_y());

        if min_x < max_x && min_y < max_y {
            Some(Rect::new(
                Point::new(min_x, min_y),
                Size::new(max_x - min_x, max_y - min_y),
            ))
        } else {
            None
        }
    }
}

/// RGBA color with values in the range 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// NaBrowse default background color (a neutral off-white).
    pub fn nabrowse_bg() -> Self {
        Self::new(0.95, 0.95, 0.97, 1.0)
    }

    /// Convert to a `[f32; 4]` for use with wgpu.
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Edge insets (margins/padding) with f32 precision.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeInsets {
    pub top: f32,
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
}

impl EdgeInsets {
    pub fn new(top: f32, left: f32, bottom: f32, right: f32) -> Self {
        Self { top, left, bottom, right }
    }

    pub fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Horizontal inset (left + right).
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Vertical inset (top + bottom).
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}
