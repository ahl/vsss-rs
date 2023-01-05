mod feldman;
mod pedersen;
mod polynomial;
mod shamir;
mod share;
mod verifier;

pub use feldman::*;
pub use pedersen::*;
pub use polynomial::*;
pub use shamir::*;
pub use share::*;
pub use verifier::*;

use crate::lib::String;
use crate::util::*;
use core::{
    marker::PhantomData,
    fmt::{self, Formatter},
};
use std::prelude::v1::Vec;
use elliptic_curve::{ff::PrimeField, group::{Group, GroupEncoding}};
use serde::{Serializer, Deserializer, de::{Visitor, SeqAccess, Error, Unexpected}, ser::{SerializeTuple, SerializeSeq}};

pub(crate) fn serialize_field<F: PrimeField, S: Serializer>(f: &F, s: S) -> Result<S::Ok, S::Error> {
    let repr = f.to_repr();
    serialize_ref(repr, s)
}

pub(crate) fn serialize_group<G: Group + GroupEncoding, S: Serializer>(g: &G, s: S) -> Result<S::Ok, S::Error> {
   let repr = g.to_bytes();
    serialize_ref(repr, s)
}

pub(crate) fn serialize_field_vec<F: PrimeField, S: Serializer>(f: &Vec<F>, s: S) -> Result<S::Ok, S::Error> {
    let is_human_readable = s.is_human_readable();
    let mut sequencer = s.serialize_seq(Some(f.len()))?;
    if is_human_readable {
        for ff in f {
            sequencer.serialize_element(&hex::encode(ff.to_repr().as_ref()))?;
        }
    } else {
        let repr = F::Repr::default();
        let mut output = Vec::with_capacity(uint_zigzag::Uint::MAX_BYTES + repr.as_ref().len() * f.len());
        let len = uint_zigzag::Uint::from(f.len());
        output.append(&mut len.to_vec());

        for ff in f {
            let repr = ff.to_repr();
            output.extend_from_slice(repr.as_ref());
        }
    }
    sequencer.end()
}

pub(crate) fn serialize_group_vec<G: Group + GroupEncoding, S: Serializer>(g: &Vec<G>, s: S) -> Result<S::Ok, S::Error> {
    let is_human_readable = s.is_human_readable();
    let mut sequencer = s.serialize_seq(Some(g.len()))?;
    if is_human_readable {
        for gg in g {
            sequencer.serialize_element(&hex::encode(gg.to_bytes().as_ref()))?;
        }
    } else {
        let repr = G::Repr::default();
        let mut output = Vec::with_capacity(uint_zigzag::Uint::MAX_BYTES + repr.as_ref().len() * g.len());
        let len = uint_zigzag::Uint::from(g.len());
        output.append(&mut len.to_vec());

        for gg in g {
            let repr = gg.to_bytes();
            output.extend_from_slice(repr.as_ref());
        }
    }
    sequencer.end()
}

fn serialize_ref<B: AsRef<[u8]>, S: Serializer>(bytes: B, s: S) -> Result<S::Ok, S::Error> {
    if s.is_human_readable() {
        let h = hex::encode(bytes.as_ref());
        s.serialize_str(&h)
    } else {
        let bytes = bytes.as_ref();
        let mut tupler = s.serialize_tuple(bytes.len())?;
        for b in bytes {
            tupler.serialize_element(b)?;
        }
        tupler.end()
    }
}

pub(crate) fn deserialize_field<'de, F: PrimeField, D: Deserializer<'de>>(d: D) -> Result<F, D::Error> {
    struct FieldVisitor<F: PrimeField> { marker: PhantomData<F> }

    impl<'de, F: PrimeField> Visitor<'de> for FieldVisitor<F> {
        type Value = F;

        fn expecting(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "a byte sequence or string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
            let bytes = hex::decode(v).map_err(|_e| Error::invalid_value(Unexpected::Str(v), &self))?;
            bytes_to_field(&bytes).ok_or(Error::custom("unable to convert to a field element"))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
            let mut bytes = Vec::new();
            while let Some(b) = seq.next_element()? {
                bytes.push(b);
            }

            bytes_to_field(&bytes).ok_or(Error::custom("unable to convert to a field element"))
        }
    }

    let v = FieldVisitor { marker: PhantomData::<F> };
    if d.is_human_readable() {
        d.deserialize_str(v)
    } else {
        let repr = F::Repr::default();
        let len = repr.as_ref().len();
        d.deserialize_tuple(len, v)
    }
}

pub(crate) fn deserialize_group<'de, G: Group + GroupEncoding, D: Deserializer<'de>>(d: D) -> Result<G, D::Error> {
    struct GroupVisitor<G: Group + GroupEncoding> { marker: PhantomData<G> }

    impl<'de, G: Group + GroupEncoding> Visitor<'de> for GroupVisitor<G> {
        type Value = G;

        fn expecting(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "a byte sequence or string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
            let bytes = hex::decode(v).map_err(|_e| Error::invalid_value(Unexpected::Str(v), &self))?;
            bytes_to_group(&bytes).ok_or(Error::custom("unable to convert to a group element"))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
            let mut bytes = Vec::new();
            while let Some(b) = seq.next_element()? {
                bytes.push(b);
            }

            bytes_to_group(&bytes).ok_or(Error::custom("unable to convert to a group element"))
        }
    }

    let v = GroupVisitor { marker: PhantomData::<G> };
    if d.is_human_readable() {
        d.deserialize_str(v)
    } else {
        let repr = G::Repr::default();
        let len = repr.as_ref().len();
        d.deserialize_tuple(len, v)
    }
}

