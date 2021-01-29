pub mod lib;

use lib::{encode, decode};

use std::path::PathBuf;
use std::process::exit;
use std::env;
use std::fs;

const ERR_WRONG_AMOUNT_ARGS: i32 = 1;
const ERR_UNKNOWN_MODE: i32 = 2;

enum Mode {
    Encode,
    Decode
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        usage();
        exit(ERR_WRONG_AMOUNT_ARGS);
    }

    let mode = match args.get(1).unwrap().as_str() {
        "encode" => Mode::Encode,
        "decode" => Mode::Decode,
        arg => {
            println!("Unknown Mode!: {}", arg);
            usage();
            exit(ERR_UNKNOWN_MODE);
        },
    };

    for i in 2..args.len() {
        match mode {
            Mode::Encode => {
                let input = PathBuf::from(args.get(i).unwrap());
                let output = match encode(&input) {
                    Ok(path) => path,
                    Err(err) => {
                        println!("Unable to compress file \'{}\': \n{}", input.to_str().unwrap(), err);
                        continue;
                    }
                };

                let size_in = fs::metadata(&input).unwrap().len() as f64;
                let size_out = fs::metadata(&output).unwrap().len() as f64;

                let compressed_by = 100.0 * (1.0 - (size_out/size_in));

                println!(
                    "\'{}\' -> \'{}\' (deflated {:.1}%)",
                    input.to_str().unwrap(),
                    output.to_str().unwrap(),
                    compressed_by
                )
            },
            Mode::Decode => {
                let mut input = PathBuf::from(args.get(i).unwrap());
                let output = match decode(&mut input) {
                    Ok(path) => path,
                    Err(err) => {
                        println!("Unable to decompress file \'{}\': \n{}", input.to_str().unwrap(), err);
                        continue;
                    }
                };

                println!(
                    "\'{}\' -> \'{}\'",
                    input.to_str().unwrap(),
                    output.to_str().unwrap(),
                )
            },
        }
    }
}

fn usage() {
    let mut msg = String::new();
    msg.push_str("Usage: huffman [MODE] [FILES]\n");
    msg.push_str("  MODE\n");
    msg.push_str("    encode - encodes the given files\n");
    msg.push_str("    decode - decodes the given files\n");

    print!("{}", msg);
}