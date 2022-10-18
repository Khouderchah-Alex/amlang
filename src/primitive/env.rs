use std::convert::TryFrom;
use std::fmt;

use serde::de::{Deserialize, Deserializer, Visitor};
use serde::{Serialize, Serializer};

use crate::environment::environment::EnvObject;
use crate::environment::local_node::LocalNode;
use crate::primitive::Node;


impl Serialize for Box<EnvObject> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let self_node = self.entry(LocalNode::default());
        serializer
            .serialize_newtype_struct("Env", <&Node>::try_from(self_node.structure()).unwrap())
    }
}

// Impl for generic Deserializers; custom Deserializers may want to
// short-circuit this logic at deserialize_newtype_struct("Env", ...).
impl<'de> Deserialize<'de> for Box<EnvObject> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvVisitor;

        impl<'de> Visitor<'de> for EnvVisitor {
            type Value = Box<EnvObject>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Env")
            }
        }

        deserializer.deserialize_newtype_struct("Env", EnvVisitor)
    }
}


impl fmt::Debug for Box<EnvObject> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} @ {:p}]", self.type_name(), self)
    }
}
