use build_async::*;
use cerdito::{ByteArr, ByteVec, Decode, Encode};
use multibase::Base;
use std::fmt::Debug;

//-------Samples-----------------------
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

//-----More samples------------
const M: usize = 100;

#[derive(Debug, Default, Encode, Decode)]
struct S1<
    T: Debug + Default + Encode + Decode,
    U: Debug + Default + Encode + Decode,
    const N: usize,
> {
    aaa: std::option::Option<std::boxed::Box<String>>,
    //bbb: [Box<S1<T, U, N>>; M], //TODO: Implement Arr<T; N> types and Default for them
    bbb: Vec<Box<S1<T, U, N>>>,
    ccc: Vec<Option<U>>,
    ddd: Vec<T>,
    eee: (i32, String),
}

#[derive(Debug, Default, Encode, Decode)]
struct S2;

#[derive(Debug, Default, Encode, Decode)]
struct S3(i32, String);

#[repr(usize)]
#[derive(Debug, Default, Encode, Decode)]
enum E1<T: Debug + Default + Encode + Decode, U: Debug + Default + Encode + Decode> {
    #[default]
    A = 0x12,
    A1 = 0x13,
    B(T) = M,
    B1(U) = 0x14,
    C {
        a: (),
        b: Box<E1<T, U>>,
    } = 0x11,
    C1 = 0x01,
}

//-----Metadata Framework--------
#[derive(Debug, Default, Encode, Decode)]
enum DataType {
    #[default]
    None,
    Bool,
    Char,
    F32,
    F64,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    String,
    Array(Box<DataType>, Option<u64>),  // Array and Vec
    Struct(Option<String>, Vec<Field>), // Struct, Term, and Tuple
    Enum(String, Vec<Variant>),
    StructName(String), // Named struct defined somewhere else
    EnumName(String),   // Enum defined somewhere else
}
#[derive(Debug, Default, Encode, Decode)]
struct Field(Option<String>, Box<DataType>);
#[derive(Debug, Default, Encode, Decode)]
struct Variant(String, Option<Discriminant>, Box<DataType>);
#[derive(Debug, Default, Encode, Decode)]
enum Discriminant {
    #[default]
    None,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
}

//-----CAS and Linked Data-------
#[repr(u8)]
#[derive(Debug, Default, Clone, Encode, Decode)]
enum Hash {
    #[default]
    None,
    Sha2x256(ByteArr<32>) = 0x12,
}

#[repr(u8)]
#[derive(Debug, Default, Clone, Encode, Decode)]
enum ContentID {
    #[default]
    None,
    File(Hash),
    Link(Hash),
}

#[repr(u8)]
#[derive(Debug, Clone)]
enum EncryptionKey {
    None = 0,
    Aes256CtrIvA(ByteArr<32>) = 1,
}

#[derive(Debug, Clone)]
struct FileName(String); // string w/o '/'

impl Encode for FileName {
    #[_async]
    fn encode<E: cerdito::Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        _await!(self.0.encode(encoder))
    }
}

impl Decode for FileName {
    #[_async]
    fn decode<D: cerdito::Decoder>(decoder: &mut D) -> Result<Self, D::Error> {
        Ok(FileName(_await!(String::decode(decoder))?))
    }
}

#[repr(u8)]
#[derive(Debug, Clone)]
enum DirectoryEntry {
    Regular {
        name: FileName,
        content_id: ContentID,
        encryption_key: EncryptionKey,
    } = 0,
    Directory {
        name: FileName,
        content_id: ContentID,
        encryption_key: EncryptionKey,
    } = 1,
}

#[derive(Debug, Clone)]
struct PublicKey;

#[derive(Debug, Clone)]
struct Signature;

#[derive(Debug, Clone)]
struct Link {
    // to verify the link make sure: 1) link CID = hash(public_key) , 2) verify signature with the public_key
    content_id: ContentID,
    sequence_number: u32,  // to prevent replay attack
    time_to_live: u32,     // in seconds, to hint how often this link will be updated
    public_key: PublicKey, // the public key of the owner; link cid = hash(public_key)
    signature: Signature, // the signature of (directory_entry, sequence_number, time_to_live) that can be verified with the public key
}

