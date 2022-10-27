//! Allows reifying ABI into API.
//!
//! Agent is in charge of determining how user-created abstractions in
//! Rust are repr'd.

use std::collections::VecDeque;

use log::debug;
use serde::{ser, Serialize};

use crate::agent::symbol_policies::policy_base;
use crate::agent::Agent;
use crate::error::Error;
use crate::primitive::prelude::*;
use crate::sexp::{Cons, ConsList, HeapSexp};


pub struct BaseSerializer<'a> {
    agent: &'a mut Agent,
    stack: VecDeque<ConsList>,
    tmp: Vec<HeapSexp>,
}

impl<'a> BaseSerializer<'a> {
    pub fn new(agent: &'a mut Agent) -> Self {
        Self {
            agent,
            stack: Default::default(),
            tmp: Default::default(),
        }
    }

    fn serialize_symbol<S: AsRef<str>>(s: S) -> Result<HeapSexp, Error> {
        // TODO(func) This should be controlled by the Agent.
        match s.to_symbol(policy_base) {
            Ok(sym) => Ok(sym.into()),
            Err(err) => panic!("{:?}", err),
        }
    }
}

impl<'a, 'b> ser::Serializer for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        let sexp = if v {
            amlang_node!(t, self.agent.context())
        } else {
            amlang_node!(f, self.agent.context())
        }
        .into();
        Ok(sexp)
    }

    // TODO(func) Have Number support more than i64, f64.
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Number::I8(v).into())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Number::I16(v).into())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Number::I32(v).into())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Number::I64(v).into())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Number::U8(v).into())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Number::U16(v).into())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Number::U32(v).into())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Number::U64(v).into())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Number::F32(v).into())
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Number::F64(v).into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(LangString::new(v).into())
    }

    // Serialize a byte array as an array of bytes. Could also use a base64
    // string here. Binary formats will typically represent byte arrays more
    // compactly.
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Cons::new(None, None).into())
    }
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Cons::new(None, None).into())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        let val = name.serialize(self)?;
        Ok(Cons::new(val, None).into())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        BaseSerializer::<'b>::serialize_symbol(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        // TODO(func) Treat like a struct.
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        debug!("Serializing newtype_variant {}::{}", name, variant);
        let name = BaseSerializer::<'b>::serialize_symbol(name)?;
        let v = value.serialize(&mut *self)?;
        Ok(list!(name, v,).into())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        debug!("Serializing seq");
        self.stack.push_front(ConsList::new());
        Ok(self)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        debug!("Serializing tuple");
        self.stack.push_front(ConsList::new());
        Ok(self)
    }
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        debug!("Serializing tuple_struct {}", name);
        self.stack.push_front(ConsList::new());
        let name = BaseSerializer::<'b>::serialize_symbol(name)?;
        self.stack[0].append(name);
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        debug!("Serializing tuple_variant {}::{}", name, variant);
        self.stack.push_front(ConsList::new());
        let name = BaseSerializer::<'b>::serialize_symbol(name)?;
        let variant = BaseSerializer::<'b>::serialize_symbol(variant)?;
        let val: HeapSexp = Cons::new(name, variant).into();
        self.stack[0].append(val);
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        debug!("Serializing map");
        self.stack.push_front(ConsList::new());
        Ok(self)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        debug!("Serializing struct {}", name);
        self.stack.push_front(ConsList::new());
        let name = BaseSerializer::<'b>::serialize_symbol(name)?;
        self.stack[0].append(name);
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        debug!("Serializing struct_variant {}::{}", name, variant);
        self.stack.push_front(ConsList::new());
        let name = BaseSerializer::<'b>::serialize_symbol(name)?;
        let variant = BaseSerializer::<'b>::serialize_symbol(variant)?;
        let val: HeapSexp = Cons::new(name, variant).into();
        self.stack[0].append(val);
        Ok(self)
    }
}

impl<'a, 'b> ser::SerializeSeq for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let val = value.serialize(&mut **self)?;
        Ok(self.stack[0].append(val))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}

impl<'a, 'b> ser::SerializeTuple for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let val = value.serialize(&mut **self)?;
        Ok(self.stack[0].append(val))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}

impl<'a, 'b> ser::SerializeTupleStruct for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let val = value.serialize(&mut **self)?;
        Ok(self.stack[0].append(val))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}

impl<'a, 'b> ser::SerializeTupleVariant for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let val = value.serialize(&mut **self)?;
        Ok(self.stack[0].append(val))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}

impl<'a, 'b> ser::SerializeMap for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let val = key.serialize(&mut **self)?;
        Ok(self.tmp.push(val))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self.tmp.pop();
        let val = value.serialize(&mut **self)?;
        Ok(self.stack[0].append(Cons::new(key, val)))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}

impl<'a, 'b> ser::SerializeStruct for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let k = BaseSerializer::<'b>::serialize_symbol(key)?;
        let v = value.serialize(&mut **self)?;
        let val: HeapSexp = Cons::new(k, v).into();
        Ok(self.stack[0].append(val))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}

impl<'a, 'b> ser::SerializeStructVariant for &'a mut BaseSerializer<'b> {
    type Ok = HeapSexp;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let k = BaseSerializer::<'b>::serialize_symbol(key)?;
        let v = value.serialize(&mut **self)?;
        let val: HeapSexp = Cons::new(k, v).into();
        Ok(self.stack[0].append(val))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.stack.pop_front().unwrap().release().into())
    }
}
