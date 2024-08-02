# `rustbif` - The Rust Binary Format

This binary serialization format is designed to encode a subset of Rust's algebraic data types, focusing on maintaining a high degree of efficiency, compactness, and backward and forward compatibility. The data structures can evolve according to these two rules:
1. Only adding new fields to the end of a struct.
2. Only adding new variants to an enum.

## TODOs
* Fix enum tags: limit to **positive** numbers **up to u32**, allow const expressions in discriminators. Negative disriminators? Consider removing support for explicit variant discriminators.
* Errors (remove unwraps and panics), figure out if need to insert `?;Ok(...)` or return directly.
* Decl macro for tuples.
* [T; N] doesn't implement Default and user can't implement it either. Default is required when "new" program reads "old" data. Need to address it.
* Implement Arr<bool, N>, Arr<char, N>, Arr<u8, N>, ..., Arr<u128, N>, Arr<i8, N>, ..., Arr<i128, N>, Arr<f32, N>, Arr<f64, N>, VArr<bool>, VArr<char>, VArr<u8>, ..., VArr<u128>, VArr<i8>, ..., VArr<i128>, VArr<f32>, VArr<f64>; and Arr<T; N> (we need this to impl Default), VArr<T> (this is the same as Vec<T>, however, it is good to have a newtype in case we need to implement foreign trait).
* Consider using Result<T, E> in your datastructures -- this is a transparent (or not?) wrapper that wont appear on the wire and that will only capture success/failure of sub-structure decoding. How to encode data that has Err()s?
* Consider using attribute (default) to control compat behaviour at the runtime: when a field is missing, return Ok(default()) OR Error(); Or try to figure out if the type impl Default or not. Explicit attribute could be better, for example, the attr can also provide value for the default.


## Wire format (element encoding)

Each data type is encoded as an element, which can be one of the following: `varint`—a data of up to 128 bits, `varbytes`—a variable-size byte sequence, `varstruct`—a variable-length sequence of elements (e.g. struct, tuple, list), or `varenum` - a tagged element.

1. **varint**: Encodes up to 128 bits of data.
   - Encodes primitive types such as booleans, chars, integers and floating point numbers, and unit enums.
   - Small integers fitting in range 0..=95 are encoded in one byte.
   - Larger integers that do not fit in range 0..=95 begin with an extra header byte `0b1110LLLL`, where `LLLL` + 1 (1..=16) indicates the number of following bytes that encode the number. Note that this is a variable size encoding, for example, `u32` integer depending on its value can be encoded using 1, 2, 3 or 4 bytes.

4. **varenum**: Encodes a tagged element (supports tags up to 32 bits)
   - Enum tags that fit in range 0..=31 are encoded in one byte.
   - Larger tags that do not fit in range 0..=31 begin with an extra header byte `0b111111MM`, where `MM` + 1 (1..=4) indicates the number of following bytes that encode the tag.
   - The tag is followed by exactly one element.

2. **varbytes**: Encodes a variable-sized byte sequence of up to 2^64 bytes.
   - An empty byte sequence is encoded as `varint` 0 (the wire format doesnt make difference if this a number 0 or en empty sequence).
   - Byte sequences of length 1..=64 are encoded as header byte `0b10LLLLLL` followed by the bytes, where `LLLLLL` + 1 is the length (1..=64) of the byte sequence.
   - Longer byte sequences use header byte `0b11110MMM`, followed by up to `MMM` + 1 (1..=8) varint bytes encoding the length of the sequence, then the sequence itself.

3. **varstruct**: Encodes a variable length sequence of elements of up to 2^32 elements.
   - An empty element sequence is encoded as `varint` 0 (the wire format doesnt make difference if this a number 0 or en empty sequence).
   - Element sequences of length 1..=32 are encoded as `0b110LLLLL` followed by `LLLLL` + 1 (1..=32) elements.
   - Longer element sequences use header byte `0b111110MM`, followed by up to `MM` + 1 (1..=4) varint bytes encoding the length of the sequence, then the elements.

## Supported Rust data types and their encodings

- **Primitive types**:
  - `u8`, `u16`, `u32`, `u64`, `u128`: Serialized to LE bytes and then encoded as `varint`
  - `i8`, `i16`, `i32`, `i64`, `i128`: Converted to unsigned int using zigzag encoding (see below) and then encoded as unsigned ints.
  - `f32`, `f64`: Serialized to BE bytes and then encoded using `varint`.
  - `bool`: Converted to `u8` (false: 0, true: 1) and then encoded as `u8`.
  - `char`: Converted to `u32` and then encoded as `u32`.

