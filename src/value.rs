use std::path::Path;

/// CFAbsoluteTime: seconds since 2001-01-01 00:00:00 UTC
pub type AbsoluteTime = f64;

/// Epoch difference: Unix epoch (1970) to CF epoch (2001) in seconds
pub const CF_EPOCH_OFFSET: f64 = 978307200.0;

#[derive(Debug, Clone, Default)]
pub struct Dictionary {
    entries: Vec<(String, Value)>,
}

impl Dictionary {
    pub fn new() -> Self {
        Dictionary {
            entries: Vec::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.entries
            .iter_mut()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    pub fn insert(&mut self, key: String, value: Value) {
        if let Some(entry) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            Some(self.entries.remove(pos).1)
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Real(f64),
    Boolean(bool),
    Date(AbsoluteTime),
    Data(Vec<u8>),
    Array(Vec<Value>),
    Dictionary(Dictionary),
}

impl Value {
    pub fn from_file(path: &Path) -> anyhow::Result<Value> {
        crate::cf::read_plist(path)
    }

    pub fn to_file_xml(&self, path: &Path) -> anyhow::Result<()> {
        crate::cf::write_plist_xml(self, path)
    }

    pub fn to_file_binary(&self, path: &Path) -> anyhow::Result<()> {
        crate::cf::write_plist_binary(self, path)
    }

    pub fn to_xml_bytes(&self) -> anyhow::Result<Vec<u8>> {
        crate::cf::value_to_xml_bytes(self)
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_signed_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            Value::Real(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_data(&self) -> Option<&[u8]> {
        match self {
            Value::Data(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_dictionary(&self) -> Option<&Dictionary> {
        match self {
            Value::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_dictionary_mut(&mut self) -> Option<&mut Dictionary> {
        match self {
            Value::Dictionary(d) => Some(d),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
}
