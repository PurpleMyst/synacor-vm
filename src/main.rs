use std::{env, fs};

mod vm;
use vm::VM;

fn main() -> vm::Result<()> {
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
        VM::load_snapshot(fs::File::open(snapshot)?)?
    } else {
        VM::load_program(include_bytes!("challenge.bin"))
    };

    loop {
        match vm.cycle() {
            Ok(()) => {}

            Err(vm::Error::Halt) => break,

            Err(err) => {
                eprintln!("error after location {:#x}: {}", vm.pc, err);
                break;
            }
        }
    }

    vm.save_snapshot(fs::File::create("snapshot.bin")?)
}
