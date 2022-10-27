//! Representation of builtin methods.

use std::convert::TryFrom;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Serialize;

use crate::agent::Agent;
use crate::builtins::generate_builtin_map;
use crate::error::Error;
use crate::primitive::Primitive;
use crate::sexp::{HeapSexp, Sexp};


#[derive(Clone, Serialize)]
pub struct BuiltIn {
    name: String,

    #[serde(skip_serializing)]
    fun: fn(Sexp, &mut Agent) -> Result<Sexp, Error>,
}

impl BuiltIn {
    pub fn new(name: &'static str, fun: fn(Sexp, &mut Agent) -> Result<Sexp, Error>) -> BuiltIn {
        BuiltIn {
            name: name.to_string(),
            fun,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn call(&self, args: Sexp, agent: &mut Agent) -> Result<Sexp, Error> {
        (self.fun)(args, agent)
    }
}

impl PartialEq for BuiltIn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl fmt::Debug for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[BUILTIN_{} @ {:p}]", self.name, &self.fun)
    }
}

impl fmt::Display for BuiltIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[BUILTIN_{}]", self.name)
    }
}


impl_try_from!(BuiltIn;
               Primitive         ->  BuiltIn,
               Sexp              ->  BuiltIn,
               HeapSexp          ->  BuiltIn,
               ref Sexp          ->  ref BuiltIn,
               Option<Sexp>      ->  BuiltIn,
               Option<ref Sexp>  ->  ref BuiltIn,
               Result<Sexp>      ->  BuiltIn,
               Result<ref Sexp>  ->  ref BuiltIn,
);


// Impl for generic Deserializers; custom Deserializers may want to
// short-circuit this logic at deserialize_struct("BuiltIn",...).
impl<'de> Deserialize<'de> for BuiltIn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Name,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`name`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "name" => Ok(Field::Name),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct BuiltInVisitor;

        impl<'de> Visitor<'de> for BuiltInVisitor {
            type Value = BuiltIn;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct BuiltIn")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<BuiltIn, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let name: &str = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                // TODO(flex) Can this come from the Deserializer?
                Ok(generate_builtin_map().get(name).unwrap().clone())
            }

            fn visit_map<V>(self, mut map: V) -> Result<BuiltIn, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name: Option<&str> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                    }
                }
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                // TODO(flex) Can this come from the Deserializer?
                Ok(generate_builtin_map().get(name).unwrap().clone())
            }
        }

        const FIELDS: &'static [&'static str] = &["name"];
        deserializer.deserialize_struct("BuiltIn", FIELDS, BuiltInVisitor)
    }
}
