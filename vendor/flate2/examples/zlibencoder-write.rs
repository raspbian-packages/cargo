extern crate flate2;

use std::io::prelude::*;
use flate2::Compression;
use flate2::write::ZlibEncoder;

// Vec<u8> implements Write to print the compressed bytes of sample string
fn main() {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(b"Hello World").unwrap();
    println!("{:?}", e.finish().unwrap());
}
