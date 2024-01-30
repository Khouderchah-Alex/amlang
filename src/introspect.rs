use serde::de::{self, Deserialize, Deserializer, Visitor};


pub struct Introspection {
    name: &'static str,
    kind: StructureKind,
}

pub enum StructureKind {
    Struct(&'static [&'static str]),
    Enum(&'static [&'static str]),
    Newtype,
    TupleStruct(usize),
    Unknown,
}

impl Introspection {
    pub fn of<'de, T>() -> Self
    where
        T: Deserialize<'de>,
    {
        let mut introspection = None;
        // Deserializer returns an error to short-circuit traversal (shallow, not deep).
        let _ = T::deserialize(IntrospectionDeserializer {
            introspection: &mut introspection,
        });
        introspection.unwrap_or_default()
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn fields(&self) -> &[&'static str] {
        match self.kind {
            StructureKind::Struct(fields) => fields,
            StructureKind::Enum(variants) => variants,
            _ => &[],
        }
    }

    pub fn kind(&self) -> &StructureKind {
        &self.kind
    }
}

impl Default for Introspection {
    fn default() -> Self {
        Self {
            name: "UNKNOWN",
            kind: StructureKind::Unknown,
        }
    }
}


struct IntrospectionDeserializer<'a> {
    introspection: &'a mut Option<Introspection>,
}

impl<'de, 'a> Deserializer<'de> for IntrospectionDeserializer<'a> {
    type Error = de::value::Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::custom(""))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        *self.introspection = Some(Introspection {
            name,
            kind: StructureKind::Struct(fields),
        });
        Err(de::Error::custom(""))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        *self.introspection = Some(Introspection {
            name,
            kind: StructureKind::Enum(variants),
        });
        Err(de::Error::custom(""))
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        *self.introspection = Some(Introspection {
            name,
            kind: StructureKind::Newtype,
        });
        Err(de::Error::custom(""))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        *self.introspection = Some(Introspection {
            name,
            kind: StructureKind::TupleStruct(len),
        });
        Err(de::Error::custom(""))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct seq tuple map identifier ignored_any
    }
}


#[cfg(test)]
#[path = "./introspect_test.rs"]
mod introspect_test;
