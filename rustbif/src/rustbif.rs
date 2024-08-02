#![allow(async_fn_in_trait)]

use build_async::*;
use std::convert::TryInto;
use std::fmt::Debug;
use zigzag::ZigZag;

//------ varintlen -------

const M_VALUE: u8 = 0b00_000000; // 0x00
const M_BYTES: u8 = 0b10_000000; // 0x80
const M_STRUCT: u8 = 0b110_00000; // 0xc0
const M_VALUE_LEN: u8 = 0b1110_0000; // 0xe0
const M_BYTES_LEN: u8 = 0b11110_000; // 0xf0
const M_STRUCT_LEN: u8 = 0b111110_00; // 0xf8
const M_ENUM_LEN: u8 = 0b111111_00; // 0xfC

#[derive(Debug, Clone, Copy, PartialEq)]
enum VarIntLen {
    Zero,               // Value(0), UnitEnumTag(0), ByteSize(0), or StructLen(0) followed by nothing
    Value([u8; 16]), // Value(value = 1..=95, 0..2^128) or UnitEnumTag(tag = 1..=95, 0..2^128) followed by nothing
    ByteSize([u8; 8]), // ByteSize(size = 1..=64, 0..2^64) followed by `size` bytes of data
    StructLen([u8; 4]), // StructLen(len = 1..=32, 0..2^32) followed by `len` elements
    EnumTag([u8; 4]), // EnumTag(tag = 0..=31 (96..=127), 0..2^32) followed by 1 element
}

impl VarIntLen {
    fn new() -> Self {
        Self::Zero
    }
    fn from_value_f32(value: f32) -> Self {
        Self::from_value_slice(&value.to_le_bytes())
    }
    fn from_value_f64(value: f64) -> Self {
        Self::from_value_slice(&value.to_le_bytes())
    }
    fn from_value_u32(value: u32) -> Self {
        Self::from_value_slice(&value.to_le_bytes())
    }
    fn from_value_u128(value: u128) -> Self {
        Self::from_value_slice(&value.to_le_bytes())
    }
    fn from_value_slice(bytes: &[u8]) -> Self {
        if bytes.iter().rposition(|x| *x != 0) == None {
            Self::Zero
        } else {
            let mut buf = [0; 16];
            buf[..bytes.len()].copy_from_slice(bytes);
            Self::Value(buf)
        }
    }
    fn from_byte_size(size: u64) -> Self {
        Self::from_byte_size_slice(&size.to_le_bytes())
    }
    fn from_byte_size_slice(bytes: &[u8]) -> Self {
        if bytes.iter().rposition(|x| *x != 0) == None {
            Self::Zero
        } else {
            let mut buf = [0; 8];
            buf[..bytes.len()].copy_from_slice(bytes);
            Self::ByteSize(buf)
        }
    }
    fn from_struct_len(len: u32) -> Self {
        Self::from_struct_len_slice(&len.to_le_bytes())
    }
    fn from_struct_len_slice(bytes: &[u8]) -> Self {
        if bytes.iter().rposition(|x| *x != 0) == None {
            Self::Zero
        } else {
            let mut buf = [0; 4];
            buf[..bytes.len()].copy_from_slice(bytes);
            Self::StructLen(buf)
        }
    }
    fn from_enum_tag(tag: u32) -> Self {
        Self::from_enum_tag_slice(&tag.to_le_bytes())
    }
    fn from_enum_tag_slice(bytes: &[u8]) -> Self {
        let mut buf = [0; 4];
        buf[..bytes.len()].copy_from_slice(bytes);
        Self::EnumTag(buf)
    }

