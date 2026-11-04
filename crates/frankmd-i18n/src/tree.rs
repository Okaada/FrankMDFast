//! Order-preserving locale tree (YAML document order matters for the
//! coverage report, and serde_json's map is sorted).

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// A scalar (or sequence) leaf as `YAML.safe_load` would return it.
#[derive(Debug, Clone, PartialEq)]
pub enum Leaf {
    /// String.
    Str(String),
    /// Boolean (YAML 1.2 `true`/`false` only).
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// `~` / empty.
    Null,
    /// Sequence.
    List(Vec<Leaf>),
}

impl Leaf {
    /// Ruby class name used in coverage messages (`String`, `FalseClass`,
    /// `TrueClass`, `Integer`, `Float`, `NilClass`, `Array`).
    pub fn ruby_class(&self) -> &'static str {
        match self {
            Leaf::Str(_) => "String",
            Leaf::Bool(true) => "TrueClass",
            Leaf::Bool(false) => "FalseClass",
            Leaf::Int(_) => "Integer",
            Leaf::Float(_) => "Float",
            Leaf::Null => "NilClass",
            Leaf::List(_) => "Array",
        }
    }

    /// The string value, if this is a string leaf.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Leaf::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Rails `present?`: a non-blank string (other non-nil scalars count as
    /// present too, matching `Object#present?`).
    pub fn is_present(&self) -> bool {
        match self {
            Leaf::Str(s) => !s.trim().is_empty(),
            Leaf::Bool(b) => *b,
            Leaf::Null => false,
            Leaf::List(items) => !items.is_empty(),
            Leaf::Int(_) | Leaf::Float(_) => true,
        }
    }

    fn from_json(v: &serde_json::Value) -> Leaf {
        match v {
            serde_json::Value::Null => Leaf::Null,
            serde_json::Value::Bool(b) => Leaf::Bool(*b),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(i) => Leaf::Int(i),
                None => Leaf::Float(n.as_f64().unwrap_or_default()),
            },
            serde_json::Value::String(s) => Leaf::Str(s.clone()),
            serde_json::Value::Array(items) => {
                Leaf::List(items.iter().map(Leaf::from_json).collect())
            }
            // A map inside a list never occurs in locale files; keep it visible.
            serde_json::Value::Object(_) => Leaf::Str(v.to_string()),
        }
    }

    fn to_json(&self) -> serde_json::Value {
        match self {
            Leaf::Str(s) => serde_json::Value::String(s.clone()),
            Leaf::Bool(b) => serde_json::Value::Bool(*b),
            Leaf::Int(i) => serde_json::Value::from(*i),
            Leaf::Float(f) => serde_json::Number::from_f64(*f)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Leaf::Null => serde_json::Value::Null,
            Leaf::List(items) => {
                serde_json::Value::Array(items.iter().map(Leaf::to_json).collect())
            }
        }
    }
}

/// A map or a leaf.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Ordered map.
    Map(Vec<(String, Node)>),
    /// Leaf.
    Leaf(Leaf),
}

impl Node {
    /// Child by key (maps only).
    pub fn get(&self, key: &str) -> Option<&Node> {
        match self {
            Node::Map(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            Node::Leaf(_) => None,
        }
    }

    /// Node at a dotted path (`"dialogs.help.find"`).
    pub fn lookup(&self, dotted: &str) -> Option<&Node> {
        dotted.split('.').try_fold(self, |node, seg| node.get(seg))
    }

    /// Insert or replace `key`, keeping the position of an existing key
    /// (Ruby `Hash#[]=` semantics). No-op on a leaf.
    pub fn insert(&mut self, key: String, value: Node) {
        if let Node::Map(entries) = self {
            match entries.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => entries.push((key, value)),
            }
        }
    }

    /// Convert from JSON (object key order as `serde_json` yields it).
    pub fn from_json(v: &serde_json::Value) -> Node {
        match v {
            serde_json::Value::Object(map) => Node::Map(
                map.iter()
                    .map(|(k, v)| (k.clone(), Node::from_json(v)))
                    .collect(),
            ),
            other => Node::Leaf(Leaf::from_json(other)),
        }
    }

    /// Convert to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Node::Map(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_json()))
                    .collect(),
            ),
            Node::Leaf(leaf) => leaf.to_json(),
        }
    }
}

// --- YAML deserialization (order preserving) ------------------------------

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(NodeVisitor)
    }
}

struct NodeVisitor;

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a YAML value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Bool(v)))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Int(v)))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Node, E> {
        Ok(Node::Leaf(
            i64::try_from(v).map_or(Leaf::Float(v as f64), Leaf::Int),
        ))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Float(v)))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Str(v.to_owned())))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Str(v)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Null))
    }

    fn visit_none<E: de::Error>(self) -> Result<Node, E> {
        Ok(Node::Leaf(Leaf::Null))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Node, D::Error> {
        Node::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut items = Vec::new();
        while let Some(node) = seq.next_element::<Node>()? {
            match node {
                Node::Leaf(leaf) => items.push(leaf),
                Node::Map(_) => {
                    return Err(de::Error::custom(
                        "maps inside sequences are not supported in locale files",
                    ));
                }
            }
        }
        Ok(Node::Leaf(Leaf::List(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut node = Node::Map(Vec::new());
        while let Some((key, value)) = map.next_entry::<Key, Node>()? {
            node.insert(key.0, value);
        }
        Ok(node)
    }
}

/// A mapping key; non-string scalar keys are stringified (Rails symbolizes
/// keys, so `1:` and `"1":` address the same entry).
struct Key(String);

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Node::deserialize(deserializer)? {
            Node::Leaf(Leaf::Str(s)) => Ok(Key(s)),
            Node::Leaf(Leaf::Bool(b)) => Ok(Key(b.to_string())),
            Node::Leaf(Leaf::Int(i)) => Ok(Key(i.to_string())),
            Node::Leaf(Leaf::Float(f)) => Ok(Key(f.to_string())),
            Node::Leaf(Leaf::Null) => Ok(Key(String::new())),
            _ => Err(de::Error::custom("unsupported mapping key")),
        }
    }
}