pub(crate) fn deserialize_field_vec<'de, F: PrimeField, D: Deserializer<'de>>(d: D) -> Result<Vec<F>, D::Error> {
    struct FieldVecVisitor<F: PrimeField> {
        is_human_readable: bool,
        marker: PhantomData<F>
    }

    impl<'de, F: PrimeField> Visitor<'de> for FieldVecVisitor<F> {
        type Value = Vec<F>;

        fn expecting(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "a byte sequence or strings")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
            let mut values = Vec::new();
            if self.is_human_readable {
                while let Some(s) = seq.next_element()?.ok_or(Error::invalid_length(values.len(), &self))? {
                    let bytes = hex::decode::<&String>(&s).map_err(|_| Error::invalid_length(values.len(), &self))?;
                    let f = bytes_to_field(&bytes).ok_or(Error::invalid_value(Unexpected::Bytes(&bytes), &self))?;
                    values.push(f);
                }
            } else {
                let mut buffer = [0u8; uint_zigzag::Uint::MAX_BYTES];
                let mut i = 0;
                while let Some(b) = seq.next_element()? {
                    buffer[i] = b;
                    if i == uint_zigzag::Uint::MAX_BYTES {
                        break;
                    }
                }
                let bytes_cnt_size = uint_zigzag::Uint::peek(&buffer)
                    .ok_or_else(|| Error::invalid_value(Unexpected::Bytes(&buffer), &self))?;
                let fields = uint_zigzag::Uint::try_from(&buffer[..bytes_cnt_size])
                    .map_err(|_| Error::invalid_value(Unexpected::Bytes(&buffer), &self))?;

                i = uint_zigzag::Uint::MAX_BYTES - bytes_cnt_size;
                let mut repr = F::Repr::default();
                {
                    let r = repr.as_mut();
                    r[..i].copy_from_slice(&buffer[bytes_cnt_size..]);
                }
                let repr_len = repr.as_ref().len();
                values.reserve(fields.0 as usize);
                while let Some(b) = seq.next_element()? {
                    repr.as_mut()[i] = b;
                    if i == repr_len {
                        i = 0;
                        let pt = F::from_repr(repr);
                        if pt.is_none().unwrap_u8() == 1u8 {
                            return Err(Error::invalid_value(Unexpected::Bytes(&buffer), &self));
                        }
                        values.push(pt.unwrap());
                        if values.len() == fields.0 as usize {
                            break;
                        }
                    }
                    i += 1;
                }
                if values.len() != fields.0 as usize {
                    return Err(Error::invalid_length(values.len(), &self));
                }
            }
            Ok(values)
        }
    }

    let v = FieldVecVisitor {
        is_human_readable: d.is_human_readable(),
        marker: PhantomData::<F>
    };
    d.deserialize_seq(v)
}

pub(crate) fn deserialize_group_vec<'de, G: Group + GroupEncoding, D: Deserializer<'de>>(d: D) -> Result<Vec<G>, D::Error> {
    struct GroupVecVisitor<G: Group + GroupEncoding> {
        is_human_readable: bool,
        marker: PhantomData<G>
    }

    impl<'de, G: Group + GroupEncoding> Visitor<'de> for GroupVecVisitor<G> {
        type Value = Vec<G>;

        fn expecting(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "a byte sequence or strings")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
            let mut values = Vec::new();
            if self.is_human_readable {
                while let Some(s) = seq.next_element()?.ok_or(Error::invalid_length(values.len(), &self))? {
                    let bytes = hex::decode::<&String>(&s).map_err(|_| Error::invalid_length(values.len(), &self))?;
                    let f = bytes_to_group(&bytes).ok_or(Error::invalid_value(Unexpected::Bytes(&bytes), &self))?;
                    values.push(f);
                }
            } else {
                let mut buffer = [0u8; uint_zigzag::Uint::MAX_BYTES];
                let mut i = 0;
                while let Some(b) = seq.next_element()? {
                    buffer[i] = b;
                    if i == uint_zigzag::Uint::MAX_BYTES {
                        break;
                    }
                }
                let bytes_cnt_size = uint_zigzag::Uint::peek(&buffer)
                    .ok_or_else(|| Error::invalid_value(Unexpected::Bytes(&buffer), &self))?;
                let groups = uint_zigzag::Uint::try_from(&buffer[..bytes_cnt_size])
                    .map_err(|_| Error::invalid_value(Unexpected::Bytes(&buffer), &self))?;

                i = uint_zigzag::Uint::MAX_BYTES - bytes_cnt_size;
                let mut repr = G::Repr::default();
                {
                    let r = repr.as_mut();
                    r[..i].copy_from_slice(&buffer[bytes_cnt_size..]);
                }
                let repr_len = repr.as_ref().len();
                values.reserve(groups.0 as usize);
                while let Some(b) = seq.next_element()? {
                    repr.as_mut()[i] = b;
                    if i == repr_len {
                        i = 0;
                        let pt = G::from_bytes(&repr);
                        if pt.is_none().unwrap_u8() == 1u8 {
                            return Err(Error::invalid_value(Unexpected::Bytes(&buffer), &self));
                        }
                        values.push(pt.unwrap());
                        if values.len() == groups.0 as usize {
                            break;
                        }
                    }
                    i += 1;
                }
                if values.len() != groups.0 as usize {
                    return Err(Error::invalid_length(values.len(), &self));
                }
            }
            Ok(values)
        }
    }

    let v = GroupVecVisitor {
        is_human_readable: d.is_human_readable(),
        marker: PhantomData::<G>
    };
    d.deserialize_seq(v)
}
