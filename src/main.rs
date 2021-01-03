use std::{env, fs};

mod vm;

fn main() {
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
        rmp_serde::from_read(fs::File::open(snapshot).unwrap()).unwrap()
    } else {
        let mut vm = vm::VM::new();
        vm.load_program_from_file("challenge.bin").unwrap();
        vm
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

    rmp_serde::encode::write_named(&mut fs::File::create("snapshot.mp").unwrap(), &vm).unwrap()
}
