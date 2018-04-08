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
        let mut index = 0;

        fs::read(path)?.exact_chunks(2).for_each(|chunk| {
            let low = chunk[0];
            let high = chunk[1];

            self.memory[index] = u16::from_le(((low as u16) << 8) | (high as u16));
            index += 1;
        });

        Ok(())
    }

    pub fn cycle(&mut self) -> Result<(), String> {
        let should_increment_pc = true;

        match self.memory[self.pc] {
            unknown_opcode => {
                return Err(format!(
                    "Unknown opcode {} (at memory location {:x})",
                    unknown_opcode, self.pc
                ))
            }
        }

        if should_increment_pc {
            self.pc += 1;
        }

        Ok(())
    }
}
