//! Light colours and the rule a spot unit uses to pick one.

use serde::{Deserialize, Serialize};

/// What a spot unit's light shows (or reports showing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Light {
    Green,
    Red,
    Blue,
    Off,
}

impl Light {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "green" => Some(Self::Green),
            "red" => Some(Self::Red),
            "blue" => Some(Self::Blue),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

/// What the server asks a unit to show. Stored in `light_override`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Override {
    /// Reserved.
    Blue,
    /// Under maintenance.
    Off,
}

impl Override {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "blue" => Some(Self::Blue),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

/// The device's rule, in priority order. The server runs the same rule to
/// work out what a light *should* show, then compares with `light_reported`.
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
    fn rule_order() {
        assert_eq!(expected_light(false, None), Light::Green);
        assert_eq!(expected_light(true, None), Light::Red);
        assert_eq!(expected_light(false, Some(Override::Blue)), Light::Blue);
        // A car in a reserved spot still shows red.
        assert_eq!(expected_light(true, Some(Override::Blue)), Light::Red);
        // Maintenance always wins.
        assert_eq!(expected_light(true, Some(Override::Off)), Light::Off);
        assert_eq!(expected_light(false, Some(Override::Off)), Light::Off);
    }
}
