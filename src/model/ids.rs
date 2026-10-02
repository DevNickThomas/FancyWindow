use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Defines a GUID-backed id that serializes as 32 hex digits (no dashes),
/// matching the format written by the original .NET app.
macro_rules! guid_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0.simple())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = String::deserialize(d)?;
                Uuid::parse_str(&text).map(Self).map_err(serde::de::Error::custom)
            }
        }
    };
}

guid_id!(ZoneId);
guid_id!(SplitId);