    #[_async]
    fn from_reader<R: Reader>(reader: &mut R) -> Result<(Self, usize), R::Error> {
        let mut cnt: usize = 0;
        let mut buf = [0_u8; 16];
        cnt += _await!(reader.read(&mut buf[..1]))?;
        let header = buf[0];
        match header.leading_ones() {
            0 => Ok((
                match header {
                    0 => Self::Zero,
                    1..=95 => Self::Value(buf),
                    96..=127 => {
                        buf[0] -= 96;
                        Self::EnumTag(buf[..4].try_into().unwrap())
                    }
                    128..=255 => unreachable!(),
                },
                cnt,
            )),
            1 => {
                buf[0] &= 0b00111111;
                buf[0] += 1;
                Ok((Self::ByteSize(buf[..8].try_into().unwrap()), cnt))
            }
            2 => {
                buf[0] &= 0b00011111;
                buf[0] += 1;
                Ok((Self::StructLen(buf[..4].try_into().unwrap()), cnt))
            }
            3 => {
                let mut buf = [0_u8; 16];
                let len: usize = ((header & 0b00001111) + 1).into();
                cnt += _await!(reader.read(&mut buf[..len]))?;
                Ok((Self::Value(buf), cnt))
            }
            4 => {
                let mut buf = [0_u8; 8];
                let len: usize = ((header & 0b00000111) + 1).into();
                cnt += _await!(reader.read(&mut buf[..len]))?;
                Ok((Self::ByteSize(buf), cnt))
            }
            5 => {
                let mut buf = [0_u8; 4];
                let len: usize = ((header & 0b00000011) + 1).into();
                cnt += _await!(reader.read(&mut buf[..len]))?;
                Ok((Self::StructLen(buf), cnt))
            }
            _ => {
                let mut buf = [0_u8; 4];
                let len: usize = ((header & 0b00000011) + 1).into();
                cnt += _await!(reader.read(&mut buf[..len]))?;
                Ok((Self::EnumTag(buf), cnt))
            }
        }
    }

    #[_async]
    fn write<W: Writer>(&self, writer: &mut W) -> Result<usize, W::Error> {
        let (mask1, mask2, bytes, corr_sub, corr_add, threshold) = match self {
            Self::Zero => (M_VALUE, M_VALUE_LEN, &[0][..], 0, 0, 0),
            Self::Value(buf) => (M_VALUE, M_VALUE_LEN, &buf[..], 0, 0, 95),
            Self::EnumTag(buf) => (M_VALUE, M_ENUM_LEN, &buf[..], 0, 96, 127),
            Self::ByteSize(buf) => (M_BYTES, M_BYTES_LEN, &buf[..], 1, 0, 64),
            Self::StructLen(buf) => (M_STRUCT, M_STRUCT_LEN, &buf[..], 1, 0, 32),
        };
        let mut cnt: usize = 0;
        let n = bytes.iter().rposition(|x| *x != 0).or(Some(0)).unwrap() + 1;
        let v = bytes[0] + corr_add;
        match n {
            1 if v == 0 => {
                cnt += _await!(writer.write(&[0]))?;
            }
            1 if v <= threshold => {
                cnt += _await!(writer.write(&[mask1 | (v - corr_sub)]))?;
            }
            _ => {
                let v: u8 = n.try_into().unwrap();
                cnt += _await!(writer.write(&[mask2 | (v - 1)]))?;
                cnt += _await!(writer.write(&bytes[..n]))?;
            }
        }
        Ok(cnt)
    }
}

//---------Reader/Writer----------------

pub trait Reader {
    type Error;
    #[_async]
    fn read(&mut self, _bytes: &mut [u8]) -> Result<usize, Self::Error> {
        unimplemented!();
    }
}

pub trait Writer {
    type Error;
    #[_async]
    fn write(&mut self, _bytes: &[u8]) -> Result<usize, Self::Error> {
        unimplemented!();
    }
}

impl<T: std::io::Read> Reader for std::io::BufReader<T> {
    type Error = std::io::Error;
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error> {
        let n = std::io::Read::read(self, bytes)?;
        Ok(n)
    }
}

impl<T: std::io::Write> Writer for std::io::BufWriter<T> {
    type Error = std::io::Error;
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        let n = std::io::Write::write(self, bytes)?;
        Ok(n)
    }
}

impl Reader for Vec<u8> {
    type Error = ();
    #[_async]
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error> {
        bytes.copy_from_slice(&self[..bytes.len()]);
        self.drain(..bytes.len());
        Ok(bytes.len())
    }
}

impl Writer for Vec<u8> {
    type Error = ();
    #[_async]
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.extend_from_slice(bytes);
        Ok(bytes.len())
    }
}

//-------Decoder----------------------

