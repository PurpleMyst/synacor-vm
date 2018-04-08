use std::{fs, io};

const INTEGER_SIZE: usize = 15;
const MAX_VALUE: u16 = 1 << INTEGER_SIZE;
const ADDRESS_SPACE: usize = MAX_VALUE as usize;
const REGISTER_COUNT: usize = 8;

pub type Stack<T> = Vec<T>;

#[allow(dead_code)]
pub struct VM {
    memory: [u16; ADDRESS_SPACE],
    registers: [u16; REGISTER_COUNT],
    stack: Stack<u16>,

    pc: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            memory: [0; ADDRESS_SPACE],
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
            Ok(self.registers[(address - 32768) as usize])
        } else {
            Err(format!("Tried to load invalid address {} (at location 0x{:x})", address, self.pc))
        }
    }

    fn set(&mut self, destination_address: u16, source_address: u16) -> Result<(), String> {
        let source = self.load(source_address)?;

        let destination = if destination_address >= 32768 && destination_address <= 32775 {
            &mut self.registers[(destination_address - 32768) as usize]
        } else {
            return Err(format!("Tried to store at invalid register {} (at location 0x{:x})", destination_address, self.pc))
        };

        *destination = source;
        Ok(())
    }

    pub fn cycle(&mut self) -> Result<bool, String> {
        let mut should_increment_pc = true;

        macro_rules! unknown_opcode {
            ($opcode: expr) => {
                return Err(format!("Un-implemented opcode {} (at location 0x{:x})", $opcode, self.pc))
            }
        }

        macro_rules! jmp {
            ($location: expr) => {
                self.pc = ($location) as usize;
                should_increment_pc = false;
            }
        }

        macro_rules! bool_operation {
            ($a:ident = $b:ident $op:tt $c:ident) => {
                if self.load($b)? $op self.load($c)? {
                    self.set($a, 1)?;
                } else {
                    self.set($a, 0)?;
                }
            }
        }

        macro_rules! binary_operation {
            ($a:ident = $b:ident $op:tt $c:ident) => {
                let __result = (self.load($b)? $op self.load($c)?) % MAX_VALUE;
                self.set($a, __result)?;
            }
        }

        match self.memory[self.pc] {
            // halt: 0
            //   stop execution and terminate the program
            0 => { return Ok(false) },

            // set: 1 a b
            //   set register <a> to the value of <b>
            1 => {
                let a = self.next_argument();
                let b = self.next_argument();
                self.set(a, b)?;
            }

            // push: 2 a
            //   push <a> onto the stack
            2 => {
                let a = self.next_argument();
                let a_value = self.load(a)?;
                self.stack.push(a_value);
            }

            // pop: 3 a
            //   remove the top element from the stack and write it into <a>; empty stack = error
            3 => {
                let a = self.next_argument();

                if let Some(tos) = self.stack.pop() {
                    self.set(a, tos)?;
                } else {
                    return Err(format!("Tried to pop from an empty stack (at location 0x{:x})", self.pc));
                }
            },

            // eq: 4 a b c
            //   set <a> to 1 if <b> is equal to <c>; set it to 0 otherwise
            4 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                bool_operation!(a = b == c);
            }

            // gt: 5 a b c
            //   set <a> to 1 if <b> is greater than <c>; set it to 0 otherwise
            5 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                bool_operation!(a = b > c);
            }

            // jmp: 6 a
            //   jump to <a>
            6 => {
                let a = self.next_argument();
                jmp!(a);
            },

            // jt: 7 a b
            //   if <a> is nonzero, jump to <b>
            7 => {
                let a = self.next_argument();
                let b = self.next_argument();

                if self.load(a)? != 0 {
                    jmp!(b);
                }
            }

            // jf: 8 a b
            //   if <a> is zero, jump to <b>
            8 => {
                let a = self.next_argument();
                let b = self.next_argument();

                if self.load(a)? == 0 {
                    jmp!(b);
                }
            }

            // add: 9 a b c
            //   assign into <a> the sum of <b> and <c> (modulo 32768)
            9 => { 
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                let result = (self.load(b)? + self.load(c)?) % MAX_VALUE;
                self.set(a, result)?;
            },

            // mult: 10 a b c
            //   store into <a> the product of <b> and <c> (modulo 32768)
            10 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                // XXX: Does this want Rust-style remainder or C-style modulus?
                binary_operation!(a = b * c);
            },

            // mod: 11 a b c
            //   store into <a> the remainder of <b> divided by <c>
            11 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                // XXX: Does this want Rust-style remainder or C-style modulus?
                binary_operation!(a = b % c);
            },

            // and: 12 a b c
            //   stores into <a> the bitwise and of <b> and <c>
            12 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                binary_operation!(a = b & c);
            },

            // or: 13 a b c
            //   stores into <a> the bitwise or of <b> and <c>
            13 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                binary_operation!(a = b | c);
            },

            // not: 14 a b
            //   stores 15-bit bitwise inverse of <b> in <a>
            14 => {
                let a = self.next_argument();
                let b = self.next_argument();

                let b_value = self.load(b)?;

                self.set(a, (!b_value) & ((1 << INTEGER_SIZE) - 1))?;
            }

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
