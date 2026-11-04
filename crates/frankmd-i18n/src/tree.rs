//! Order-preserving locale tree (YAML document order matters for the
//! coverage report, and serde_json's map is sorted).

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

    /// Convert from JSON (object key order as `serde_json` yields it).
    pub fn from_json(v: &serde_json::Value) -> Node {
        todo!()
    }

    /// Convert to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        todo!()
    }
}