macro_rules! fn_decode_uint {
    ($ty:ty, $order: ident) => {
        paste::item! {
            #[_async] fn [<decode_ $ty>](&mut self) -> Result<$ty, Self::Error> {
                let mut buf = [0_u8; std::mem::size_of::<$ty>()];
                _await!(self.decode_uint(&mut buf))?;
                Ok($ty::[<from_ $order _bytes>](buf))
            }
        }
    };
}
macro_rules! fn_decode_int {
    ($ty:ty, $uty:ty, $order: ident) => {
        paste::item! {
            #[_async] fn [<decode_ $ty>](&mut self) -> Result<$ty, Self::Error> {
                let mut buf = [0_u8; std::mem::size_of::<$ty>()];
                _await!(self.decode_uint(&mut buf))?;
                Ok(ZigZag::decode($uty::[<from_ $order _bytes>](buf)))
            }
        }
    };
}

macro_rules! fn_decode_vec {
    ($ty:ty, $closure: expr) => {
        paste::item! {
            #[_async] fn [<decode_vec_ $ty>](&mut self, len: Option<usize>) -> Result<Vec<$ty>, Self::Error> {
                let size = _await!(self.decode_bytes_begin(len.map(|x| x * std::mem::size_of::<$ty>())))?;
                let len = size / std::mem::size_of::<$ty>();
                if size != len * std::mem::size_of::<$ty>() {
                    panic!("byte array size {} is not a multiple of element size {}", size, std::mem::size_of::<$ty>());
                }
                let mut v = Vec::with_capacity(len);
                let mut buf = [0_u8; std::mem::size_of::<$ty>()];
                for _i in 0..len {
                    _await!(self.decode_bytes_payload(&mut buf))?;
                    v.push(($closure)(buf));
                }
                _await!(self.decode_bytes_end())?;
                Ok(v)
            }
        }
    };
}

pub struct Decoder<R: Reader> {
    pub reader: R,
}

impl<R: Reader> cerdito::Decoder for Decoder<R> {
    type Error = R::Error;

    #[_async]
    fn decode_bool(&mut self) -> Result<bool, Self::Error> {
        Ok(if _await!(self.decode_u8())? != 0 {
            true
        } else {
            false
        })
    }
    #[_async]
    fn decode_char(&mut self) -> Result<char, Self::Error> {
        Ok(char::from_u32(_await!(self.decode_u32())?).unwrap())
    }
    fn_decode_uint! {u8, le}
    fn_decode_uint! {u16, le}
    fn_decode_uint! {u32, le}
    fn_decode_uint! {u64, le}
    fn_decode_uint! {u128, le}
    fn_decode_int! {i8, u8, le}
    fn_decode_int! {i16, u16, le}
    fn_decode_int! {i32, u32, le}
    fn_decode_int! {i64, u64, le}
    fn_decode_int! {i128, u128, le}
    fn_decode_uint! {f32, be}
    fn_decode_uint! {f64, be}

    #[_async]
    fn decode_string(&mut self) -> Result<String, Self::Error> {
        Ok(String::from_utf8(_await!(self.decode_binary(None))?).unwrap())
    }
    #[_async]
    fn decode_binary(&mut self, size: Option<usize>) -> Result<Vec<u8>, Self::Error> {
        let size = _await!(self.decode_bytes_begin(size))?;
        let mut buf = vec![0_u8; size];
        _await!(self.decode_bytes_payload(&mut buf))?;
        _await!(self.decode_bytes_end())?;
        Ok(buf)
    }

    #[_async]
    fn decode_vec_u8(&mut self, len: Option<usize>) -> Result<Vec<u8>, Self::Error> {
        _await!(self.decode_binary(len))
    }
    fn_decode_vec! {bool, |buf| if u8::from_le_bytes(buf) != 0 {true} else {false}}
    fn_decode_vec! {char, |buf| char::from_u32(u32::from_le_bytes(buf)).unwrap()}
    fn_decode_vec! {u16, |buf| u16::from_le_bytes(buf)}
    fn_decode_vec! {u32, |buf| u32::from_le_bytes(buf)}
    fn_decode_vec! {u64, |buf| u64::from_le_bytes(buf)}
    fn_decode_vec! {u128, |buf| u128::from_le_bytes(buf)}
    fn_decode_vec! {i8, |buf| i8::from_le_bytes(buf)}
    fn_decode_vec! {i16, |buf| i16::from_le_bytes(buf)}
    fn_decode_vec! {i32, |buf| i32::from_le_bytes(buf)}
    fn_decode_vec! {i64, |buf| i64::from_le_bytes(buf)}
    fn_decode_vec! {i128, |buf| i128::from_le_bytes(buf)}
    fn_decode_vec! {f32, |buf| f32::from_le_bytes(buf)}
    fn_decode_vec! {f64, |buf| f64::from_le_bytes(buf)}

