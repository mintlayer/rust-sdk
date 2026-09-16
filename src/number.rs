// Copyright (c) 2026 Mintlayer Institutional FZCO
// Contact: hello@mintlayer.org
//
// Use of this source code is governed by an MIT license
// that can be found in the LICENSE file.

//! Lenient wire-format helpers for amounts and integers that daemons may
//! encode either as JSON numbers or as decimal strings.

/// `(de)serializes` a `u128` atom amount as a decimal string, accepting a
/// decimal string or a plain integer when deserializing.
pub(crate) mod atoms {
    use std::fmt;

    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};

    #[allow(dead_code)]
    pub(crate) fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<u128, D::Error> {
        struct AtomsVisitor;

        impl Visitor<'_> for AtomsVisitor {
            type Value = u128;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a decimal atom amount (string or integer)")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<u128, E> {
                value.parse::<u128>().map_err(de::Error::custom)
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<u128, E> {
                Ok(u128::from(value))
            }
        }

        deserializer.deserialize_any(AtomsVisitor)
    }
}

/// `(de)serializes` a `u128` atom amount wrapped in the daemon's object
/// envelope `{"atoms": "<decimal string>"}`.
pub(crate) mod atoms_object {
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Amount", 1)?;
        state.serialize_field("atoms", &value.to_string())?;
        state.end()
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<u128, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(deserialize_with = "super::atoms::deserialize")]
            atoms: u128,
        }
        Wire::deserialize(deserializer).map(|wire| wire.atoms)
    }
}

/// Deserializes an `Option<u64>` that may arrive as `null`, an integer, or a
/// decimal string; serializes integers and `null`.
pub(crate) mod option_u64_lenient {
    use std::fmt;

    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(
        value: &Option<u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => serializer.serialize_u64(*value),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u64>, D::Error> {
        struct U64Visitor;

        impl Visitor<'_> for U64Visitor {
            type Value = Option<u64>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("null, an integer, or a decimal string")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Option<u64>, E> {
                Ok(None)
            }

            fn visit_none<E: de::Error>(self) -> Result<Option<u64>, E> {
                Ok(None)
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Option<u64>, E> {
                Ok(Some(value))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Option<u64>, E> {
                value.parse::<u64>().map(Some).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_any(U64Visitor)
    }
}
