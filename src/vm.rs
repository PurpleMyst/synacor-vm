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

            self.memory[index] = ((high as u16) << 8) | (low as u16);
            if index == 0 { println!("{} {}", low, high); }
            index += 1;
        });

        Ok(())
    }

    #[inline(always)]
    fn next_argument(&mut self) -> u16 {
        self.pc += 1;
        self.memory[self.pc]
    }

    fn load(&self, address: u16) -> Result<u16, String> {
        // - numbers 0..32767 mean a literal value
        // - numbers 32768..32775 instead mean registers 0..7
        // - numbers 32776..65535 are invalid
        if address <= 32767 {
            Ok(address)
        } else if address <= 32775 {
            Ok(self.registers[(address - 32767) as usize])
        } else {
            Err(format!("Tried to load invalid address {} (at location 0x{:x})", address, self.pc))
        }
    }

    pub fn cycle(&mut self) -> Result<bool, String> {
        let should_increment_pc = true;

        macro_rules! unknown_opcode {
            ($opcode: expr) => {
                return Err(format!("Un-implemented opcode {} (at location 0x{:x})", $opcode, self.pc))
            }
        }

        match self.memory[self.pc] {
            // halt: 0
            //   stop execution and terminate the program
            0 => { return Ok(false) },

            // set: 1 a b
            //   set register <a> to the value of <b>
            1 => { unknown_opcode!(1); },

            // push: 2 a
            //   push <a> onto the stack
            2 => { unknown_opcode!(2); },

            // pop: 3 a
            //   remove the top element from the stack and write it into <a>; empty stack = error
            3 => { unknown_opcode!(3); },

            // eq: 4 a b c
            //   set <a> to 1 if <b> is equal to <c>; set it to 0 otherwise
            4 => { unknown_opcode!(4); },

            // gt: 5 a b c
            //   set <a> to 1 if <b> is greater than <c>; set it to 0 otherwise
            5 => { unknown_opcode!(5); },

            // jmp: 6 a
            //   jump to <a>
            6 => { unknown_opcode!(6); },

            // jt: 7 a b
            //   if <a> is nonzero, jump to <b>
            7 => { unknown_opcode!(7); },

            // jf: 8 a b
            //   if <a> is zero, jump to <b>
            8 => { unknown_opcode!(8); },

            // add: 9 a b c
            //   assign into <a> the sum of <b> and <c> (modulo 32768)
            9 => { unknown_opcode!(9); },

            // mult: 10 a b c
            //   store into <a> the product of <b> and <c> (modulo 32768)
            10 => { unknown_opcode!(10); },

            // mod: 11 a b c
            //   store into <a> the remainder of <b> divided by <c>
            11 => { unknown_opcode!(11); },

            // and: 12 a b c
            //   stores into <a> the bitwise and of <b> and <c>
            12 => { unknown_opcode!(12); },

            // or: 13 a b c
            //   stores into <a> the bitwise or of <b> and <c>
            13 => { unknown_opcode!(13); },

            // not: 14 a b
            //   stores 15-bit bitwise inverse of <b> in <a>
            14 => { unknown_opcode!(14); },

            // rmem: 15 a b
            //   read memory at address <b> and write it to <a>
            15 => { unknown_opcode!(15); },

            // wmem: 16 a b
            //   write the value from <b> into memory at address <a>
            16 => { unknown_opcode!(16); },

            // call: 17 a
            //   write the address of the next instruction to the stack and jump to <a>
            17 => { unknown_opcode!(17); },

            // ret: 18
            //   remove the top element from the stack and jump to it; empty stack = halt
            18 => { unknown_opcode!(18); },

            // out: 19 a
            //   write the character represented by ascii code <a> to the terminal
            19 => {
                let a = self.next_argument();

                print!("{}", self.load(a)? as u8 as char);
            },

            // in: 20 a
            //   read a character from the terminal and write its ascii code to <a>; it can be assumed that once input starts, it will continue until a newline is encountered; this means that you can safely read whole lines from the keyboard and trust that they will be fully read
            20 => { unknown_opcode!(20) },

            // noop: 21
            //   no operation
            21 => { /* do nothing */ },

            unknown_opcode => {
                return Err(format!(
                    "Unknown opcode {} (at location 0x{:x})",
                    unknown_opcode, self.pc
                ))
            }
        }

        if should_increment_pc {
            self.pc += 1;
        }

        Ok(true)
    }
}