- **Opaque arrays**
  - `String`: Encoded as `varbyte`.
  - `ByteVec` (same as `VArr<u8>`): Encoded as `varbyte`.
  - `Arr<bool, N>`, `Arr<char, N>`, `Arr<u8, N>`, ..., `Arr<u128, N>`, `Arr<i8, N>`, ..., `Arr<i128, N>`, `Arr<f32, N>`, `Arr<f64, N>`, `VArr<bool>`, `VArr<char>`, `VArr<u8>`, ..., `VArr<u128>`, `VArr<i8>`, ..., `VArr<i128>`, `VArr<f32>`, `VArr<f64>`: Encoded as `varbyte`, array data is encoded using fixed-size LE order of corresponding primitive data types.

- **Generic arrays**:
  - `[T; N]`: Encoded as `varstruct`.
  - `Vec<T>`: Encoded as `varstruct`.

- **Tuples, structs and enums**:
  - Tuples `()`, `(T1,)`, `(T1, T2, ..., T32)`: Encoded as `varstruct`.
  - Tuple structs `Struct(T1, ..., TN)`: Encoded as `(T1, ..., TN)`.
  - Structs with named fields `Struct{ f1: T1, ..., fN: TN }`: Encoded as `(T1, ..., TN)`.
  - Enums with variants `Enum{ V1 = d1, ..., Vi(T1, ..., TM) = di, ..., VN = dN }`: Variant `i` is encoded as `varint` if it is a unit variant. Otherwise, it is encoded as `varenum`, i.e. enum tag followed by variant struct.

## Example

If we encode the following data structure:

```rust
(
    SampleEnum::B {
        a: 'A',
        b: SampleStruct {
            a: String::from("hello, world!"),
            b: 15,
        },
    },
    (),
)
```

Where the struct and thge enum are defined as follows:

```rust
#[derive(Debug, Default, Encode, Decode)]
struct SampleStruct {
    a: String,
    b: i32,
}

#[repr(u8)]
#[derive(Debug, Default, Encode, Decode)]
enum SampleEnum {
    #[default]
    None,
    A(String) = 10,
    B {
        a: char,
        b: SampleStruct,
    } = 20,
}
```

We will produce the following binary code:

```
c1 74 c1 41 c1 8c 68 65 6c 6c 6f 2c 20 77 6f 72 6c 64 21 1e 00
```

Where:

```
0xc1 0b11000001 - varstruct tuple, its 2 elements follow
    0x74 0b01110100 - varenum with tag 20, its 1 element follow
        0xc1 0b11000001 - varstruct struct "SampleEnum::B", its 2 fields follow
            0x41 - field a: varint encoded char 'A'
            0xc1 0b11000001 - field b: varstruct struct "SampleStruct", its 2 fields follow
                0x8c 0b10001100 - field a: varbyte string, its 13 bytes follow
                    0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x21 - "hello, world!"
                0x1e - zigzag encoded integer 15
    0x00 - empty varstruct encoded as 0
```

## Zigzag encoding

Zigzag encoding takes a signed integer and encodes it as an unsigned
integer. It does so by counting up, starting at zero, alternating
between representing a positive number and a negative number.

To encode any signed integer, `x`, with a representation length—in bits,
`n`, the formula is as follows:

```
(x >> n - 1) ^ (x << 1)
```

*Note*: The `^` operator is bitwise XOR.

To convert a zigzag-encoded unsigned integer, `x`, to its decoded signed
counterpart, the formula is as follows:

```
(x >>> 1) ^ -(x & 1)
```

*Note*: The `>>>` operator represents a logical right shift as opposed
to an arithmetic right shift. In Rust, unsigned integer types implement the
right shift operator as logical instead of arithmetic. Therefore, the
formula in Rust is simplified as `(x >> 1) ^ -(x & 1)`.

## Handling data structure evolution and compatibility

This encoding format is not fully self-describing, meaning the original data structure cannot be reconstructed solely from the binary format. However, it includes enough information to support automatic backward and forward compatibility under certain constraints:
- New struct fields are appended only at the end.
- New enum variants are only added.

When a new program reads old data, it assigns default values to any new struct fields. Conversely, an old program reading new data will skip unknown struct fields and signal an error if it encounters an unsupported new enum variant. This ensures a high degree of robustness as data structures evolve over time and eliminates the need for tedious management of data structure versioning.
