#![allow(async_fn_in_trait)]

use build_async::*;
use std::convert::TryInto;
use std::fmt::Debug;

//------Decoder----------------------------

macro_rules! fn_decode_arr {
    ($ty:ty) => {
        paste::item! {
            #[_async] fn [<decode_arr_ $ty>](&mut self, len: Option<usize>) -> Result<Vec<$ty>, Self::Error> {
                _await!(self.[<decode_vec_ $ty>](len))
            }
        }
    };
}

pub trait Decoder {
    type Error;

    // scalars
    #[_async]
    fn decode_bool(&mut self) -> Result<bool, Self::Error>;
    #[_async]
    fn decode_char(&mut self) -> Result<char, Self::Error>;
    #[_async]
    fn decode_u8(&mut self) -> Result<u8, Self::Error>;
    #[_async]
    fn decode_i8(&mut self) -> Result<i8, Self::Error>;
    #[_async]
    fn decode_u16(&mut self) -> Result<u16, Self::Error>;
    #[_async]
    fn decode_i16(&mut self) -> Result<i16, Self::Error>;
    #[_async]
    fn decode_u32(&mut self) -> Result<u32, Self::Error>;
    #[_async]
    fn decode_i32(&mut self) -> Result<i32, Self::Error>;
    #[_async]
    fn decode_u64(&mut self) -> Result<u64, Self::Error>;
    #[_async]
    fn decode_i64(&mut self) -> Result<i64, Self::Error>;
    #[_async]
    fn decode_u128(&mut self) -> Result<u128, Self::Error>;
    #[_async]
    fn decode_i128(&mut self) -> Result<i128, Self::Error>;
    #[_async]
    fn decode_f32(&mut self) -> Result<f32, Self::Error>;
    #[_async]
    fn decode_f64(&mut self) -> Result<f64, Self::Error>;

    // string and binary blob
    #[_async]
    fn decode_string(&mut self) -> Result<String, Self::Error>;
    #[_async]
    fn decode_binary(&mut self, size: Option<usize>) -> Result<Vec<u8>, Self::Error>;

    // raw fixed size (arr) and variable size (vec) arrays
    #[_async]
    fn decode_vec_bool(&mut self, len: Option<usize>) -> Result<Vec<bool>, Self::Error>;
    #[_async]
    fn decode_vec_char(&mut self, len: Option<usize>) -> Result<Vec<char>, Self::Error>;
    #[_async]
    fn decode_vec_u8(&mut self, len: Option<usize>) -> Result<Vec<u8>, Self::Error>;
    #[_async]
    fn decode_vec_i8(&mut self, len: Option<usize>) -> Result<Vec<i8>, Self::Error>;
    #[_async]
    fn decode_vec_u16(&mut self, len: Option<usize>) -> Result<Vec<u16>, Self::Error>;
    #[_async]
    fn decode_vec_i16(&mut self, len: Option<usize>) -> Result<Vec<i16>, Self::Error>;
    #[_async]
    fn decode_vec_u32(&mut self, len: Option<usize>) -> Result<Vec<u32>, Self::Error>;
    #[_async]
    fn decode_vec_i32(&mut self, len: Option<usize>) -> Result<Vec<i32>, Self::Error>;
    #[_async]
    fn decode_vec_u64(&mut self, len: Option<usize>) -> Result<Vec<u64>, Self::Error>;
    #[_async]
    fn decode_vec_i64(&mut self, len: Option<usize>) -> Result<Vec<i64>, Self::Error>;
    #[_async]
    fn decode_vec_u128(&mut self, len: Option<usize>) -> Result<Vec<u128>, Self::Error>;
    #[_async]
    fn decode_vec_i128(&mut self, len: Option<usize>) -> Result<Vec<i128>, Self::Error>;
    #[_async]
    fn decode_vec_f32(&mut self, len: Option<usize>) -> Result<Vec<f32>, Self::Error>;
    #[_async]
    fn decode_vec_f64(&mut self, len: Option<usize>) -> Result<Vec<f64>, Self::Error>;