    #[_async]
    fn decode_seq_begin(&mut self, _len: Option<usize>) -> Result<usize, Self::Error> {
        let (v, _n) = _await!(VarIntLen::from_reader(&mut self.reader))?;
        match v {
            VarIntLen::StructLen(buf) => Ok(u32::from_le_bytes(buf).try_into().unwrap()),
            VarIntLen::Zero => Ok(0),
            _ => panic!("bad seq header"),
        }
    }
    #[_async]
    fn decode_seq_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn decode_enum_begin(&mut self, _enum_name: &str) -> Result<(u32, usize), Self::Error> {
        let (v, _n) = _await!(VarIntLen::from_reader(&mut self.reader))?;
        match v {
            VarIntLen::EnumTag(buf) => Ok((u32::from_le_bytes(buf), 1)),
            VarIntLen::Value(buf) => Ok((u32::from_le_bytes(buf[..4].try_into().unwrap()), 0)),
            VarIntLen::Zero => Ok((0, 0)),
            _ => panic!("bad varenum header"),
        }
    }
    #[_async]
    fn decode_enum_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn decode_struct_begin(
        &mut self,
        _len: usize,
        _struct_name: Option<&str>,
    ) -> Result<usize, Self::Error> {
        let (v, _n) = _await!(VarIntLen::from_reader(&mut self.reader))?;
        match v {
            VarIntLen::StructLen(buf) => Ok(u32::from_le_bytes(buf).try_into().unwrap()),
            VarIntLen::Zero => Ok(0),
            _ => panic!("bad varstruct header"),
        }
    }
    #[_async]
    fn decode_struct_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn decode_bytes_begin(&mut self, _size: Option<usize>) -> Result<usize, Self::Error> {
        let (v, _n) = _await!(VarIntLen::from_reader(&mut self.reader))?;
        match v {
            VarIntLen::ByteSize(buf) => Ok(u64::from_le_bytes(buf).try_into().unwrap()),
            VarIntLen::Zero => Ok(0),
            _ => panic!("bad varbyte header"),
        }
    }
    #[_async]
    fn decode_bytes_payload(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        _await!(self.reader.read(buf))
    }
    #[_async]
    fn decode_bytes_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn decode_uint(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error> {
        let (v, _n) = _await!(VarIntLen::from_reader(&mut self.reader))?;
        match v {
            VarIntLen::Zero => {
                bytes.fill(0);
                Ok(bytes.len())
            }
            VarIntLen::Value(buf) => {
                bytes.copy_from_slice(&buf[..bytes.len()]);
                Ok(bytes.len())
            }
            _ => panic!("bad varint header: {:?}", v),
        }
    }

    #[_async]
    fn decode_skip(&mut self, n: usize) -> Result<(), Self::Error> {
        let mut counter = n;

        while counter != 0 {
            counter -= 1;
            let (v, _n) = _await!(VarIntLen::from_reader(&mut self.reader))?;
            match v {
                VarIntLen::ByteSize(buf) => {
                    let size = u64::from_le_bytes(buf).try_into().unwrap();
                    let mut buf = vec![0_u8; size];
                    _await!(self.decode_bytes_payload(&mut buf))?;
                }
                VarIntLen::StructLen(buf) => {
                    let len: usize = u32::from_le_bytes(buf).try_into().unwrap();
                    counter += len;
                }
                VarIntLen::EnumTag(_buf) => {
                    counter += 1;
                }
                VarIntLen::Value(_) | VarIntLen::Zero => {}
            }
        }
        Ok(())
    }
}

//--------Encoder----------------

macro_rules! fn_encode_vec {
    ($ty:ty) => {
        paste::item! {
            #[_async] fn [<encode_vec_ $ty>](&mut self, values: &[$ty]) -> Result<(), Self::Error> {
                _await!(self.encode_bytes_begin(values.len() * std::mem::size_of::<$ty>()))?;
                for value in values {
                    _await!(self.encode_bytes_payload(&value.to_le_bytes()))?;
                }
                _await!(self.encode_bytes_end())
            }
        }
    };
}
macro_rules! fn_encode_uint {
    ($ty:ty, $order: ident) => {
        paste::item! {
            #[_async] fn [<encode_ $ty>](&mut self, value: &$ty) -> Result<(), Self::Error> {
                _await!(self.encode_uint(&value.[<to_ $order _bytes>]()))
            }
        }
    };
}
macro_rules! fn_encode_int {
    ($ty:ty, $order: ident) => {
        paste::item! {
            #[_async] fn [<encode_ $ty>](&mut self, value: &$ty) -> Result<(), Self::Error> {
                _await!(self.encode_uint(&ZigZag::encode(*value).[<to_ $order _bytes>]()))
            }
        }
    };
}

