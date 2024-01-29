use std::collections::VecDeque;
use std::convert::TryFrom;

use log::debug;
use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;

use super::deserialize_error::DeserializeError;
use crate::agent::lang_error::LangError;
use crate::agent::Agent;
use crate::error::Error;
use crate::parser::Parser;
use crate::primitive::symbol_policies::{policy_env_serde, AdminSymbolInfo};
use crate::primitive::*;
use crate::sexp::{Cons, Sexp};
use crate::stream::input::StringReader;
use crate::token::Tokenizer;

use DeserializerState::*;

pub struct BaseDeserializer<'de> {
    agent: &'de mut Agent,
    stack: VecDeque<(DeserializerState, Sexp)>,
}

#[derive(Debug)]
enum DeserializerState {
    Base, // Only state possible when depth is 0.
    Map,
}

impl<'de> BaseDeserializer<'de> {
    pub fn from_sexp(agent: &'de mut Agent, sexp: Sexp) -> Self {
        Self {
            agent,
            stack: vec![(Base, sexp)].into(),
        }
    }

    fn input(&mut self) -> Sexp {
        self.stack.pop_front().unwrap().1
    }

    fn deserialize_node(&self, sexp: Sexp) -> Result<Node, Error> {
        match sexp {
            Sexp::Primitive(Primitive::Node(node)) => Ok(node),
            Sexp::Primitive(Primitive::Symbol(sym)) => {
                match policy_env_serde(sym.as_str()).unwrap() {
                    AdminSymbolInfo::Identifier => {
                        if let Ok(resolved) = self.agent.resolve(&sym) {
                            Ok(resolved.into())
                        } else {
                            err!(self.agent, LangError::UnboundSymbol(sym.clone()))
                        }
                    }
                    AdminSymbolInfo::LocalNode(node) => Ok(node.globalize(self.agent)),
                    AdminSymbolInfo::LocalTriple(idx) => {
                        let triple = self.agent.env().triple_from_index(idx);
                        Ok(triple.node().globalize(self.agent))
                    }
                    AdminSymbolInfo::GlobalNode(env, node) => Ok(Node::new(env, node)),
                    AdminSymbolInfo::GlobalTriple(env, idx) => Ok(Node::new(
                        env,
                        self.agent.env().triple_from_index(idx).node(),
                    )),
                }
            }
            sexp @ _ => err!(
                self.agent,
                DeserializeError::UnexpectedType {
                    given: sexp,
                    expected: "Node repr".into()
                },
            ),
        }
    }
}

pub fn from_str<'a, T>(agent: &'a mut Agent, s: &'a str) -> Result<T, Error>
where
    T: Deserialize<'a>,
{
    let mut sexps = pull_transform!(StringReader::new(s)
                                    =>> Tokenizer::new(policy_base)
                                    =>. Parser::new());
    if let Some(sexp) = sexps.next() {
        if let Some(extra) = sexps.next() {
            return err_nost!(DeserializeError::ExtraneousData(extra?));
        }
        let mut deserializer = BaseDeserializer::from_sexp(agent, sexp?);
        T::deserialize(&mut deserializer)
    } else {
        return err_nost!(DeserializeError::MissingData);
    }
}


