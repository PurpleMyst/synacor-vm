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
            Err(format!("Tried to load invalid address {} (at location {:x})", address, self.pc))
        }
    }

    pub fn cycle(&mut self) -> Result<bool, String> {
        let should_increment_pc = true;

        match self.memory[self.pc] {
            // halt: 0
            //   stop execution and terminate the program
            0 => { return Ok(false) },

            // set: 1 a b
            //   set register <a> to the value of <b>
            1 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 1, self.pc))},

            // push: 2 a
            //   push <a> onto the stack
            2 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 2, self.pc))},

            // pop: 3 a
            //   remove the top element from the stack and write it into <a>; empty stack = error
            3 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 3, self.pc))},

            // eq: 4 a b c
            //   set <a> to 1 if <b> is equal to <c>; set it to 0 otherwise
            4 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 4, self.pc))},

            // gt: 5 a b c
            //   set <a> to 1 if <b> is greater than <c>; set it to 0 otherwise
            5 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 5, self.pc))},

            // jmp: 6 a
            //   jump to <a>
            6 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 6, self.pc))},

            // jt: 7 a b
            //   if <a> is nonzero, jump to <b>
            7 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 7, self.pc))},

            // jf: 8 a b
            //   if <a> is zero, jump to <b>
            8 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 8, self.pc))},

            // add: 9 a b c
            //   assign into <a> the sum of <b> and <c> (modulo 32768)
            9 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 9, self.pc))},

            // mult: 10 a b c
            //   store into <a> the product of <b> and <c> (modulo 32768)
            10 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 10, self.pc))},

            // mod: 11 a b c
            //   store into <a> the remainder of <b> divided by <c>
            11 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 11, self.pc))},

            // and: 12 a b c
            //   stores into <a> the bitwise and of <b> and <c>
            12 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 12, self.pc))},

            // or: 13 a b c
            //   stores into <a> the bitwise or of <b> and <c>
            13 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 13, self.pc))},

            // not: 14 a b
            //   stores 15-bit bitwise inverse of <b> in <a>
            14 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 14, self.pc))},

            // rmem: 15 a b
            //   read memory at address <b> and write it to <a>
            15 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 15, self.pc))},

            // wmem: 16 a b
            //   write the value from <b> into memory at address <a>
            16 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 16, self.pc))},

            // call: 17 a
            //   write the address of the next instruction to the stack and jump to <a>
            17 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 17, self.pc))},

            // ret: 18
            //   remove the top element from the stack and jump to it; empty stack = halt
            18 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 18, self.pc))},

            // out: 19 a
            //   write the character represented by ascii code <a> to the terminal
            19 => {
                let a = self.next_argument();

                print!("{}", self.load(a)? as u8 as char);
            },

            // in: 20 a
            //   read a character from the terminal and write its ascii code to <a>; it can be assumed that once input starts, it will continue until a newline is encountered; this means that you can safely read whole lines from the keyboard and trust that they will be fully read
            20 => { return Err(format!("Un-implemented opcode {} (at location {:x})", 20, self.pc))},

            // noop: 21
            //   no operation
            21 => { /* do nothing */ },

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

        Ok(true)
    }
}
