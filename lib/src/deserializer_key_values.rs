// Copyright 2018 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::serializer_error::{Error, Result};
use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer,
    MapAccess, SeqAccess, VariantAccess, Visitor,
};
use std::ops::{AddAssign, MulAssign};

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de str,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'de str) -> Self {
        Deserializer { input }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
    where
        T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    // Look at the first character in the input without consuming it.
    fn peek_char(&mut self) -> Result<char> {
        self.input.chars().next().ok_or(Error::Eof)
    }

    // Consume the first character in the input.
    fn next_char(&mut self) -> Result<char> {
        let ch = self.peek_char()?;
        self.input = &self.input[ch.len_utf8()..];
        Ok(ch)
    }

    // Parse the JSON identifier `true` or `false`.
    fn parse_bool(&mut self) -> Result<bool> {
        if self.input.starts_with("true") {
            self.input = &self.input["true".len()..];
            Ok(true)
        } else if self.input.starts_with("false") {
            self.input = &self.input["false".len()..];
            Ok(false)
        } else {
            Err(Error::ExpectedBoolean)
        }
    }

    // Parse a group of decimal digits as an unsigned integer of type T.
    //
    // This implementation is a bit too lenient, for example `001` is not
    // allowed in JSON. Also the various arithmetic operations can overflow and
    // panic or return bogus data. But it is good enough for example code!
    fn parse_unsigned<T>(&mut self) -> Result<T>
        where
            T: AddAssign<T> + MulAssign<T> + From<u8>,
    {
            if self.next_char()? != '"' {
                return Err(Error::ExpectedString);
            }
            match self.input.find('"') {
                Some(len) => {
                    let s = &self.input[..len];
                    self.input = &self.input[len + 1..];
                    // let mut int = T::from(s[0] as u8 - b'0');
                    let mut int = T::from(0);
                    for ch in s[0..].chars() {
                        int *= T::from(10);
                        int += T::from(ch as u8 - b'0');
                    }
                    Ok(int)
                }
                None => Err(Error::Eof),
            }
    }

    // Parse a possible minus sign followed by a group of decimal digits as a
    // signed integer of type T.


    fn parse_signed<T>(&mut self) -> Result<T>
        where
            T: AddAssign<T> + MulAssign<T> + From<i8>,
    {
        if self.next_char()? != '"' {
            return Err(Error::ExpectedString);
        }
        match self.input.find('"') {
            Some(len) => {
                let s_src = &self.input[..len];
                let s = if s_src.starts_with("-") {
                    &s_src[1..]
                } else {
                    s_src
                };
                let sign = if s_src.starts_with("-") {
                    -1
                } else {
                    1
                };
                self.input = &self.input[len + 1..];
                let mut int = T::from(0);
                for ch in s[0..].chars() {
                    int *= T::from(10);
                    let rrr = ch as u8 - b'0';
                    int += T::from(rrr as i8);
                }

                int *= T::from(sign);
                Ok(int)
            }
            None => Err(Error::Eof),
        }
    }


    // Parse a string until the next '"' character.
    //
    // Makes no attempt to handle escape sequences. What did you expect? This is
    // example code!

    fn parse_string(&mut self) -> Result<String> {
        if self.next_char()? != '"' {
            return Err(Error::ExpectedString);
        }

        let start_idx = 0;
        let mut end_idx = 0;
        let mut is_escaped = false;

        for (idx, char) in self.input.char_indices() {
            if is_escaped {
                is_escaped = false;
            } else if char == '\\' {
                is_escaped = true;
            } else if char == '"' {
                end_idx = idx;
                break;
            }
        }

        // if end_idx == 0 {
        //     return Err(Error::Eof);
        // }

        let s = &self.input[start_idx..end_idx];
        self.input = &self.input[end_idx + 1..];

        let r = s.to_string();
        let fixed_r = r.replace("\\\"", "\"");
        // let fixed_r = fixed_r.replace("\\r", "\r");
        // let fixed_r = fixed_r.replace("\\n", "\n");
        // let fixed_r = fixed_r.replace("\\t", "\t");
       let fixed_r = fixed_r.replace("\\\\", "\\");
        // println!("r: {}", r);
        // println!("fixed_r: {}", fixed_r);
        Ok(fixed_r)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        match self.peek_char()? {
            'n' => self.deserialize_unit(visitor),
            't' | 'f' => self.deserialize_bool(visitor),
            '"' => self.deserialize_str(visitor),
            '0'..='9' => self.deserialize_u64(visitor),
            '-' => self.deserialize_i64(visitor),
            '[' => self.deserialize_seq(visitor),
            '{' => self.deserialize_map(visitor),
            _ => Err(Error::Syntax),
        }
    }

    // Uses the `parse_bool` parsing function defined above to read the JSON
    // identifier `true` or `false` from the input.
    //
    // Parsing refers to looking at the input and deciding that it contains the
    // JSON value `true` or `false`.
    //
    // Deserialization refers to mapping that JSON value into Serde's data
    // model by invoking one of the `Visitor` methods. In the case of JSON and
    // bool that mapping is straightforward so the distinction may seem silly,
    // but in other cases Deserializers sometimes perform non-obvious mappings.
    // For example the TOML format has a Datetime type and Serde's data model
    // does not. In the `toml` crate, a Datetime in the input is deserialized by
    // mapping it to a Serde data model "struct" type with a special name and a
    // single field containing the Datetime represented as a string.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    // The `parse_signed` function is generic over the integer type `T` so here
    // it is invoked with `T=i8`. The next 8 methods are similar.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    // Float parsing is stupidly hard.
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        unimplemented!()
    }

    // Float parsing is stupidly hard.
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        unimplemented!()
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        // Parse a string, check that it is one character, call `visit_char`.
        unimplemented!()
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        unimplemented!()
    }

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_unit()
        } else {
            Err(Error::ExpectedNull)
        }
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        // Parse the opening bracket of the sequence.
        if self.next_char()? == '[' {
            // Give the visitor access to each element of the sequence.
            let value = visitor.visit_seq(CommaSeparated::new(self))?;
            // Parse the closing bracket of the sequence.
            if self.next_char()? == ']' {
                Ok(value)
            } else {
                Err(Error::ExpectedArrayEnd)
            }
        } else {
            Err(Error::ExpectedArray)
        }
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        // Parse the opening brace of the map.
        if self.next_char()? == '{' {
            // Give the visitor access to each entry of the map.
            let value = visitor.visit_map(CommaSeparated::new(self))?;
            // Parse the closing brace of the map.
            if self.next_char()? == '}' {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedMap)
        }
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        if self.peek_char()? == '"' {
            // Visit a unit variant.
            visitor.visit_enum(self.parse_string()?.into_deserializer())
        } else if self.next_char()? == '{' {
            // Visit a newtype variant, tuple variant, or struct variant.
            let value = visitor.visit_enum(Enum::new(self))?;
            // Parse the matching close brace.
            if self.next_char()? == '}' {
                Ok(value)
            } else {
                Err(Error::ExpectedMapEnd)
            }
        } else {
            Err(Error::ExpectedEnum)
        }
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

