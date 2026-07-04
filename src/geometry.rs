#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.y >= self.origin.y
            && point.x < self.origin.x + self.size.width
            && point.y < self.origin.y + self.size.height
    }

    pub fn union(self, other: Self) -> Self {
        let left = self.origin.x.min(other.origin.x);
        let top = self.origin.y.min(other.origin.y);

        let right = (self.origin.x + self.size.width).max(other.origin.x + other.size.width);

        let bottom = (self.origin.y + self.size.height).max(other.origin.y + other.size.height);

        Self::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let left = self.origin.x.max(other.origin.x);
        let top = self.origin.y.max(other.origin.y);

        let right = (self.origin.x + self.size.width).min(other.origin.x + other.size.width);

        let bottom = (self.origin.y + self.size.height).min(other.origin.y + other.size.height);

        if right <= left || bottom <= top {
            return None;
        }

        Some(Self::new(left, top, right - left, bottom - top))
    }

    pub fn expanded(self, amount: f32) -> Self {
        let amount = if amount.is_finite() {
            amount.max(0.0)
        } else {
            0.0
        };

        Self::new(
            self.origin.x - amount,
            self.origin.y - amount,
            self.size.width + amount * 2.0,
            self.size.height + amount * 2.0,
        )
    }

    pub fn is_empty(self) -> bool {
        self.size.width <= 0.0 || self.size.height <= 0.0
    }
}