pub struct Encoder<W: Writer> {
    pub writer: W,
}

impl<W: Writer> cerdito::Encoder for Encoder<W> {
    type Error = W::Error;

    #[_async]
    fn encode_bool(&mut self, value: &bool) -> Result<(), Self::Error> {
        _await!(self.encode_u8(&(*value).into()))
    }
    #[_async]
    fn encode_char(&mut self, value: &char) -> Result<(), Self::Error> {
        _await!(self.encode_u32(&(*value).into()))
    }
    #[_async]
    fn encode_u8(&mut self, value: &u8) -> Result<(), Self::Error> {
        _await!(self.encode_uint(&[*value]))
    }
    #[_async]
    fn encode_i8(&mut self, value: &i8) -> Result<(), Self::Error> {
        _await!(self.encode_uint(&[ZigZag::encode(*value)]))
    }
    fn_encode_uint! {u16, le}
    fn_encode_uint! {u32, le}
    fn_encode_uint! {u64, le}
    fn_encode_uint! {u128, le}
    fn_encode_int! {i16, le}
    fn_encode_int! {i32, le}
    fn_encode_int! {i64, le}
    fn_encode_int! {i128, le}
    fn_encode_uint! {f32, be}
    fn_encode_uint! {f64, be}

    #[_async]
    fn encode_binary(&mut self, value: &[u8]) -> Result<(), Self::Error> {
        _await!(self.encode_bytes_begin(value.len()))?;
        _await!(self.encode_bytes_payload(value))?;
        _await!(self.encode_bytes_end())
    }

    #[_async]
    fn encode_string(&mut self, value: &str) -> Result<(), Self::Error> {
        _await!(self.encode_binary(value.as_bytes()))
    }

    #[_async]
    fn encode_vec_bool(&mut self, values: &[bool]) -> Result<(), Self::Error> {
        _await!(self.encode_bytes_begin(values.len() * std::mem::size_of::<bool>()))?;
        for value in values {
            _await!(self.encode_bytes_payload(&[(*value).into()]))?;
        }
        _await!(self.encode_bytes_end())
    }
    #[_async]
    fn encode_vec_char(&mut self, values: &[char]) -> Result<(), Self::Error> {
        _await!(self.encode_bytes_begin(values.len() * std::mem::size_of::<char>()))?;
        for value in values {
            let value: u32 = (*value).into();
            _await!(self.encode_bytes_payload(&value.to_le_bytes()))?;
        }
        _await!(self.encode_bytes_end())
    }
    #[_async]
    fn encode_vec_u8(&mut self, values: &[u8]) -> Result<(), Self::Error> {
        _await!(self.encode_binary(values))
    }
    fn_encode_vec! {u16}
    fn_encode_vec! {u32}
    fn_encode_vec! {u64}
    fn_encode_vec! {u128}
    fn_encode_vec! {i8}
    fn_encode_vec! {i16}
    fn_encode_vec! {i32}
    fn_encode_vec! {i64}
    fn_encode_vec! {i128}
    fn_encode_vec! {f32}
    fn_encode_vec! {f64}

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

