#![feature(exact_chunks)]
mod vm;

fn main() {
    let mut vm = vm::VM::new();
    vm.load_program_from_file("challenge.bin").unwrap();

    while vm.cycle().unwrap() {}
}
