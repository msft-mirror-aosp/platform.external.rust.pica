// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::uci_packets::PicaPosition;
use glam::{EulerRot, Quat, Vec3};
use serde::{
    de::{self, Deserializer, MapAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
    Deserialize,
};
use std::convert::From;
use std::default::Default;

#[derive(Debug, Clone, Copy)]
pub struct Position {
    position: Vec3,
    rotation: Quat,
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Position", 5)?;
        state.serialize_field("x", &(self.position.x as i16))?;
        state.serialize_field("y", &(self.position.y as i16))?;
        state.serialize_field("z", &(self.position.z as i16))?;

        let (roll, pitch, yaw) = self.rotation.to_euler(EulerRot::ZXY);

        state.serialize_field("yaw", &(yaw.to_degrees().round() as i16))?;
        state.serialize_field("pitch", &(pitch.to_degrees().round() as i8))?;
        state.serialize_field("roll", &(roll.to_degrees().round() as i16))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(non_camel_case_types)]
        enum Field {
            x,
            y,
            z,
            yaw,
            pitch,
            roll,
        }

        struct PositionVisitor;

        impl<'de> Visitor<'de> for PositionVisitor {
            type Value = Position;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Struct Position")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Position, V::Error>
            where
                V: MapAccess<'de>,
            {
                let (mut x, mut y, mut z, mut yaw, mut pitch, mut roll) =
                    (None, None, None, None, None, None);
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::x => {
                            if x.is_some() {
                                return Err(de::Error::duplicate_field("x"));
                            }
                            x = Some(map.next_value()?);
                        }
                        Field::y => {
                            if y.is_some() {
                                return Err(de::Error::duplicate_field("y"));
                            }
                            y = Some(map.next_value()?);
                        }
                        Field::z => {
                            if z.is_some() {
                                return Err(de::Error::duplicate_field("z"));
                            }
                            z = Some(map.next_value()?);
                        }
                        Field::yaw => {
                            if yaw.is_some() {
                                return Err(de::Error::duplicate_field("yaw"));
                            }
                            yaw = Some(map.next_value()?);
                        }
                        Field::pitch => {
                            if pitch.is_some() {
                                return Err(de::Error::duplicate_field("pitch"));
                            }
                            pitch = Some(map.next_value()?);
                        }
                        Field::roll => {
                            if roll.is_some() {
                                return Err(de::Error::duplicate_field("roll"));
                            }
                            roll = Some(map.next_value()?);
                        }
                    }
                }
                let x = x.ok_or_else(|| de::Error::missing_field("x"))?;
                let y = y.ok_or_else(|| de::Error::missing_field("y"))?;
                let z = z.ok_or_else(|| de::Error::missing_field("z"))?;
                let yaw = yaw.ok_or_else(|| de::Error::missing_field("yaw"))?;
                let pitch = pitch.ok_or_else(|| de::Error::missing_field("pitch"))?;
                let roll = roll.ok_or_else(|| de::Error::missing_field("roll"))?;
                Ok(Position::new(x, y, z, yaw, pitch, roll))
            }
        }
        const FIELDS: &'static [&'static str] = &["x", "y", "z", "yaw", "pitch", "roll"];
        deserializer.deserialize_struct("Position", FIELDS, PositionVisitor)
    }
}

fn checked_div(num: f32, den: f32) -> Option<f32> {
    if den == 0. {
        None
    } else {
        Some(num / den)
    }
}

fn azimuth(delta: Vec3) -> f32 {
    checked_div(delta.x, delta.z).map_or(
        if delta.x == 0. {
            0.
        } else {
            delta.x.signum() * std::f32::consts::FRAC_PI_2
        },
        f32::atan,
    ) + if delta.z >= 0. {
        0.
    } else {
        delta.x.signum() * std::f32::consts::PI
    }
}

fn elevation(delta: Vec3) -> f32 {
    checked_div(delta.y, f32::sqrt(delta.x.powi(2) + delta.z.powi(2)))
        .map_or(delta.y.signum() * std::f32::consts::FRAC_PI_2, f32::atan)
}

impl Position {
    pub fn new(x: i16, y: i16, z: i16, yaw: i16, pitch: i8, roll: i16) -> Self {
        Self {
            position: Vec3::new(x as f32, y as f32, z as f32),
            rotation: Quat::from_euler(
                EulerRot::ZXY, // Rotation performed from right to left order
                (roll as f32).to_radians(),
                (pitch as f32).to_radians(),
                (yaw as f32).to_radians(),
            ),
        }
    }

