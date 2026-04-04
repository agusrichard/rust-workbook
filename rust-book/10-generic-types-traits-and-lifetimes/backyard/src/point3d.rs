use std::fmt;
use crate::r#trait::Distance;


pub struct Point3D<X, Y, Z> {
    pub x: X,
    pub y: Y,
    pub z: Z
}

impl<X, Y, Z> Point3D<X, Y, Z> {
    pub fn new(x: X, y: Y, z: Z) -> Self {
        Self {x, y, z}
    }
}

impl<X, Y, Z> Distance for Point3D<X, Y, Z>
where
    X: Into<f64> + Copy,
    Y: Into<f64> + Copy,
    Z: Into<f64> + Copy,
{
    fn distance(&self) -> f64 {
        let (x, y, z) = (self.x.into(), self.y.into(), self.z.into());
        let d_sqr = x.powi(2) + y.powi(2) + z.powi(2);
        d_sqr.sqrt()
    }
}

impl<X, Y, Z> fmt::Display for Point3D<X, Y, Z>
    where
        X: fmt::Display,
        Y: fmt::Display,
        Z: fmt::Display

{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(x={},y={},z={})", self.x, self.y, self.z)
    }
}