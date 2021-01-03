use std::{env, fs};

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
            Ok(()) => {}

            Err(vm::Error::Halt) => break,

            Err(err) => {
                eprintln!("error after location {:#x}: {}", vm.pc, err);
                break;
            }
        }
    }

    fs::write("snapshot.json", serde_json::to_string(&vm).unwrap()).unwrap();
}