    pub fn compute_range_azimuth_elevation(&self, other: &Position) -> (u16, i16, i8) {
        let delta = other.position - self.position;

        let distance = delta.length();
        let direction = self.rotation.mul_vec3(delta);

        let azimuth = azimuth(direction).to_degrees().round();
        let elevation = elevation(direction).to_degrees().round();

        assert!((-180. ..=180.).contains(&azimuth));
        assert!((-90. ..=90.).contains(&elevation));

        (
            f32::min(distance, u16::MAX as f32) as u16,
            azimuth as i16,
            elevation as i8,
        )
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, 0, 0)
    }
}

impl From<&PicaPosition> for Position {
    fn from(other: &PicaPosition) -> Self {
        Self::new(
            other.x as i16,
            other.y as i16,
            other.z as i16,
            other.yaw as i16,
            other.pitch as i8,
            other.roll as i16,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn range() {
        let position_a = Position::new(0, 0, 0, 0, 0, 0);
        {
            let position_b = Position::new(10, 0, 0, 0, 0, 0);
            let (range, _, _) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(range == 10);
        }
        {
            let position_b = Position::new(-10, 0, 0, 0, 0, 0);
            let (range, _, _) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(range == 10);
        }
        {
            let position_b = Position::new(10, 10, 0, 0, 0, 0);
            let (range, _, _) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(range == f32::sqrt(10. * 10. + 10. * 10.).round() as u16);
        }
        {
            let position_b = Position::new(-10, -10, -10, 0, 0, 0);
            let (range, _, _) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(range == f32::sqrt(10. * 10. + 10. * 10. + 10. * 10.).round() as u16);
        }
    }

    #[test]
    fn azimuth_without_rotation() {
        let position_a = Position::new(0, 0, 0, 0, 0, 0);
        {
            let position_b = Position::new(10, 0, 10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 45);
            assert!(elevation == 0);
        }
        {
            let position_b = Position::new(-10, 0, 10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == -45);
            assert!(elevation == 0);
        }
        {
            let position_b = Position::new(10, 0, -10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 135);
            assert!(elevation == 0);
        }
        {
            let position_b = Position::new(-10, 0, -10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == -135);
            assert!(elevation == 0);
        }
    }

    #[test]
    fn elevation_without_rotation() {
        let position_a = Position::new(0, 0, 0, 0, 0, 0);
        {
            let position_b = Position::new(0, 10, 10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 0);
            assert!(elevation == 45);
        }
        {
            let position_b = Position::new(0, -10, 10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 0);
            assert!(elevation == -45);
        }
        {
            let position_b = Position::new(0, 10, -10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 180 || azimuth == -180);
            assert!(elevation == 45);
        }
        {
            let position_b = Position::new(0, -10, -10, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 180 || azimuth == -180);
            assert!(elevation == -45);
        }
    }

    #[test]
    fn rotation_only() {
        let position_b = Position::new(0, 0, 10, 0, 0, 0);
        {
            let position_a = Position::new(0, 0, 0, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 0);
            assert!(elevation == 0);
        }
        {
            let position_a = Position::new(0, 0, 0, 45, 0, 0); // <=> azimuth = -45deg
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 45);
            assert!(elevation == 0);
        }
        {
            let position_a = Position::new(0, 0, 0, 0, 45, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 0);
            assert!(elevation == -45);
        }
        {
            let position_a = Position::new(0, 0, 0, 0, 0, 45);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 0);
            assert!(elevation == 0);
        }
    }

    #[test]
    fn rotation_only_complex_position() {
        let position_b = Position::new(10, 10, 10, 0, 0, 0);
        {
            let position_a = Position::new(0, 0, 0, 0, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 45);
            assert!(elevation == 35);
        }
        {
            let position_a = Position::new(0, 0, 0, 90, 0, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 90 + 45);
            assert!(elevation == 35);
        }
        {
            let position_a = Position::new(0, 0, 0, 0, 90, 0);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 45);
            assert!(elevation == -35);
        }
        {
            let position_a = Position::new(0, 0, 0, 0, 0, 90);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == -45);
            assert!(elevation == 35);
        }
        {
            let position_a = Position::new(0, 0, 0, -45, 35, 42);
            let (_, azimuth, elevation) = position_a.compute_range_azimuth_elevation(&position_b);
            assert!(azimuth == 0);
            assert!(elevation == 0);
        }
    }
}
