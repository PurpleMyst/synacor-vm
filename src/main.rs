#![feature(exact_chunks)]

use std::{env, fs};

extern crate colored;
use colored::Colorize;

extern crate serde;

extern crate serde_json;

mod vm;

fn main() {
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
        serde_json::from_str(&fs::read_to_string(snapshot).unwrap()).unwrap()
    } else {
        let mut vm = vm::VM::new();
        vm.load_program_from_file("challenge.bin").unwrap();
        vm
    };

    loop {
        match vm.cycle() {
            Ok(true) => {}

            Ok(false) => break,

            Err(msg) => {
                eprintln!("{}", msg.red());
                break;
            }
        }
    }

    fs::write("snapshot.json", serde_json::to_string(&vm).unwrap()).unwrap();
}
