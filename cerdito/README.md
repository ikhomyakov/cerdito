# Rust encoding and decoding framework

This encoding and decoding framework is similar to `serde` but much smaller and simpler (hence its name, `cerdito`). It does not use an intermediary data model, does not utilize the visitor pattern, and does not support zero-copy decoding. However, it provides both synchronous and asynchronous APIs. This framework was implemented mainly to support `rustbif`—a compact binary format for encoding Rust data types.

## TODOs
* Fix enum tags: limit to **positive** numbers **up to u32**, allow const expressions in discriminators. Negative disriminators? Consider removing support for explicit variant discriminators.
* Errors (remove unwraps and panics), figure out if need to insert `?;Ok(...)` or return directly.
* Decl macro for tuples.
* [T; N] doesn't implement Default and user can't implement it either. Default is required when "new" program reads "old" data. Need to address it.
* Implement Arr<bool, N>, Arr<char, N>, Arr<u8, N>, ..., Arr<u128, N>, Arr<i8, N>, ..., Arr<i128, N>, Arr<f32, N>, Arr<f64, N>, VArr<bool>, VArr<char>, VArr<u8>, ..., VArr<u128>, VArr<i8>, ..., VArr<i128>, VArr<f32>, VArr<f64>; and Arr<T; N> (we need this to impl Default), VArr<T> (this is the same as Vec<T>, however, it is good to have a newtype in case we need to implement foreign trait).
* Consider using Result<T, E> in your datastructures -- this is a transparent (or not?) wrapper that wont appear on the wire and that will only capture success/failure of sub-structure decoding. How to encode data that has Err()s? 
* Consider using attribute (default) to control compat behaviour at the runtime: when a field is missing, return Ok(default()) OR Error(); Or try to figure out if the type impl Default or not. Explicit attribute could be better, for example, the attr can also provide value for the default.
