use std::{
    fmt,
    ops::{Add, AddAssign, Mul, Neg, Sub},
};

const THRESHOLD: f32 = 0.000_001;

#[derive(Clone, Copy, Debug, Default)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn checked_div(self, scalar: f32) -> Option<Self> {
        if scalar == 0.0 {
            None
        } else {
            Some(Self::new(self.x / scalar, self.y / scalar))
        }
    }

    pub fn magnitude_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn magnitude(self) -> f32 {
        self.magnitude_squared().sqrt()
    }

    pub fn copy(self) -> Self {
        self
    }

    pub fn as_tuple(self) -> (f32, f32) {
        (self.x, self.y)
    }

    pub fn as_int(self) -> (i32, i32) {
        (self.x as i32, self.y as i32)
    }
}

impl PartialEq for Vector2 {
    fn eq(&self, other: &Self) -> bool {
        (self.x - other.x).abs() < THRESHOLD && (self.y - other.y).abs() < THRESHOLD
    }
}

impl Add for Vector2 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Vector2 {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Neg for Vector2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<f32> for Vector2 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self::Output {
        Self::new(self.x * scalar, self.y * scalar)
    }
}

impl fmt::Display for Vector2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}, {}>", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::Vector2;

    #[test]
    fn vector_arithmetic_matches_the_tutorial() {
        let vector = Vector2::new(3.0, 4.0);
        let other = Vector2::new(1.0, -2.0);

        assert_eq!(vector + other, Vector2::new(4.0, 2.0));
        assert_eq!(vector - other, Vector2::new(2.0, 6.0));
        assert_eq!(-vector, Vector2::new(-3.0, -4.0));
        assert_eq!(vector * 2.0, Vector2::new(6.0, 8.0));
    }

    #[test]
    fn checked_division_rejects_zero() {
        let vector = Vector2::new(8.0, 10.0);

        assert_eq!(vector.checked_div(2.0), Some(Vector2::new(4.0, 5.0)));
        assert_eq!(vector.checked_div(0.0), None);
    }

    #[test]
    fn equality_uses_a_small_threshold() {
        let lhs = Vector2::new(3.0, 4.0);
        let rhs = Vector2::new(3.000_000_4, 4.000_000_3);

        assert_eq!(lhs, rhs);
    }

    #[test]
    fn magnitude_helpers_match_3_4_5_triangle() {
        let vector = Vector2::new(3.0, 4.0);

        assert_eq!(vector.magnitude_squared(), 25.0);
        assert_eq!(vector.magnitude(), 5.0);
        assert_eq!(vector.as_tuple(), (3.0, 4.0));
        assert_eq!(vector.as_int(), (3, 4));
        assert_eq!(vector.copy(), vector);
        assert_eq!(vector.to_string(), "<3, 4>");
    }
}
