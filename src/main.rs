#![feature(exact_chunks)]

extern crate colored;
use colored::Colorize;

mod vm;

fn main() {
    let mut vm = vm::VM::new();
    vm.load_program_from_file("challenge.bin").unwrap();

    loop {
        match vm.cycle() {
            Ok(true) => {},

            Ok(false) => break,

            Err(msg) => {
                eprintln!("{}", msg.red());
                break;
            },
        }
    }
}
