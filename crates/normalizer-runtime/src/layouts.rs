use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub offset: usize,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArrayDef {
    pub start_offset: usize,
    pub element_size: usize,
    pub count: usize,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum LayoutJson {
    // Simple struct layout
    Fields {
        name: String,
        version: String,
        fields: Vec<FieldDef>,
    },
    // Array-of-structs layout (tick arrays / bins)
    Array {
        name: String,
        version: String,
        array: ArrayDef,
    },
}

#[derive(Debug, Clone)]
pub struct LayoutRegistry {
    // We just keep the raw JSONs keyed by "<dex>/<kind>"
    pub map: std::collections::HashMap<String, LayoutJson>,
}

impl LayoutRegistry {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, lj: LayoutJson) {
        self.map.insert(key.into(), lj);
    }

    pub fn get(&self, key: &str) -> Option<&LayoutJson> {
        self.map.get(key)
    }

    pub fn load_file(&mut self, key: &str, path: &Path) -> anyhow::Result<()> {
        let text = fs::read_to_string(path)?;
        let lj: LayoutJson = serde_json::from_str(&text)?;
        self.insert(key.to_string(), lj);
        Ok(())
    }
}