// In order to handle commas correctly when deserializing a JSON array or map,
// we need to track whether we are on the first element or past the first
// element.
struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        CommaSeparated { de, first: true }
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where
            T: DeserializeSeed<'de>,
    {
        // Check if there are no more elements.
        if self.de.peek_char()? == ']' {
            return Ok(None);
        }
        // Comma is required before every element except the first.
        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedArrayComma);
        }
        self.first = false;
        // Deserialize an array element.
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
        where
            K: DeserializeSeed<'de>,
    {
        // Check if there are no more entries.
        if self.de.peek_char()? == '}' {
            return Ok(None);
        }
        // Comma is required before every entry except the first.
        // println!("{}", self.de.next_char()?);
        if !self.first && self.de.next_char()? != ',' {
            return Err(Error::ExpectedMapComma);
        }
        self.first = false;
        // Deserialize a map key.
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
        where
            V: DeserializeSeed<'de>,
    {
        // It doesn't make a difference whether the colon is parsed at the end
        // of `next_key_seed` or at the beginning of `next_value_seed`. In this
        // case the code is a bit simpler having it here.
        if self.de.next_char()? != ':' {
            return Err(Error::ExpectedMapColon);
        }
        // Deserialize a map value.
        seed.deserialize(&mut *self.de)
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
        where
            V: DeserializeSeed<'de>,
    {
        // The `deserialize_enum` method parsed a `{` character so we are
        // currently inside of a map. The seed will be deserializing itself from
        // the key of the map.
        let val = seed.deserialize(&mut *self.de)?;
        // Parse the colon separating map key from value.
        if self.de.next_char()? == ':' {
            Ok((val, self))
        } else {
            Err(Error::ExpectedMapColon)
        }
    }
}

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::ExpectedString)
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
        where
            T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
        where
            V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::from_str;
    use serde_derive::Deserialize;

    #[test]
    fn test_struct() {

        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            id: i32,
            id_positive: i32,
            name: String,
            ud: u64,

        }

        #[derive( Deserialize, Debug, Clone)]
        pub struct FileDescription {
            pub id: i32,
            pub path: String,
            pub internal: Option<String>,
            pub disk: String,
            pub size: i32,
            pub modified: i32,
            pub content: Option<String>,
        }
        let j = r#"{"id":"25","path":"C:\\ODS\\~reserved.txt","internal":null,"disk":"C","size":"0","modified":"0","content":"  "}"#;
        let r: FileDescription = from_str(j).unwrap();
        let j = r#"{"id":"-222","id_positive":"1","name":"a\"