impl<'de, 'a> de::Deserializer<'de> for &'a mut BaseDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match &self.stack[0].0 {
            Base => match &self.stack[0].1 {
                Sexp::Cons(_cons) => self.deserialize_seq(visitor),
                Sexp::Primitive(_p) => self.deserialize_str(visitor),
            },
            Map => self.deserialize_map(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let node = as_type!(Node, self.input());
        // TODO(perf, func) Might be better to directly access context somehow.
        let b = if node
            == self
                .agent
                .resolve(&"true".to_symbol_or_panic(policy_base))?
        {
            true
        } else {
            false
        };
        visitor.visit_bool(b)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(as_type!(i8, self.input()))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(as_type!(i16, self.input()))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(as_type!(i32, self.input()))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(as_type!(i64, self.input()))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(as_type!(u8, self.input()))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(as_type!(u16, self.input()))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(as_type!(u32, self.input()))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let input = self.input();
        let num = match Node::try_from(input) {
            Ok(node) => node.local().id(),
            Err(input) => match u64::try_from(input) {
                Ok(v) => v,
                Err(raw) => {
                    return err_nost!(DeserializeError::UnexpectedType {
                        given: raw,
                        expected: stringify!($type).into()
                    });
                }
            },
        };
        visitor.visit_u64(num)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(as_type!(f32, self.input()))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(as_type!(f64, self.input()))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        // Primitive & tokenization don't special case chars rn.
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(as_type!(LangString, self.input()).as_str().to_owned())
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("newtype_struct {}", name);
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("seq");
        let value = visitor.visit_seq(CompositeAccessor::new(self))?;
        Ok(value)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("tuple");
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("tuple_struct {}", name);
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("map");
        let value = visitor.visit_map(CompositeAccessor::new(self))?;
        Ok(value)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("struct {}", name);
        let (_state, top) = self.stack.pop_front().unwrap();
        if name == "Node" {
            let node = self.deserialize_node(top)?;
            self.stack.push_front((
                Map,
                list!(
                    Cons::new(
                        "env".to_symbol_or_panic(policy_base),
                        Some(node.env().id().into())
                    ),
                    Cons::new(
                        "local".to_symbol_or_panic(policy_base),
                        Some(node.local().id().into())
                    ),
                ),
            ));
        } else {
            let (_head, tail) = Cons::try_from(top).unwrap_or_default().consume();
            self.stack.push_front((Map, *tail.unwrap_or_default()));
        }
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        debug!("enum {}", name);
        visitor.visit_enum(CompositeAccessor::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(as_type!(Symbol, self.input()).to_string())
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.input();
        visitor.visit_unit()
    }
}

struct CompositeAccessor<'a, 'de: 'a> {
    de: &'a mut BaseDeserializer<'de>,
}

impl<'a, 'de> CompositeAccessor<'a, 'de> {
    fn new(de: &'a mut BaseDeserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> SeqAccess<'de> for CompositeAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let (state, top) = self.de.stack.pop_front().unwrap();
        if top.is_none() {
            return Ok(None);
        }
        let (head, tail) = Cons::try_from(top).unwrap_or_default().consume();
        self.de.stack.push_front((state, *tail.unwrap_or_default()));

        let head = head.unwrap_or_default();
        self.de.stack.push_front((Base, *head));

        seed.deserialize(&mut *self.de).map(Some)
    }
}

impl<'de, 'a> MapAccess<'de> for CompositeAccessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        let (state, top) = self.de.stack.pop_front().unwrap();
        if top.is_none() {
            return Ok(None);
        }
        let (head, tail) = Cons::try_from(top).unwrap_or_default().consume();
        self.de.stack.push_front((state, *tail.unwrap_or_default()));

        let head = head.unwrap_or_default();
        let (k, v) = Cons::try_from(*head).unwrap_or_default().consume();
        self.de.stack.push_front((Base, *v.unwrap_or_default()));
        self.de.stack.push_front((Base, *k.unwrap_or_default()));

        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}

impl<'de, 'a> EnumAccess<'de> for CompositeAccessor<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (state, top) = self.de.stack.pop_front().unwrap();
        match top {
            Sexp::Cons(cons) => {
                let (head, tail) = cons.consume();
                self.de
                    .stack
                    .push_front((Base, tail.unwrap_or_default().into()));
                self.de
                    .stack
                    .push_front((state, head.unwrap_or_default().into()));
            }
            Sexp::Primitive(p) => {
                self.de.stack.push_front((Base, p.into()));
            }
        };
        Ok((seed.deserialize(&mut *self.de)?, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for CompositeAccessor<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let (state, top) = self.de.stack.pop_front().unwrap();
        let (head, _tail) = Cons::try_from(top).unwrap_or_default().consume();
        self.de.stack.push_front((state, *head.unwrap_or_default()));

        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}


#[macro_export]
macro_rules! as_type {
    ($type:ident, $($raw:tt)+) => {
        match $type::try_from($($raw)+) {
            Ok(v) => v,
            Err(raw) => {
                return err_nost!(DeserializeError::UnexpectedType {
                    given: raw,
                    expected: stringify!($type).into()
                });
            }
        }
    };
}
use as_type;