    fn_decode_arr! {bool}
    fn_decode_arr! {char}
    fn_decode_arr! {u8}
    fn_decode_arr! {u16}
    fn_decode_arr! {u32}
    fn_decode_arr! {u64}
    fn_decode_arr! {u128}
    fn_decode_arr! {i8}
    fn_decode_arr! {i16}
    fn_decode_arr! {i32}
    fn_decode_arr! {i64}
    fn_decode_arr! {i128}
    fn_decode_arr! {f32}
    fn_decode_arr! {f64}

    // fixed and variable size sequences of elements
    #[_async]
    fn decode_arr_begin(&mut self, _len: usize) -> Result<usize, Self::Error> {
        _await!(self.decode_vec_begin())
    }
    #[_async]
    fn decode_arr_end(&mut self) -> Result<(), Self::Error> {
        _await!(self.decode_vec_end())
    }
    #[_async]
    fn decode_vec_begin(&mut self) -> Result<usize, Self::Error> {
        _await!(self.decode_seq_begin(None))
    }
    #[_async]
    fn decode_vec_end(&mut self) -> Result<(), Self::Error> {
        _await!(self.decode_seq_end())
    }

    // sequences
    #[_async]
    fn decode_seq_begin(&mut self, len: Option<usize>) -> Result<usize, Self::Error>;
    #[_async]
    fn decode_seq_end(&mut self) -> Result<(), Self::Error>;

    // enums
    #[_async]
    fn decode_enum_begin(&mut self, enum_name: &str) -> Result<(u32, usize), Self::Error>;
    #[_async]
    fn decode_enum_end(&mut self) -> Result<(), Self::Error>;

    // structs
    #[_async]
    fn decode_struct_begin(
        &mut self,
        len: usize,
        struct_name: Option<&str>,
    ) -> Result<usize, Self::Error>;
    #[_async]
    fn decode_struct_end(&mut self) -> Result<(), Self::Error>;

    // seq or struct/enum element
    #[_async]
    fn decode_elem_begin(
        &mut self,
        _index: usize,
        _elem_name: Option<&str>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    #[_async]
    fn decode_elem_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn decode_bytes_begin(&mut self, size: Option<usize>) -> Result<usize, Self::Error>;
    #[_async]
    fn decode_bytes_payload(&mut self, value: &mut [u8]) -> Result<usize, Self::Error>;
    #[_async]
    fn decode_bytes_end(&mut self) -> Result<(), Self::Error>;
    #[_async]
    fn decode_uint(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error>;
    #[_async]
    fn decode_skip(&mut self, n: usize) -> Result<(), Self::Error>;
}

//--------Encoder--------------------
macro_rules! fn_encode_arr {
    ($ty:ty) => {
        paste::item! {
            #[_async] fn [<encode_arr_ $ty>](&mut self, values: &[$ty]) -> Result<(), Self::Error> {
                _await!(self.[<encode_vec_ $ty>](values))
            }
        }
    };
}

pub trait Encoder {
    type Error;

    // scalars
    #[_async]
    fn encode_bool(&mut self, value: &bool) -> Result<(), Self::Error>;
    #[_async]
    fn encode_char(&mut self, value: &char) -> Result<(), Self::Error>;
    #[_async]
    fn encode_u8(&mut self, value: &u8) -> Result<(), Self::Error>;
    #[_async]
    fn encode_i8(&mut self, value: &i8) -> Result<(), Self::Error>;
    #[_async]
    fn encode_u16(&mut self, value: &u16) -> Result<(), Self::Error>;
    #[_async]
    fn encode_i16(&mut self, value: &i16) -> Result<(), Self::Error>;
    #[_async]
    fn encode_u32(&mut self, value: &u32) -> Result<(), Self::Error>;
    #[_async]
    fn encode_i32(&mut self, value: &i32) -> Result<(), Self::Error>;
    #[_async]
    fn encode_u64(&mut self, value: &u64) -> Result<(), Self::Error>;
    #[_async]
    fn encode_i64(&mut self, value: &i64) -> Result<(), Self::Error>;
    #[_async]
    fn encode_u128(&mut self, value: &u128) -> Result<(), Self::Error>;
    #[_async]
    fn encode_i128(&mut self, value: &i128) -> Result<(), Self::Error>;
    #[_async]
    fn encode_f32(&mut self, value: &f32) -> Result<(), Self::Error>;
    #[_async]
    fn encode_f64(&mut self, value: &f64) -> Result<(), Self::Error>;

