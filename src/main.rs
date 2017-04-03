// extern crate clap;
#[macro_use]
extern crate nom;
// we shouldn't really need these
#[macro_use]
extern crate lazy_static;
extern crate regex;

#[allow(dead_code)]
mod desktop;

use std::env;
use desktop::*;

fn main() {
    let path = env::args().nth(1).unwrap();
    let result = parse_file(path).unwrap();
    println!("Parsed: {:?}", result);
}
