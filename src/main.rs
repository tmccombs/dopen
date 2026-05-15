#[allow(dead_code)]
mod desktop;

use desktop::*;
use std::env;
use std::process;

use execute::execute;

fn main() {
    let mut args = env::args();
    let _binary = args.next();
    let path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: dopen <desktop file> [arguments...]");
            process::exit(1);
        }
    };
    let exec_args: Vec<String> = args.collect();
    let entry = match parse_file(&path) {
        Ok(entry) => entry,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
    };

    if let Err(err) = execute(&entry, &exec_args, Some(path)) {
        eprintln!("Failed to execute desktop file: {:?}", err);
        process::exit(1);
    }
}
