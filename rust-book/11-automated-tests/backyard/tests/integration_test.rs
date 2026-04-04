mod common;

use backyard::add;

#[test]
fn it_adds_two() {
    common::setup();
    assert!(add(10, 10) == 20);
}