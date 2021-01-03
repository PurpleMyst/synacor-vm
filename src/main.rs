use std::{env, fs, io};

use synacor_vm::VM;

#[allow(clippy::unnecessary_wraps)]
fn print_ch(ch: u8) -> synacor_vm::Result<()> {
    print!("{}", ch as char);
    Ok(())
}

fn main() -> synacor_vm::Result<()> {
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
        VM::load_snapshot(
            Box::new(io::stdin()),
            Box::new(print_ch),
            fs::File::open(snapshot)?,
        )?
    } else {
        VM::load_program(
            Box::new(io::stdin()),
            Box::new(print_ch),
            include_bytes!("challenge.bin"),
        )
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