//-------main------------
fn main() -> Result<(), ()> {
    let mut encoder = rustbif::Encoder { writer: Vec::new() };

    println!("----------------------Encoding/Decoding: recursive enum E1, tuple (SampleEnum:B, (String,)),  SampleStruct Дима, 1024 and 1_u32");

    type E1U8I8 = E1<u8, i8>;
    E1U8I8::C {
        a: (),
        b: Box::new(E1U8I8::C {
            a: (),
            b: Box::new(E1U8I8::C {
                a: (),
                b: Box::new(E1U8I8::A),
            }),
        }),
    }
    .encode(&mut encoder)
    .unwrap();

    (
        SampleEnum::B {
            a: 'A',
            b: SampleStruct {
                a: "hello, world!".to_string(),
                b: 15,
            },
        },
        (String::from("hello"),),
    )
        .encode(&mut encoder)
        .unwrap();

    SampleStruct {
        a: "Дима".to_string(),
        b: 1024,
    }
    .encode(&mut encoder)
    .unwrap();

    1_u32.encode(&mut encoder).unwrap();

    println!("{:02x?}", encoder.writer);

    let mut decoder = rustbif::Decoder {
        reader: encoder.writer,
    };
    println!("decoding enum E1");
    let v = E1U8I8::decode(&mut decoder);
    dbg!(&v);

    println!("decoding tuple w enum");
    let v = <(SampleEnum, (String,))>::decode(&mut decoder);
    dbg!(&v);

    println!("decoding struct");
    let v = SampleStruct::decode(&mut decoder);
    dbg!(&v);

    println!("decoding u32");
    let v = u32::decode(&mut decoder);
    dbg!(&v);

    let mut encoder = rustbif::Encoder { writer: Vec::new() };

    println!("----------------------Encoding: String Игорь");
    String::from("Игорь").encode(&mut encoder).unwrap();

    let x: Vec<u8> = vec![1, 2, 3];
    println!("----------------------Encoding: Vec<u8> = vec![1, 2, 3]");
    x.encode(&mut encoder).unwrap();
    println!("----------------------Encoding: ByteVec(Vec<u8> = vec![1, 2, 3])");
    ByteVec(x).encode(&mut encoder).unwrap();

    println!("----------------------Encoding: ByteArr([10_u8, 20_u8])");
    ByteArr([10_u8, 20_u8]).encode(&mut encoder).unwrap();

    println!("----------------------Encoding: [aaa.to_string(), bbb.to_string()]");
    ["aaa".to_string(), "bbb".to_string()]
        .encode(&mut encoder)
        .unwrap();

    println!("----------------------Encoding: (123e5_f64, String::from(uuu))");
    (123e5_f64, String::from("uuu"))
        .encode(&mut encoder)
        .unwrap();

    println!("----------------------Encoding: (SampleEnum B a b, ()) and ()");
    let mut encoder = rustbif::Encoder { writer: Vec::new() };
    (
        SampleEnum::B {
            a: 'A',
            b: SampleStruct {
                a: "hello, world!".to_string(),
                b: 15,
            },
        },
        (),
    )
        .encode(&mut encoder)
        .unwrap();
    println!("Example: {:02x?}", encoder.writer);

    ().encode(&mut encoder).unwrap();

    println!("----------------------Encoding: SampleStruct a b");
    SampleStruct {
        a: "sss".to_string(),
        b: 15,
    }
    .encode(&mut encoder)
    .unwrap();

    //--------------

    let mut encoder = rustbif::Encoder {
        writer: std::io::BufWriter::new(std::fs::File::create("foo.ld").unwrap()),
    };
    let mut encoder2 = rustbif::Encoder { writer: Vec::new() };

    println!("----------------------ContentIDs!");
    // let hash = core::array::from_fn::<u8, 32, _>(|i| i as u8 + 1);
    // let cid = ContentID::File(Hash::SHA2_256(ByteArr(hash)));
    // let cids = vec![cid; 50_000_000];
    // cids.encode(&mut encoder).unwrap();

    for i in 0..1 {
        let i: u32 = i;
        let mut hash = [0_u8; 32];
        hash[0..4].copy_from_slice(&i.to_le_bytes()[..]);
        let cid = if i % 2 == 0 {
            ContentID::File(Hash::Sha2x256(ByteArr(hash)))
        } else {
            ContentID::Link(Hash::Sha2x256(ByteArr(hash)))
        };
        cid.encode(&mut encoder).unwrap();
        cid.encode(&mut encoder2).unwrap();
        let cid32 = multibase::encode(Base::Base32Lower, &encoder2.writer);
        println!("{cid32}");
        let alphabet = "abcdefghijklmnopqrstuvwxyz234567";
        let encoded = base_x::encode(alphabet, &encoder2.writer);
        println!("{encoded}");
        encoder2.writer.clear();
    }

    println!("\n----------------------Done encoding!");

    let x = b"abc";
    let multi58 = multibase::encode(Base::Base58Btc, &x);
    let multi64 = multibase::encode(Base::Base64, &x);
    let multi32 = multibase::encode(Base::Base32Lower, &x);
    let multi16 = multibase::encode(Base::Base16Lower, &x);
    let multi02 = multibase::encode(Base::Base2, &x);
    dbg!((multi64, multi58, multi32, multi16, multi02));

    Ok(())
}
