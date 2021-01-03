use std::{env, fs, io};

use synacor_vm::VM;

fn main() -> synacor_vm::Result<()> {
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
        VM::load_snapshot(io::stdin(), io::stdout(), fs::File::open(snapshot)?)?
    } else {
        VM::load_program(io::stdin(), io::stdout(), include_bytes!("challenge.bin"))
    };

    loop {
        match vm.cycle() {
            Ok(()) => {}

            Err(synacor_vm::Error::Halt) => break,

            Err(err) => {
                eprintln!("error after location {:#x}: {}", vm.pc, err);
                break;
            }
        }
    }

    vm.save_snapshot(fs::File::create("snapshot.bin")?)
}
