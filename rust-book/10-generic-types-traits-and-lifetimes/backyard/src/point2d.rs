use std::fmt;
use crate::r#trait::Distance;

pub struct Point2D<X, Y> {
    pub x: X,
    pub y: Y
}

impl<X, Y> Point2D<X, Y> {
    pub fn new(x: X, y: Y) -> Self {
        Self {x, y}
    }
}

impl<X, Y> Distance for Point2D<X, Y>
where
    X: Into<f64> + Copy,
    Y: Into<f64> + Copy,
{
    fn distance(&self) -> f64 {
        let (x, y) = (self.x.into(), self.y.into());
        let d_sqr = x.powi(2) + y.powi(2);
        d_sqr.sqrt()
    }
}

impl<X, Y> fmt::Display for Point2D<X, Y>
where
    X: fmt::Display,
    Y: fmt::Display,

{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(x={},y={})", self.x, self.y)
    }
}