    #[_async]
    fn encode_seq_begin(&mut self, len: usize) -> Result<(), Self::Error> {
        let v = VarIntLen::from_struct_len(len.try_into().unwrap());
        _await!(v.write(&mut self.writer))?;
        Ok(())
    }
    #[_async]
    fn encode_seq_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn encode_enum_begin(
        &mut self,
        enum_tag: u32,
        len: usize,
        _enum_name: &str,
        _variant_name: &str,
    ) -> Result<(), Self::Error> {
        let v = match len {
            0 => VarIntLen::from_value_u32(enum_tag),
            1 => VarIntLen::from_enum_tag(enum_tag),
            _ => unreachable!(),
        };
        _await!(v.write(&mut self.writer))?;
        Ok(())
    }
    #[_async]
    fn encode_enum_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn encode_struct_begin(
        &mut self,
        len: usize,
        _struct_name: Option<&str>,
    ) -> Result<(), Self::Error> {
        let v = VarIntLen::from_struct_len(len.try_into().unwrap());
        _await!(v.write(&mut self.writer))?;
        Ok(())
    }
    #[_async]
    fn encode_struct_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn encode_bytes_begin(&mut self, size: usize) -> Result<(), Self::Error> {
        let v = VarIntLen::from_byte_size(size.try_into().unwrap());
        _await!(v.write(&mut self.writer))?;
        Ok(())
    }
    #[_async]
    fn encode_bytes_payload(&mut self, value: &[u8]) -> Result<(), Self::Error> {
        _await!(self.writer.write(value))?;
        Ok(())
    }
    #[_async]
    fn encode_bytes_end(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[_async]
    fn encode_uint(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        let v = VarIntLen::from_value_slice(bytes);
        _await!(v.write(&mut self.writer))?;
        Ok(())
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varintlen_write_read(v: VarIntLen) -> VarIntLen {
        let mut rw: Vec<u8> = Vec::new();
        v.write(&mut rw).unwrap();
        let (v2, _n) = VarIntLen::from_reader(&mut rw).unwrap();
        v2
    }

    #[test]
    fn test_varintlen_write_values() {
        let mut w: Vec<u8> = Vec::new();
        VarIntLen::new().write(&mut w).unwrap();
        VarIntLen::from_value_slice(&[63_u8][..])
            .write(&mut w)
            .unwrap();
        VarIntLen::from_value_slice(&[127_u8, 0, 0, 0][..])
            .write(&mut w)
            .unwrap();
        VarIntLen::from_value_slice(&[128_u8, 0, 0][..])
            .write(&mut w)
            .unwrap();
        VarIntLen::from_value_slice(&[255_u8, 0][..])
            .write(&mut w)
            .unwrap();
        assert_eq!(
            w,
            vec![
                M_VALUE | 0_u8,
                M_VALUE | 63_u8,
                M_VALUE_LEN | 0_u8,
                127_u8,
                M_VALUE_LEN | 0_u8,
                128,
                M_VALUE_LEN | 0_u8,
                255,
            ]
        );
    }

    #[test]
    fn test_varintlen_write_byte_sizes() {
        let mut w: Vec<u8> = Vec::new();
        VarIntLen::from_byte_size(0).write(&mut w).unwrap();
        VarIntLen::from_byte_size(1).write(&mut w).unwrap();
        VarIntLen::from_byte_size(7).write(&mut w).unwrap();
        VarIntLen::from_byte_size(63).write(&mut w).unwrap();
        VarIntLen::from_byte_size(64).write(&mut w).unwrap();
        VarIntLen::from_byte_size(65).write(&mut w).unwrap();
        VarIntLen::from_byte_size(0xffffff).write(&mut w).unwrap();
        VarIntLen::from_byte_size(0xffffffff_ffffffff)
            .write(&mut w)
            .unwrap();
        assert_eq!(
            w,
            vec![
                M_VALUE | 0_u8,
                M_BYTES | 0_u8,
                M_BYTES | 6_u8,
                M_BYTES | 62_u8,
                M_BYTES | 63_u8,
                M_BYTES_LEN | 0_u8,
                65,
                M_BYTES_LEN | 2_u8,
                0xff,
                0xff,
                0xff,
                M_BYTES_LEN | 7_u8,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
            ]
        );
    }

    #[test]
    fn test_varintlen_write_struct_lens() {
        let mut w: Vec<u8> = Vec::new();
        VarIntLen::from_struct_len(0).write(&mut w).unwrap();
        VarIntLen::from_struct_len(1).write(&mut w).unwrap();
        VarIntLen::from_struct_len(7).write(&mut w).unwrap();
        VarIntLen::from_struct_len(31).write(&mut w).unwrap();
        VarIntLen::from_struct_len(32).write(&mut w).unwrap();
        VarIntLen::from_struct_len(33).write(&mut w).unwrap();
        VarIntLen::from_struct_len(0xffff_ffff)
            .write(&mut w)
            .unwrap();
        VarIntLen::from_struct_len(0x01020304)
            .write(&mut w)
            .unwrap();
        assert_eq!(
            w,
            vec![
                M_VALUE | 0_u8,
                M_STRUCT | 0_u8,
                M_STRUCT | 6_u8,
                M_STRUCT | 30_u8,
                M_STRUCT | 31_u8,
                M_STRUCT_LEN | 0_u8,
                33,
                M_STRUCT_LEN | 3_u8,
                0xff,
                0xff,
                0xff,
                0xff,
                M_STRUCT_LEN | 3_u8,
                0x04,
                0x03,
                0x02,
                0x01,
            ]
        );
    }

    #[test]
    fn test_varintlen_read() {
        let mut r: Vec<u8> = vec![
            M_VALUE | 0_u8,
            M_STRUCT | 0_u8,
            M_STRUCT | 6_u8,
            M_STRUCT | 30_u8,
            M_STRUCT | 31_u8,
            M_STRUCT_LEN | 0_u8,
            33,
            M_STRUCT_LEN | 3_u8,
            0xff,
            0xff,
            0xff,
            0xff,
            M_STRUCT_LEN | 3_u8,
            0x08,
            0x07,
            0x06,
            0x05,
            M_BYTES | 0_u8,
            M_BYTES | 6_u8,
            M_BYTES | 62_u8,
            M_BYTES | 63_u8,
            M_BYTES_LEN | 0_u8,
            65,
            M_BYTES_LEN | 2_u8,
            0xff,
            0xff,
            0xff,
            M_BYTES_LEN | 7_u8,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            0xff,
            M_VALUE | 1_u8,
            M_VALUE | 63_u8,
            M_VALUE_LEN | 0_u8,
            64,
            M_VALUE_LEN | 0_u8,
            255,
            M_VALUE_LEN | 1_u8,
            0,
            1,
            M_VALUE_LEN | 0_u8,
            127,
            0_u8 + 96,
            1_u8 + 96,
            31_u8 + 96,
            M_ENUM_LEN | 0_u8,
            32,
            M_ENUM_LEN | 0_u8,
            62,
            M_ENUM_LEN | 0_u8,
            63,
            M_ENUM_LEN | 0_u8,
            64,
            M_ENUM_LEN | 0_u8,
            65,
            M_ENUM_LEN | 0_u8,
            127,
            M_ENUM_LEN | 0_u8,
            128,
            M_ENUM_LEN | 3_u8,
            0xff,
            0xff,
            0xff,
            0xff,
            0x00,
        ];

        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::Zero);
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(1));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(7));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(31));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(32));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(33));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(0xffff_ffff));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_struct_len(0x05060708));

        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(1));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(7));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(63));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(64));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(65));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(0xffffff));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_byte_size(0xffffffff_ffffffff));

        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_value_slice(&[1_u8][..]));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_value_slice(&[63_u8, 0][..]));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_value_slice(&[64_u8, 0, 0, 0, 0][..]));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_value_slice(&[255_u8, 0, 0, 0, 0][..]));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_value_slice(&[0_u8, 1, 0, 0, 0][..]));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_value_slice(&[127_u8, 0][..]));

        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(0));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(1));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(31));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(32));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(62));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(63));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(64));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(65));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(127));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(128));
        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::from_enum_tag(0xffff_ffff));

        let (v, _n) = VarIntLen::from_reader(&mut r).unwrap();
        assert_eq!(v, VarIntLen::Zero);
    }
}