\\","ud":"777"}"#;
        let expected = Test {
            id: -222,
            id_positive: 1,
            name:  "a\"\n\\".to_string(),
            ud: 777,
        };
        println!("{:?}", expected);
                    let r: Test = from_str(j).unwrap();
        println!("{:?}", r);

        assert_eq!(expected, from_str(j).unwrap());
    }


    #[test]
    fn test_escape() {

        #[derive(Deserialize, PartialEq, Debug)]
        struct Test {
            id: i32,
            id_positive: i32,
            name: String,
            ud: u64,

        }

        let j = r#"{"id":"-222","id_positive":"1","name":"c:\temp:","ud":"777"}"#;
        let expected = Test {
            id: -222,
            id_positive: 1,
            name:  "c:\\temp:".to_string(),
            ud: 777,
        };
        println!("{:?}", expected);
        let r: Test = from_str(j).unwrap();
        println!("{:?}", r);

        assert_eq!(expected, from_str(j).unwrap());
    }

    // #[test]
    fn test_more() {
        let str = "{\"id\":\"15\",\"path\":\"C:\\$SysReset\\Logs\\diagwrn.xml\",\"internal\":null,\"mime_type\":\"application/xml\",\"disk\":\"C\",\"size\":\"47278\",\"modified\":\"1679648060\",\"content\":\"<xml xmlns:s=\\\"uuid:BDC6E3F0-6DA3-11d1-A2A3-00AA00C14882\\\"
     xmlns:dt=\\\"uuid:C2F41010-65B3-11d1-A29F-00AA00C14882\\\"
     xmlns:rs=\\\"urn:schemas-microsoft-com:rowset\\\"
     xmlns:z=\\\"#RowsetSchema\\\">
    <s:Schema id=\\\"RowsetSchema\\\">
    <s:ElementType name=\\\"row\\\" content=\\\"eltOnly\\\" rs:updatable=\\\"true\\\">
    <s:AttributeType name=\\\"Cls\\\" rs:number=\\\"0\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Sev\\\" rs:number=\\\"1\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Maj\\\" rs:number=\\\"2\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Min\\\" rs:number=\\\"3\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"LN\\\" rs:number=\\\"4\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fil\\\" rs:number=\\\"5\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fun\\\" rs:number=\\\"6\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Uid\\\" rs:number=\\\"7\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Msg\\\" rs:number=\\\"8\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"PID\\\" rs:number=\\\"9\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"TID\\\" rs:number=\\\"10\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Con\\\" rs:number=\\\"11\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Exe\\\" rs:number=\\\"12\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Mod\\\" rs:number=\\\"13\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Err\\\" rs:number=\\\"14\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"MD\\\" rs:number=\\\"15\\\">
    <s:datatype dt:type=\\\"hexBinary\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"DT\\\" rs:number=\\\"16\\\">
    <s:datatype dt:type=\\\"dateTime\\\"/>
    </s:AttributeType>
    </s:ElementType>
    </s:Schema>
    <rs:data>
    </rs:data>
    </xml>
    <xml xmlns:s=\\\"uuid:BDC6E3F0-6DA3-11d1-A2A3-00AA00C14882\\\"
     xmlns:dt=\\\"uuid:C2F41010-65B3-11d1-A29F-00AA00C14882\\\"
     xmlns:rs=\\\"urn:schemas-microsoft-com:rowset\\\"
     xmlns:z=\\\"#RowsetSchema\\\">
    <s:Schema id=\\\"RowsetSchema\\\">
    <s:ElementType name=\\\"row\\\" content=\\\"eltOnly\\\" rs:updatable=\\\"true\\\">
    <s:AttributeType name=\\\"Cls\\\" rs:number=\\\"0\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Sev\\\" rs:number=\\\"1\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Maj\\\" rs:number=\\\"2\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Min\\\" rs:number=\\\"3\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"LN\\\" rs:number=\\\"4\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fil\\\" rs:number=\\\"5\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fun\\\" rs:number=\\\"6\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Uid\\\" rs:number=\\\"7\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Msg\\\" rs:number=\\\"8\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"PID\\\" rs:number=\\\"9\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"TID\\\" rs:number=\\\"10\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Con\\\" rs:number=\\\"11\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Exe\\\" rs:number=\\\"12\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Mod\\\" rs:number=\\\"13\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Err\\\" rs:number=\\\"14\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"MD\\\" rs:number=\\\"15\\\">
    <s:datatype dt:type=\\\"hexBinary\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"DT\\\" rs:number=\\\"16\\\">
    <s:datatype dt:type=\\\"dateTime\\\"/>
    </s:AttributeType>
    </s:ElementType>
    </s:Schema>
    <rs:data>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070002 in PbrGetOSMetadata (base\\reset\\engine\\scenario\\src\\sensetargetos.cpp:408): Failed to read Compact value from target OS, assuming not compact\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:50\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070001 in PushButtonReset::WofOverlay::EnumOverlays (base\\reset\\util\\src\\wofoverlay.cpp:64): WofEnumEntries failed, assuming no overlays on volume [\\\\?\\Volume{3a642dc6-ed52-4bd2-9229-b048c85196e8}]\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"1\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"127\\\" Fil=\\\"\\\" Fun=\\\"IsNarratorRunning\\\" Uid=\\\"50331648\\\" Msg=\\\"IsNarratorRunning: Error finding window NarratorUIClass; HR = 0x80070002\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070002 in PbrSenseNarrator (base\\reset\\engine\\scenario\\src\\sensemisc.cpp:321): Failed to query whether narrator is running\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070490 in PushButtonReset::TestFlag::Get (base\\reset\\engine\\session\\src\\testflag.cpp:50): Test flag not set: [FormatExclude]\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"234\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] RegOpenKeyEx(GP) failed: 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] WinReGetGroupPolicies failed with error code 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] NOTE: overwrite error code 0x2 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:51\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Clear storage reserve] ([ClearStorageReserve]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [-5379440640] bytes, leaving [961550602240] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [-5368451072] bytes, leaving [961539612672] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"7615\\\" Fil=\\\"\\\" Fun=\\\"pGetAntiVirusInfo\\\" Uid=\\\"51150848\\\" Msg=\\\"Failed to connect securitycenter2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"332\\\" Fil=\\\"\\\" Fun=\\\"CSetupOneSetting::InitializeAndQuery\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to Query Onesettings: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"122\\\" Fil=\\\"\\\" Fun=\\\"CreateSetupOneSettings\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to initialize and query: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"405\\\" Fil=\\\"\\\" Fun=\\\"CSetupPlatformTracing::ReadTracingOneSettings\\\" Uid=\\\"50331648\\\" Msg=\\\"Failed to create OneSettings infrastructure. Error: 0x0880072EE7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"528\\\" Fil=\\\"\\\" Fun=\\\"CSetupPlatformTracing::Initialize\\\" Uid=\\\"50331648\\\" Msg=\\\"No onesetting configured or error getting onesetting\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"14237\\\" Fil=\\\"\\\" Fun=\\\"CSetupPlatform::Initialize\\\" Uid=\\\"51150848\\\" Msg=\\\"CSetupPlatform::Initialize: Failed to initialize tracing\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"203\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:53\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Delete OS uninstall image] ([DeleteUninstall]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [961539612672] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [57344] bytes, leaving [961539555328] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070005 in PushButtonReset::Directory::EnumExcept (base\\reset\\util\\src\\filesystem.cpp:2008): Failed to check whether [C:\\hiberfil.sys] is a child path of [C:\\DumpStack.log.tmp], assuming no\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"5\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070005 in PushButtonReset::Directory::EnumExcept (base\\reset\\util\\src\\filesystem.cpp:2008): Failed to check whether [C:\\hiberfil.sys] is a child path of [C:\\pagefile.sys], assuming no\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"5\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070005 in PushButtonReset::Directory::EnumExcept (base\\reset\\util\\src\\filesystem.cpp:2008): Failed to check whether [C:\\hiberfil.sys] is a child path of [C:\\swapfile.sys], assuming no\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"5\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Archive user data files] ([ArchiveUserData]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [961539555328] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [8192] bytes, leaving [961539547136] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"7615\\\" Fil=\\\"\\\" Fun=\\\"pGetAntiVirusInfo\\\" Uid=\\\"51150848\\\" Msg=\\\"Failed to connect securitycenter2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"332\\\" Fil=\\\"\\\" Fun=\\\"CSetupOneSetting::InitializeAndQuery\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to Query Onesettings: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"122\\\" Fil=\\\"\\\" Fun=\\\"CreateSetupOneSettings\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to initialize and query: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"405\\\" Fil=\\\"\\\" Fun=\\\"CSetupPlatformTracing::ReadTracingOneSettings\\\" Uid=\\\"50331648\\\" Msg=\\\"Failed to create OneSettings infrastructure. Error: 0x0880072EE7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"528\\\" Fil=\\\"\\\" Fun=\\\"CSetupPlatformTracing::Initialize\\\" Uid=\\\"50331648\\\" Msg=\\\"No onesetting configured or error getting onesetting\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"14237\\\" Fil=\\\"\\\" Fun=\\\"CSetupPlatform::Initialize\\\" Uid=\\\"51150848\\\" Msg=\\\"CSetupPlatform::Initialize: Failed to initialize tracing\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"203\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:54\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"332\\\" Fil=\\\"\\\" Fun=\\\"CSetupOneSetting::InitializeAndQuery\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to Query Onesettings: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:55\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"122\\\" Fil=\\\"\\\" Fun=\\\"CreateSetupOneSettings\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to initialize and query: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:55\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"4079\\\" Fil=\\\"\\\" Fun=\\\"CNewSystem::PreInitialize\\\" Uid=\\\"51150848\\\" Msg=\\\"Failed to create OneSettings infrastructure. Error: 0x0880072EE7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:55\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"332\\\" Fil=\\\"\\\" Fun=\\\"CSetupOneSetting::InitializeAndQuery\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to Query Onesettings: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"122\\\" Fil=\\\"\\\" Fun=\\\"CreateSetupOneSettings\\\" Uid=\\\"50331648\\\" Msg=\\\"Onesettings: Failed to initialize and query: 0x80072ee7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5274\\\" Fil=\\\"\\\" Fun=\\\"CNewSystem::QueueOperations\\\" Uid=\\\"51150848\\\" Msg=\\\"Failed to create OneSettings infrastructure. Error: 0x0880072EE7\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"1030\\\" Fil=\\\"\\\" Fun=\\\"SPDeleteOldUpgradeSnapshots\\\" Uid=\\\"51150848\\\" Msg=\\\"SPDeleteOldUpgradeSnapshots: Cannot open snapshot key, assume not exist. Error: 0x00000002\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:01:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"1030\\\" Fil=\\\"\\\" Fun=\\\"SPDeleteOldUpgradeSnapshots\\\" Uid=\\\"51150848\\\" Msg=\\\"    SPDeleteOldUpgradeSnapshots: Cannot open snapshot key, assume not exist. Error: 0x00000002\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:02:00\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 8192 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:02:00\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 532480 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:46\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 12288 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:46\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 8192 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 8192 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"3793\\\" Fil=\\\"\\\" Fun=\\\"SPCalculateDriveMappings\\\" Uid=\\\"51150848\\\" Msg=\\\"    SPCalculateDriveMappings: C:\\ already maps to C:\\, ignoring the newer mapping C:\\\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 36864 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:52\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 4096 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:06:52\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"420\\\" Fil=\\\"\\\" Fun=\\\"CGlobalPath::FindGlobalPathCallback\\\" Uid=\\\"51150848\\\" Msg=\\\"    FindGlobalPath: Cannot find volume name for \\\\?\\GLOBALROOT\\Device\\HardDisk0\\Partition2. Error: 0x0000001F\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"31\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:09\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 149057536 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:35\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 262144 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:45\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 1208320 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:45\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 29331456 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:46\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 28672 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:46\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"3194\\\" Fil=\\\"\\\" Fun=\\\"SPMoveFileWithShortName\\\" Uid=\\\"51150848\\\" Msg=\\\"    SPMoveFileWithShortName: Failed to move C:\\inetpub to C:\\Windows.old\\inetpub, error: 0x00000002\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"3194\\\" Fil=\\\"\\\" Fun=\\\"SPMoveFileWithShortName\\\" Uid=\\\"51150848\\\" Msg=\\\"    SPMoveFileWithShortName: Failed to move C:\\SkyDriveTemp to C:\\Windows.old\\SkyDriveTemp, error: 0x00000002\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 8192 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"214\\\" Fil=\\\"\\\" Fun=\\\"CAddProvisioningPackage::DoExecute\\\" Uid=\\\"51150848\\\" Msg=\\\"    CAddProvisioningPackage::DoExecute: Failed to initialize COM security. Was it initialized before us? hr = 0x80010119\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"183\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 274432 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 65536 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:47\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"5103\\\" Fil=\\\"\\\" Fun=\\\"COperationQueue::ExecuteOperationsInternal\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACEEXCEED: Operation consumed more disk space than declared. Exceeded by 8388608 bytes\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"12967\\\" Fil=\\\"\\\" Fun=\\\"CNewSystem::Finalize\\\" Uid=\\\"51150848\\\" Msg=\\\"DISKSPACETRACK: Size of SafeOS WIM C:\\$WINDOWS.~BT\\Sources\\SafeOS\\winre.wim is 0\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"SetupPlatform.dll\\\" Err=\\\"183\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070005 in PushButtonReset::OpExecSetup::InternalExecute (base\\reset\\engine\\operations\\src\\execsetup.cpp:2308): Failed to move log directory\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"5\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"236\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::TraceErr\\\" Uid=\\\"50331648\\\" Msg=\\\"0x80070020 in PushButtonReset::OpMigrateSettings::MigrateFiles (base\\reset\\engine\\operations\\src\\migratesettings.cpp:987): Failed to copy file [C:\\Windows.old\\Windows\\containers\\serviced\\WindowsDefenderApplicationGuard.wim] to [C:\\Windows\\containers\\serviced\\WindowsDefenderApplicationGuard.wim]\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"32\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:49\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Execute PBR plugins] ([ExecutePbrPlugin]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:50\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [960654041088] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:50\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [1310720] bytes, leaving [960652730368] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:50\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Migrate AppX Provisioned Apps] ([MigrateProvisionedApps]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [960652730368] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [24829952] bytes, leaving [960627900416] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] failed to get child attribute by tag: 0xd\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] ReAgentConfig::ReadBcdAndUpdateEnhancedConfigInfo GetOsInfoForBootEntry returned 0x2 \\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  failed to add trailing back slash to string  (0x57) in file base\\diagnosis\\srt\\reagent2\\reinfo\\shared.cpp line 873\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  overwrites error code 0x57 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  failed to add trailing back slash to string  (0x57) in file base\\diagnosis\\srt\\reagent2\\reinfo\\shared.cpp line 873\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  overwrites error code 0x57 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  failed to add trailing back slash to string  (0x57) in file base\\diagnosis\\srt\\reagent2\\reinfo\\shared.cpp line 873\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  overwrites error code 0x57 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  failed to add trailing back slash to string  (0x57) in file base\\diagnosis\\srt\\reagent2\\reinfo\\shared.cpp line 873\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  overwrites error code 0x57 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  failed to add trailing back slash to string  (0x57) in file base\\diagnosis\\srt\\reagent2\\reinfo\\shared.cpp line 873\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  overwrites error code 0x57 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  failed to add trailing back slash to string  (0x57) in file base\\diagnosis\\srt\\reagent2\\reinfo\\shared.cpp line 873\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe]  overwrites error code 0x57 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] WinReRestoreConfigAfterPBR Failed to find a recovery image\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Restore WinRE information] ([RestoreWinRE]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [960627900416] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [32768] bytes, leaving [960627867648] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] read xml file (C:\\Recovery\\ReAgentOld.xml) failed: 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] ReAgentXMLParser::ParseConfigFile failed to read config xml file (0x2) in file base\\diagnosis\\srt\\reagent2\\reinfo\\parser_2.0.cpp line 825\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] ReAgentXMLParser::ParseConfigFile (xml file: C:\\Recovery\\ReAgentOld.xml) returning 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] Failed to get recovery entries: 0xc0000225\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Install WinRE on target OS] ([InstallWinRE]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [960627867648] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [4096] bytes, leaving [960627863552] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:07:58\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] RegOpenKeyEx(GP) failed: 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:08:02\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] WinReGetGroupPolicies failed with error code 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:08:02\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] NOTE: overwrite error code 0x2 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:08:02\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Delete old OS files] ([DeleteOldOS]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:10:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [-68206202880] bytes, leaving [1032894144512] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:10:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [-15929724928] bytes, leaving [980617666560] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:10:48\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"Operation [Decrypt disk [0] partition offset [290455552]] ([DecryptVolume]) consumed more disk space than expected\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:33\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Expected to use [0] bytes, leaving [980617666560] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:33\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"169\\\" Fil=\\\"\\\" Fun=\\\"PushButtonReset::Logging::Trace\\\" Uid=\\\"50331648\\\" Msg=\\\"-&gt; Actually used [835584] bytes, leaving [980616830976] bytes free\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ResetEngine.dll\\\" Err=\\\"2\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:33\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] RegOpenKeyEx(GP) failed: 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:33\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] WinReGetGroupPolicies failed with error code 0x2\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:33\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[sysreset.exe] NOTE: overwrite error code 0x2 because it is not critical\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:33\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"215\\\" Fil=\\\"\\\" Fun=\\\"DoTraceMessage\\\" Uid=\\\"50331648\\\" Msg=\\\"StopUserModeTrace failed\\\" PID=\\\"1516\\\" TID=\\\"1520\\\" Con=\\\"\\\" Exe=\\\"X:\\windows\\system32\\sysreset.exe\\\" Mod=\\\"\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:12:34\\\"/>
    </rs:data>
    </xml>
    <xml xmlns:s=\\\"uuid:BDC6E3F0-6DA3-11d1-A2A3-00AA00C14882\\\"
     xmlns:dt=\\\"uuid:C2F41010-65B3-11d1-A29F-00AA00C14882\\\"
     xmlns:rs=\\\"urn:schemas-microsoft-com:rowset\\\"
     xmlns:z=\\\"#RowsetSchema\\\">
    <s:Schema id=\\\"RowsetSchema\\\">
    <s:ElementType name=\\\"row\\\" content=\\\"eltOnly\\\" rs:updatable=\\\"true\\\">
    <s:AttributeType name=\\\"Cls\\\" rs:number=\\\"0\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Sev\\\" rs:number=\\\"1\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Maj\\\" rs:number=\\\"2\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Min\\\" rs:number=\\\"3\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"LN\\\" rs:number=\\\"4\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fil\\\" rs:number=\\\"5\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fun\\\" rs:number=\\\"6\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Uid\\\" rs:number=\\\"7\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Msg\\\" rs:number=\\\"8\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"PID\\\" rs:number=\\\"9\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"TID\\\" rs:number=\\\"10\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Con\\\" rs:number=\\\"11\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Exe\\\" rs:number=\\\"12\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Mod\\\" rs:number=\\\"13\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Err\\\" rs:number=\\\"14\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"MD\\\" rs:number=\\\"15\\\">
    <s:datatype dt:type=\\\"hexBinary\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"DT\\\" rs:number=\\\"16\\\">
    <s:datatype dt:type=\\\"dateTime\\\"/>
    </s:AttributeType>
    </s:ElementType>
    </s:Schema>
    <rs:data>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[ResetEngine.exe] RegOpenKeyEx(GP) failed: 0x2\\\" PID=\\\"8432\\\" TID=\\\"8436\\\" Con=\\\"\\\" Exe=\\\"C:\\Windows\\System32\\ResetEngine.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:16:50\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[ResetEngine.exe] WinReGetGroupPolicies failed with error code 0x2\\\" PID=\\\"8432\\\" TID=\\\"8436\\\" Con=\\\"\\\" Exe=\\\"C:\\Windows\\System32\\ResetEngine.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:16:50\\\"/>
    <z:row Cls=\\\"D\\\" Sev=\\\"50331648\\\" Maj=\\\"Def\\\" Min=\\\"Def\\\" LN=\\\"472\\\" Fil=\\\"\\\" Fun=\\\"UnattendLogWV\\\" Uid=\\\"50331648\\\" Msg=\\\"[ResetEngine.exe] NOTE: overwrite error code 0x2 because it is not critical\\\" PID=\\\"8432\\\" TID=\\\"8436\\\" Con=\\\"\\\" Exe=\\\"C:\\Windows\\System32\\ResetEngine.exe\\\" Mod=\\\"ReAgent.dll\\\" Err=\\\"0\\\" MD=\\\"\\\" DT=\\\"2022-12-05T15:16:50\\\"/>
    </rs:data>
    </xml>
    <xml xmlns:s=\\\"uuid:BDC6E3F0-6DA3-11d1-A2A3-00AA00C14882\\\"
     xmlns:dt=\\\"uuid:C2F41010-65B3-11d1-A29F-00AA00C14882\\\"
     xmlns:rs=\\\"urn:schemas-microsoft-com:rowset\\\"
     xmlns:z=\\\"#RowsetSchema\\\">
    <s:Schema id=\\\"RowsetSchema\\\">
    <s:ElementType name=\\\"row\\\" content=\\\"eltOnly\\\" rs:updatable=\\\"true\\\">
    <s:AttributeType name=\\\"Cls\\\" rs:number=\\\"0\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Sev\\\" rs:number=\\\"1\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Maj\\\" rs:number=\\\"2\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Min\\\" rs:number=\\\"3\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"LN\\\" rs:number=\\\"4\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fil\\\" rs:number=\\\"5\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Fun\\\" rs:number=\\\"6\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Uid\\\" rs:number=\\\"7\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Msg\\\" rs:number=\\\"8\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"PID\\\" rs:number=\\\"9\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"TID\\\" rs:number=\\\"10\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Con\\\" rs:number=\\\"11\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Exe\\\" rs:number=\\\"12\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Mod\\\" rs:number=\\\"13\\\">
    <s:datatype dt:type=\\\"string\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"Err\\\" rs:number=\\\"14\\\">
    <s:datatype dt:type=\\\"int\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"MD\\\" rs:number=\\\"15\\\">
    <s:datatype dt:type=\\\"hexBinary\\\"/>
    </s:AttributeType>
    <s:AttributeType name=\\\"DT\\\" rs:number=\\\"16\\\">
    <s:datatype dt:type=\\\"dateTime\\\"/>
    </s:AttributeType>
    </s:ElementType>
    </s:Schema>
    <rs:data>
    </rs:data>
    </xml>

    \"}";

        #[derive( Deserialize, Debug, Clone)]
        pub struct FileDescription {
            pub id: i32,
            pub path: String,
            pub internal: Option<String>,
            pub disk: String,
            pub size: i32,
            pub modified: i32,
            pub content: Option<String>,
        }
        let new_str = str.replace("\n", "\\n");
        println!("{}", new_str);
        let r: FileDescription = from_str(new_str.as_str()).unwrap();

    }
}
