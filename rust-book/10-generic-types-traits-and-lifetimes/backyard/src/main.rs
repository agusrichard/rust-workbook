mod r#trait;
pub mod point3d;
pub mod point2d;

use point3d::Point3D;
use point2d::Point2D;
use crate::r#trait::Distance;

fn find_largest<T: PartialOrd>(list: &[T]) -> Option<&T> {
    if list.len() == 0 {
        return None
    }

    let mut largest = &list[0];
    for i in list {
        if i > largest {
            largest = i;
        }
    }

    Some(largest)
}

fn point_distance(point: &impl Distance) -> f64 {
    point.distance()
}

fn trait_bound_point_distance<T: Distance>(point: &T) -> f64 {
    point.distance()
}

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

fn lifetime() {

}

fn main() {
    // let v1 = vec![1, 2, 3, 4, 5];
    // let v1_result = find_largest(&v1).unwrap_or(&0);
    // println!("{v1_result}");
    //
    // let p1 = Point3D::new(10, 10.0, 10u32);
    // println!("p1=>{p1}");
    // println!("p1 distance = {}", p1.distance());
    // let p2 = Point2D::new(10, 10);
    // println!("p2=>{p2}");
    // println!("p2 distance = {}", p2.distance());
    // println!("p2 distance = {}", point_distance(&p2));
    // println!("p2=>{p2}");

    lifetime();
}