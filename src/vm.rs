use std::{fs, io};

const INTEGER_SIZE: usize = 15;
const MAX_VALUE: usize = 1 << INTEGER_SIZE;
const REGISTER_COUNT: usize = 8;

pub type Stack<T> = Vec<T>;

#[allow(dead_code)]
pub struct VM {
    // Our address space is 15 bits long, so our max address is the same as the max value.
    memory: [u16; MAX_VALUE],
    registers: [u16; REGISTER_COUNT],
    stack: Stack<u16>,

    pc: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            memory: [0; MAX_VALUE],
            registers: [0; REGISTER_COUNT],
            stack: Stack::new(),

            pc: 0,
        }
    }

    pub fn load_program_from_file(&mut self, path: &str) -> io::Result<()> {
        fs::read(path)?.exact_chunks(2).for_each(|chunk| {
            let low: u8 = chunk[0];
            let high: u8 = chunk[1];

            self.memory[self.pc] = u16::from_le(((low as u16) << 8) | (high as u16));
            self.pc += 1;
        });

        Ok(())
    }
}
