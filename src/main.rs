use std::{env, fs, io};

use eyre::{bail, Result};

use synacor_vm::VM;

fn main() -> Result<()> {
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
        VM::load_snapshot(io::stdin(), io::stdout(), fs::File::open(snapshot)?)?
    } else {
        VM::load_program(io::stdin(), io::stdout(), include_bytes!("challenge.bin"))
    };

    loop {
        match vm.cycle() {
            Ok(()) => {}

            Err(err) => {
                if let Some(synacor_vm::Error::Halt) = err.downcast_ref::<synacor_vm::Error>() {
                    break;
                }

                bail!(err);
            }
        }
    }

    vm.save_snapshot(fs::File::create("snapshot.bin")?)
}
