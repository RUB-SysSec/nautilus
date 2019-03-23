#![feature(vec_remove_item)]
mod lib;

extern crate serde;
extern crate serde_json;

use std::env;
use std::fs::File;

fn main() {
    let input_path = env::args().nth(1).expect("input filename missing");
    let output_path = env::args().nth(2).expect("output filename missing");

    let mut my_parser = lib::AntlrParser::new();
    my_parser.parse_antlr_grammar(&input_path);

    let of = File::create(output_path).expect("cannot create output file");
    serde_json::to_writer(&of, &my_parser.rules).expect("Can not write to output file");
}
