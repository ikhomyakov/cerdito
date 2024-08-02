use build_async::*;
use cerdito::{ByteArr, Decode, Encode};
use std::fmt::Debug;

use tokio::io::AsyncWriteExt;

pub struct TokioReader<T>(pub T);
impl<T: tokio::io::AsyncRead + std::marker::Unpin> rustbif::Reader for TokioReader<T> {
    type Error = tokio::io::Error;
    async fn read_async(&mut self, bytes: &mut [u8]) -> Result<usize, Self::Error> {
        let n = tokio::io::AsyncReadExt::read(&mut self.0, bytes).await?;
        Ok(n)
    }
}

pub struct TokioWriter<T>(pub T);
impl<T: tokio::io::AsyncWrite + std::marker::Unpin> rustbif::Writer for TokioWriter<T> {
    type Error = tokio::io::Error;
    async fn write_async(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        let n = tokio::io::AsyncWriteExt::write(&mut self.0, bytes).await?;
        Ok(n)
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub enum Hash {
    #[default]
    None,
    Sha2x256(ByteArr<32>) = 0x12,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub enum ContentID {
    #[default]
    None,
    File(Hash),
    Link(Hash),
}

#[repr(u8)]
#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub enum EncryptionKey {
    #[default]
    None = 0,
    Aes256CtrIvA(ByteArr<32>) = 1,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct FileName(String); // string w/o '/'

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
#[derive(Debug, Default, PartialEq, Clone, Encode, Decode)]
pub enum DirectoryEntry {
    #[default]
    None,
    Regular {
        name: FileName,
        content_id: ContentID,
        encryption_key: EncryptionKey,
    } = 1,
    Directory {
        name: FileName,
        content_id: ContentID,
        encryption_key: EncryptionKey,
    } = 2,
}

#[repr(u8)]
#[derive(Debug, Default, PartialEq, Clone, Encode, Decode)]
pub enum DirectoryEntryV2 {
    #[default]
    None = 0,
    Regular {
        name: FileName,
        content_id: ContentID,
        encryption_key: EncryptionKey,
    } = 1,
    Directory {
        name: FileName,
        content_id: ContentID,
        encryption_key: EncryptionKey,
    } = 2,
    SymLink {
        name: FileName,
        content_id: ContentID,
    } = 4,
}

#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub struct PublicKey;

#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub struct Signature;

#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub struct Link {
    // to verify the link make sure: 1) link CID = hash(public_key) , 2) verify signature with the public_key
    pub content_id: ContentID,
    pub sequence_number: u32,  // to prevent replay attack
    pub time_to_live: u32,     // in seconds, to hint how often this link will be updated
    pub public_key: PublicKey, // the public key of the owner; link cid = hash(public_key)
    pub signature: Signature, // the signature of (directory_entry, sequence_number, time_to_live) that can be verified with the public key
}

#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub struct LinkV2 {
    // to verify the link make sure: 1) link CID = hash(public_key) , 2) verify signature with the public_key
    pub content_id: ContentID,
    pub sequence_number: u32,  // to prevent replay attack
    pub time_to_live: u32,     // in seconds, to hint how often this link will be updated
    pub public_key: PublicKey, // the public key of the owner; link cid = hash(public_key)
    pub signature: Signature, // the signature of (directory_entry, sequence_number, time_to_live) that can be verified with the public key
    pub new_field: (ContentID,),
}

#[derive(Debug, PartialEq, Default, Clone, Encode, Decode)]
pub enum S {
    S1(Box<S>),
    #[default]
    S2,
}

#[tokio::main]
async fn main() {
    let content_id = ContentID::File(Hash::Sha2x256(ByteArr([7; 32])));
    let encryption_key = EncryptionKey::Aes256CtrIvA(ByteArr([9; 32]));
    let directory = DirectoryEntry::Regular {
        name: FileName(String::from("file.txt")),
        content_id: content_id.clone(),
        encryption_key,
    };
    let directory_v2 = DirectoryEntryV2::SymLink {
        name: FileName(String::from("file.txt")),
        content_id: content_id.clone(),
    };
    let link = Link {
        content_id: content_id.clone(),
        sequence_number: 2,
        time_to_live: 900,
        public_key: PublicKey,
        signature: Signature,
    };
    let link_v2 = LinkV2 {
        content_id: content_id.clone(),
        sequence_number: 2,
        time_to_live: 900,
        public_key: PublicKey,
        signature: Signature,
        new_field: (content_id.clone(),),
    };

    let mut vec_encoder = rustbif::Encoder { writer: Vec::new() };
    link.encode_async(&mut vec_encoder).await.unwrap();
    link_v2.encode_async(&mut vec_encoder).await.unwrap();
    directory.encode_async(&mut vec_encoder).await.unwrap();
    directory_v2.encode_async(&mut vec_encoder).await.unwrap();
    let mut vec_decoder = rustbif::Decoder {
        reader: vec_encoder.writer,
    };
    let link2_v2 = LinkV2::decode_async(&mut vec_decoder).await.unwrap();
    dbg!(&link2_v2);
    let link2 = Link::decode_async(&mut vec_decoder).await.unwrap();
    dbg!(&link2);
    let directory2_v2 = DirectoryEntryV2::decode_async(&mut vec_decoder)
        .await
        .unwrap();
    dbg!(&directory2_v2);

    // The following panics: Enum "DirectoryEntry" doesn't support variant 4
    // let directory2 = DirectoryEntry::decode_async(&mut vec_decoder).await.unwrap();
    // dbg!(&directory2);

    let s = S::S1(Box::new(S::S1(Box::new(S::S2))));

    let mut vec_encoder = rustbif::Encoder { writer: Vec::new() };
    directory.encode_async(&mut vec_encoder).await.unwrap();
    link.encode(&mut vec_encoder).unwrap();
    s.encode(&mut vec_encoder).unwrap();

    // The following does not compile: recursion in an async fn requires boxing
    //s.encode_async(&mut vec_encoder).await.unwrap();

    let mut file_encoder = rustbif::Encoder {
        writer: TokioWriter(tokio::io::BufWriter::new(
            tokio::fs::File::create("foo_async.ld").await.unwrap(),
        )),
    };
    directory.encode_async(&mut file_encoder).await.unwrap();
    link.encode_async(&mut file_encoder).await.unwrap();
    file_encoder.writer.0.shutdown().await.unwrap();

    let mut vec_decoder = rustbif::Decoder {
        reader: vec_encoder.writer,
    };
    let directory2 = DirectoryEntry::decode(&mut vec_decoder).unwrap();
    let link2 = Link::decode_async(&mut vec_decoder).await.unwrap();
    let s2 = S::decode(&mut vec_decoder).unwrap();

    // The following does not compile: recursion in an async fn requires boxing
    //let s2 = S::decode_async(&mut vec_decoder).await.unwrap();

    let mut file_decoder = rustbif::Decoder {
        reader: TokioReader(tokio::io::BufReader::new(
            tokio::fs::File::open("foo_async.ld").await.unwrap(),
        )),
    };
    let directory3 = DirectoryEntry::decode_async(&mut file_decoder)
        .await
        .unwrap();
    let link3 = Link::decode_async(&mut file_decoder).await.unwrap();

    dbg!(&directory3);
    dbg!(&link3);
    dbg!(&s2);

    assert_eq!(directory, directory2);
    assert_eq!(directory, directory3);
    assert_eq!(link, link2);
    assert_eq!(link, link3);
    assert_eq!(s, s2);
}
