//! A JSON value that keeps object key order (Ruby hash order), which
//! `serde_json::Value` loses without its `preserve_order` feature.
//! Duplicate keys keep the first position and the last value, like Ruby's
//! `JSON.parse`.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// Order-preserving JSON value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OrderedJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<OrderedJson>),
    Object(Vec<(String, OrderedJson)>),
}

impl OrderedJson {
    /// Object member by key.
    pub(crate) fn get(&self, key: &str) -> Option<&OrderedJson> {
        match self {
            OrderedJson::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            OrderedJson::String(s) => Some(s),
            _ => None,
        }
    }

    /// Convert from a `serde_json::Value` (object order as it iterates).
    pub(crate) fn from_value(v: &serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => OrderedJson::Null,
            serde_json::Value::Bool(b) => OrderedJson::Bool(*b),
            serde_json::Value::Number(n) => OrderedJson::Number(n.clone()),
            serde_json::Value::String(s) => OrderedJson::String(s.clone()),
            serde_json::Value::Array(items) => {
                OrderedJson::Array(items.iter().map(Self::from_value).collect())
            }
            serde_json::Value::Object(map) => OrderedJson::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), Self::from_value(v)))
                    .collect(),
            ),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OrderedVisitor)
    }
}

struct OrderedVisitor;

impl<'de> Visitor<'de> for OrderedVisitor {
    type Value = OrderedJson;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Null)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Number(v.into()))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Number(v.into()))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<OrderedJson, E> {
        serde_json::Number::from_f64(v)
            .map(OrderedJson::Number)
            .ok_or_else(|| E::custom("non-finite number"))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<OrderedJson, E> {
        Ok(OrderedJson::String(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<OrderedJson, E> {
        Ok(OrderedJson::String(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OrderedJson, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(OrderedJson::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<OrderedJson, A::Error> {
        let mut entries: Vec<(String, OrderedJson)> = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, OrderedJson>()? {
            match entries.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => entries.push((key, value)),
            }
        }
        Ok(OrderedJson::Object(entries))
    }
}
