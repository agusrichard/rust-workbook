//! The light rule as a physical spot unit runs it.

use serde::{Deserialize, Serialize};

/// What the unit's light shows, reported in every event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Light {
    Green,
    Red,
    Blue,
    Off,
}

/// What the server asks for, learned from heartbeat replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Override {
    Blue,
    Off,
}

/// Priority order: maintenance, then a car, then a reservation, then free.
/// A car in a reserved spot shows red, and maintenance always turns it off.
pub fn expected_light(occupied: bool, override_color: Option<Override>) -> Light {
    match (override_color, occupied) {
        (Some(Override::Off), _) => Light::Off,
        (_, true) => Light::Red,
        (Some(Override::Blue), false) => Light::Blue,
        (None, false) => Light::Green,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_light_rule_order_is_preserved() {
        assert_eq!(expected_light(false, None), Light::Green);
        assert_eq!(expected_light(false, Some(Override::Blue)), Light::Blue);
        assert_eq!(expected_light(true, Some(Override::Blue)), Light::Red);
        assert_eq!(expected_light(true, Some(Override::Off)), Light::Off);
    }
}
