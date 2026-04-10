//! Custom serde helpers for flexible deserialization of Bitkub API responses.
//!
//! Bitkub sends `Decimal` values as JSON strings in REST responses but as JSON
//! numbers in WebSocket messages. These helpers accept either representation.

use rust_decimal::Decimal;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serializer,
};
use std::fmt;

/// Deserialize a `Decimal` from a JSON number **or** a JSON string.
///
/// Use with `#[serde(deserialize_with = "decimal_from_any")]` on struct fields
/// that may arrive as either format.
pub fn decimal_from_any<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    struct DecimalVisitor;

    impl<'de> Visitor<'de> for DecimalVisitor {
        type Value = Decimal;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a decimal number or numeric string")
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            Decimal::try_from(v)
                .map_err(de::Error::custom)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Decimal::from(v))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Decimal::from(v))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            v.parse::<Decimal>().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(DecimalVisitor)
}

/// Serialize a `Decimal` as a JSON string.
///
/// Paired with [`decimal_from_any`] for round-trip support.
pub fn decimal_to_string<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

/// Deserialize an `Option<Decimal>` from a JSON number, string, or null.
pub fn option_decimal_from_any<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<DecimalAny>::deserialize(deserializer).map(|opt| opt.map(|d| d.0))
}

/// Serialize an `Option<Decimal>` as a JSON string or null.
pub fn option_decimal_to_string<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(d) => serializer.serialize_str(&d.to_string()),
        None => serializer.serialize_none(),
    }
}

/// Internal newtype for deserializing `Decimal` from any JSON scalar.
#[derive(Debug, Clone)]
pub(crate) struct DecimalAny(pub Decimal);

impl<'de> Deserialize<'de> for DecimalAny {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        decimal_from_any(deserializer).map(DecimalAny)
    }
}

/// Internal newtype for deserializing `bool` from a JSON boolean or integer (0/1).
#[derive(Debug, Clone)]
pub(crate) struct BoolAny(pub bool);

impl<'de> Deserialize<'de> for BoolAny {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = BoolAny;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a boolean or 0/1 integer")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(BoolAny(v))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(BoolAny(v != 0))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(BoolAny(v != 0))
            }
        }

        deserializer.deserialize_any(V)
    }
}

/// Module for use with `#[serde(with = "flexible_decimal")]` on struct fields.
///
/// Deserializes from number or string, serializes as string.
pub mod flexible_decimal {
    use super::*;

    pub fn serialize<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        decimal_to_string(value, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
    where
        D: Deserializer<'de>,
    {
        decimal_from_any(deserializer)
    }
}

/// Module for use with `#[serde(with = "flexible_option_decimal")]` on
/// `Option<Decimal>` struct fields.
pub mod flexible_option_decimal {
    use super::*;

    pub fn serialize<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        option_decimal_to_string(value, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
    where
        D: Deserializer<'de>,
    {
        option_decimal_from_any(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct TestStruct {
        #[serde(deserialize_with = "decimal_from_any")]
        value: Decimal,
    }

    #[test]
    fn decimal_from_string() {
        let json = r#"{"value": "123.456"}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(s.value, Decimal::new(123456, 3));
    }

    #[test]
    fn decimal_from_number() {
        let json = r#"{"value": 123.456}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        // f64 conversion may lose some precision but the value should be close
        assert!(s.value > Decimal::new(123455, 3));
        assert!(s.value < Decimal::new(123457, 3));
    }

    #[test]
    fn decimal_from_integer() {
        let json = r#"{"value": 100}"#;
        let s: TestStruct = serde_json::from_str(json).unwrap();
        assert_eq!(s.value, Decimal::from(100));
    }
}