    // string and binary blob
    #[_async]
    fn encode_binary(&mut self, value: &[u8]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_string(&mut self, value: &str) -> Result<(), Self::Error>;

    // raw fixed size (arr) and variable size (vec) arrays
    #[_async]
    fn encode_vec_bool(&mut self, values: &[bool]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_char(&mut self, values: &[char]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_u8(&mut self, values: &[u8]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_i8(&mut self, values: &[i8]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_u16(&mut self, values: &[u16]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_i16(&mut self, values: &[i16]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_u32(&mut self, values: &[u32]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_i32(&mut self, values: &[i32]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_u64(&mut self, values: &[u64]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_i64(&mut self, values: &[i64]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_u128(&mut self, values: &[u128]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_i128(&mut self, values: &[i128]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_f32(&mut self, values: &[f32]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_vec_f64(&mut self, values: &[f64]) -> Result<(), Self::Error>;

    fn_encode_arr! {bool}
    fn_encode_arr! {char}
    fn_encode_arr! {u8}
    fn_encode_arr! {u16}
    fn_encode_arr! {u32}
    fn_encode_arr! {u64}
    fn_encode_arr! {u128}
    fn_encode_arr! {i8}
    fn_encode_arr! {i16}
    fn_encode_arr! {i32}
    fn_encode_arr! {i64}
    fn_encode_arr! {i128}
    fn_encode_arr! {f32}
    fn_encode_arr! {f64}

    // fixed and variable size sequences of elements
    #[_async]
    fn encode_arr_begin(&mut self, len: usize) -> Result<(), Self::Error> {
        _await!(self.encode_vec_begin(len))
    }
    #[_async]
    fn encode_arr_end(&mut self) -> Result<(), Self::Error> {
        _await!(self.encode_vec_end())
    }

    #[_async]
    fn encode_vec_begin(&mut self, len: usize) -> Result<(), Self::Error> {
        _await!(self.encode_seq_begin(len))
    }
    #[_async]
    fn encode_vec_end(&mut self) -> Result<(), Self::Error> {
        _await!(self.encode_seq_end())
    }

    // sequences
    #[_async]
    fn encode_seq_begin(&mut self, len: usize) -> Result<(), Self::Error>;
    #[_async]
    fn encode_seq_end(&mut self) -> Result<(), Self::Error>;

    // enums
    #[_async]
    fn encode_enum_begin(
        &mut self,
        enum_tag: u32,
        len: usize, // 0 or 1
        enum_name: &str,
        variant_name: &str,
    ) -> Result<(), Self::Error>;
    #[_async]
    fn encode_enum_end(&mut self) -> Result<(), Self::Error>;

    // structs
    #[_async]
    fn encode_struct_begin(
        &mut self,
        len: usize,
        struct_name: Option<&str>,
    ) -> Result<(), Self::Error>;
    #[_async]
    fn encode_struct_end(&mut self) -> Result<(), Self::Error>;

    // seq or struct/enum element
    #[_async]
    fn encode_elem_begin(
        &mut self,
        _index: usize,
        _elem_name: Option<&str>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    #[_async]
    fn encode_elem_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn encode_bytes_begin(&mut self, _size: usize) -> Result<(), Self::Error>;
    #[_async]
    fn encode_bytes_payload(&mut self, _value: &[u8]) -> Result<(), Self::Error>;
    #[_async]
    fn encode_bytes_end(&mut self) -> Result<(), Self::Error>;
    #[_async]
    fn encode_uint(&mut self, _bytes: &[u8]) -> Result<(), Self::Error>;
}

//-------Decode-----------------------

pub trait Decode {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error>
    where
        Self: Sized;
}

macro_rules! impl_decode {
    ($ty:ty) => {
        paste::item! {
            impl Decode for $ty {
                #[_async] fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
                    Ok(_await!(decoder.[<decode_ $ty>]())?)
                }
            }
        }
    };
}

impl_decode! {bool}
impl_decode! {char}
impl_decode! {u8}
impl_decode! {u16}
impl_decode! {u32}
impl_decode! {u64}
impl_decode! {u128}
impl_decode! {i8}
impl_decode! {i16}
impl_decode! {i32}
impl_decode! {i64}
impl_decode! {i128}
impl_decode! {f32}
impl_decode! {f64}

impl Decode for String {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        Ok(_await!(decoder.decode_string())?)
    }
}

impl<T: Decode> Decode for Box<T> {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        Ok(Box::new(_await!(T::decode(decoder))?))
    }
}

impl<T: Default + Decode> Decode for Option<T> {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let (tag, enum_len) = _await!(decoder.decode_enum_begin("Option"))?;
        let v = match tag {
            0 => match enum_len {
                0 => Self::None,
                1 => {
                    let len = _await!(decoder.decode_struct_begin(0, None))?;
                    _await!(decoder.decode_skip(len))?;
                    _await!(decoder.decode_struct_end())?;
                    Self::None
                }
                _ => unreachable!(),
            },
            1 => match enum_len {
                0 => Self::Some(T::default()),
                1 => {
                    let len = _await!(decoder.decode_struct_begin(1, None))?;
                    _await!(decoder.decode_elem_begin(0, None))?;
                    let field_0 = if len > 0 {
                        _await!(<T as Decode>::decode(decoder))?
                    } else {
                        <T>::default()
                    };
                    _await!(decoder.decode_elem_end())?;
                    if len > 1 {
                        _await!(decoder.decode_skip(len - 1))?;
                    }
                    _await!(decoder.decode_struct_end())?;
                    Self::Some(field_0)
                }
                _ => unreachable!(),
            },
            _ => panic!("Enum Option doesn't support variant {}", tag),
        };
        _await!(decoder.decode_enum_end())?;
        Ok(v)
    }
}

impl Decode for ByteVec {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        Ok(ByteVec(_await!(decoder.decode_binary(None))?))
    }
}

impl<const N: usize> Decode for ByteArr<N> {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let value = _await!(decoder.decode_binary(Some(N)))?;
        Ok(ByteArr(value.try_into().unwrap()))
    }
}

impl<T: Decode> Decode for Vec<T> {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let len = _await!(decoder.decode_seq_begin(None))?;
        let mut value = Vec::with_capacity(len);
        for i in 0..len {
            _await!(decoder.decode_elem_begin(i, None))?;
            value.push(_await!(T::decode(decoder))?);
            _await!(decoder.decode_elem_end())?;
        }
        _await!(decoder.decode_seq_end())?;
        Ok(value)
    }
}

impl<T: Decode + Debug, const N: usize> Decode for [T; N] {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let len = _await!(decoder.decode_seq_begin(Some(N)))?;
        let mut value = Vec::with_capacity(len);
        for i in 0..len {
            _await!(decoder.decode_elem_begin(i, None))?;
            value.push(_await!(T::decode(decoder))?);
            _await!(decoder.decode_elem_end())?;
        }
        _await!(decoder.decode_seq_end())?;
        Ok(value.try_into().unwrap())
    }
}

impl Decode for () {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let len = _await!(decoder.decode_struct_begin(0, None))?;
        _await!(decoder.decode_skip(len))?;
        _await!(decoder.decode_struct_end())?;
        Ok(())
    }
}

impl<T0: Decode + Default> Decode for (T0,) {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let len = _await!(decoder.decode_struct_begin(1, None))?;
        let v0 = if len > 0 {
            _await!(decoder.decode_elem_begin(0, None))?;
            let v0 = _await!(T0::decode(decoder))?;
            _await!(decoder.decode_elem_end())?;
            v0
        } else {
            T0::default()
        };
        if len > 1 {
            _await!(decoder.decode_skip(len - 1))?;
        }
        _await!(decoder.decode_struct_end())?;
        Ok((v0,))
    }
}

impl<T0: Decode + Default, T1: Decode + Default> Decode for (T0, T1) {
    #[_async]
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        let len = _await!(decoder.decode_struct_begin(2, None))?;
        let v0 = if len > 0 {
            _await!(decoder.decode_elem_begin(0, None))?;
            let v0 = _await!(T0::decode(decoder))?;
            _await!(decoder.decode_elem_end())?;
            v0
        } else {
            T0::default()
        };
        let v1 = if len > 1 {
            _await!(decoder.decode_elem_begin(1, None))?;
            let v1 = _await!(T1::decode(decoder))?;
            _await!(decoder.decode_elem_end())?;
            v1
        } else {
            T1::default()
        };
        if len > 2 {
            _await!(decoder.decode_skip(len - 2))?;
        }
        _await!(decoder.decode_struct_end())?;
        Ok((v0, v1))
    }
}

//------Encode--------------------

pub trait Encode {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error>;
}

macro_rules! impl_encode {
    ($ty:ty) => {
        paste::item! {
            impl Encode for $ty {
                #[_async] fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
                    _await!(encoder.[<encode_ $ty>](self))?;
                    Ok(())
                }
            }
        }
    };
}

impl_encode! {bool}
impl_encode! {char}
impl_encode! {u8}
impl_encode! {u16}
impl_encode! {u32}
impl_encode! {u64}
impl_encode! {u128}
impl_encode! {i8}
impl_encode! {i16}
impl_encode! {i32}
impl_encode! {i64}
impl_encode! {i128}
impl_encode! {f32}
impl_encode! {f64}

impl Encode for String {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(encoder.encode_string(self))?;
        Ok(())
    }
}

impl<T: Encode> Encode for Option<T> {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        match self {
            Self::None => {
                _await!(encoder.encode_enum_begin(0, 0, "Option", "None"))?;
                _await!(encoder.encode_enum_end())?;
            }
            Self::Some(v) => {
                _await!(encoder.encode_enum_begin(1, 1, "Option", "Some"))?;
                _await!(encoder.encode_struct_begin(1, None))?;
                _await!(encoder.encode_elem_begin(1, None))?;
                _await!(v.encode(encoder))?;
                _await!(encoder.encode_elem_end())?;
                _await!(encoder.encode_struct_end())?;
                _await!(encoder.encode_enum_end())?;
            }
        }
        Ok(())
    }
}

impl<T: Encode> Encode for Box<T> {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(T::encode(self, encoder))?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ByteVec(pub Vec<u8>);

impl Default for ByteVec {
    fn default() -> Self {
        ByteVec(Vec::new())
    }
}

impl Encode for ByteVec {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        let ByteVec(v) = self;
        _await!(encoder.encode_binary(v))?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ByteArr<const N: usize>(pub [u8; N]);

impl<const N: usize> Default for ByteArr<N> {
    fn default() -> Self {
        ByteArr([0; N])
    }
}

impl<const N: usize> Encode for ByteArr<N> {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        let ByteArr(v) = self;
        _await!(encoder.encode_binary(v))?;
        Ok(())
    }
}

impl<T: Encode> Encode for Vec<T> {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(encoder.encode_seq_begin(self.len()))?;
        for (i, v) in self.iter().enumerate() {
            _await!(encoder.encode_elem_begin(i, None))?;
            _await!(v.encode(encoder))?;
            _await!(encoder.encode_elem_end())?;
        }
        _await!(encoder.encode_seq_end())?;
        Ok(())
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(encoder.encode_seq_begin(self.len()))?;
        for (i, v) in self.iter().enumerate() {
            _await!(encoder.encode_elem_begin(i, None))?;
            _await!(v.encode(encoder))?;
            _await!(encoder.encode_elem_end())?;
        }
        _await!(encoder.encode_seq_end())?;
        Ok(())
    }
}

impl Encode for () {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(encoder.encode_struct_begin(0, None))?;
        _await!(encoder.encode_struct_end())?;
        Ok(())
    }
}

impl<T0: Encode> Encode for (T0,) {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(encoder.encode_struct_begin(1, None))?;
        let (v0,) = self;
        _await!(encoder.encode_elem_begin(0, None))?;
        _await!(v0.encode(encoder))?;
        _await!(encoder.encode_elem_end())?;
        _await!(encoder.encode_struct_end())?;
        Ok(())
    }
}

impl<T0: Encode, T1: Encode> Encode for (T0, T1) {
    #[_async]
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(encoder.encode_struct_begin(2, None))?;
        let (v0, v1) = self;
        _await!(encoder.encode_elem_begin(0, None))?;
        _await!(v0.encode(encoder))?;
        _await!(encoder.encode_elem_end())?;
        _await!(encoder.encode_elem_begin(1, None))?;
        _await!(v1.encode(encoder))?;
        _await!(encoder.encode_elem_end())?;
        _await!(encoder.encode_struct_end())?;
        Ok(())
    }
}
