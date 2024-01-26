use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::env::EnvObject;
use crate::primitive::prelude::*;
use crate::version::{Version, VersionString};


pub struct EnvHeader {
    file_version: VersionString,
    node_count: usize,
    triple_count: usize,
    unrecognized: SymSexpTable,
}

impl EnvHeader {
    pub fn from_env(env: &Box<EnvObject>) -> Self {
        let node_count = env.all_nodes().len();
        let triple_count = env.match_all().len();
        Self {
            file_version: Version::new(0, 0, 3).into(),
            node_count,
            triple_count,
            unrecognized: SymSexpTable::default(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn triple_count(&self) -> usize {
        self.triple_count
    }
}

impl Serialize for EnvHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("header", 3)?;
        state.serialize_field("version", &self.file_version)?;
        state.serialize_field("node-count", &self.node_count)?;
        state.serialize_field("triple-count", &self.triple_count)?;
        for (k, v) in self.unrecognized.as_map() {
            state.serialize_field(
                // UB, but SerializeStruct isn't working with us here.
                unsafe { std::mem::transmute::<&'_ str, &'static str>(k.as_str()) },
                v,
            )?;
        }
        state.end()
    }
}


impl<'de> Deserialize<'de> for EnvHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Version,
            NodeCount,
            TripleCount,
            Unrecognized(String),
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`version`, `node-count` or `triple-count`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "version" => Ok(Field::Version),
                            "node-count" => Ok(Field::NodeCount),
                            "triple-count" => Ok(Field::TripleCount),
                            other @ _ => Ok(Field::Unrecognized(other.to_string())),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct EnvHeaderVisitor;

        impl<'de> Visitor<'de> for EnvHeaderVisitor {
            type Value = EnvHeader;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct EnvHeader")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<EnvHeader, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let version = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let node_count = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let triple_count = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(EnvHeader {
                    file_version: version,
                    node_count,
                    triple_count,
                    unrecognized: Default::default(),
                })
            }

            fn visit_map<V>(self, mut map: V) -> Result<EnvHeader, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut version = None;
                let mut node_count = None;
                let mut triple_count = None;
                let mut unrecognized = SymSexpTable::default();
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Version => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            version = Some(map.next_value()?);
                        }
                        Field::NodeCount => {
                            if node_count.is_some() {
                                return Err(de::Error::duplicate_field("node-count"));
                            }
                            node_count = Some(map.next_value()?);
                        }
                        Field::TripleCount => {
                            if triple_count.is_some() {
                                return Err(de::Error::duplicate_field("triple-count"));
                            }
                            triple_count = Some(map.next_value()?);
                        }
                        Field::Unrecognized(other) => {
                            unrecognized.insert(
                                other
                                    .to_symbol(policy_base)
                                    .unwrap_or_else(|_| "invalid".to_symbol_or_panic(policy_base)),
                                map.next_value()?,
                            );
                        }
                    }
                }
                let version = version.ok_or_else(|| de::Error::missing_field("version"))?;
                let node_count =
                    node_count.ok_or_else(|| de::Error::missing_field("node-count"))?;
                let triple_count =
                    triple_count.ok_or_else(|| de::Error::missing_field("triple-count"))?;
                Ok(EnvHeader {
                    file_version: version,
                    node_count,
                    triple_count,
                    unrecognized,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["version", "node-count", "triple-count"];
        deserializer.deserialize_struct("header", FIELDS, EnvHeaderVisitor)
    }
}
