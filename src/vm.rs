use std::{
    convert::TryInto,
    io::{self, Read},
    mem::size_of,
};

const INTEGER_SIZE: usize = 15;
const MAX_VALUE: u32 = 1 << INTEGER_SIZE;
const ADDRESS_SPACE: usize = MAX_VALUE as usize;
const REGISTER_COUNT: usize = 8;

pub type Stack<T> = Vec<T>;

pub struct VM {
    pub memory: [u32; ADDRESS_SPACE],

    pub registers: [u32; REGISTER_COUNT],

    pub stack: Stack<u32>,

    pub pc: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IOError(#[from] io::Error),

    #[error("Tried to load invalid address {0:#x}")]
    InvalidLoad(u32),

    #[error("Tried to store at invalid address {0:#x}")]
    InvalidStore(u32),

    #[error("Tried to pop from an empty stack")]
    PopFromEmptyStack,

    #[error("Unknown opcode {0}")]
    UnknownOpcode(u32),

    #[error("Program halted")]
    Halt,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl VM {
    pub fn load_program(program: &'static [u8]) -> Self {
        let mut this = Self {
            memory: [0; ADDRESS_SPACE],
            registers: [0; REGISTER_COUNT],
            stack: Stack::new(),
            pc: 0,
        };

        program
            .chunks_exact(2)
            .zip(this.memory.iter_mut())
            .for_each(|(chunk, cell)| {
                *cell = u32::from(u16::from_le_bytes(chunk.try_into().unwrap()));
            });

        this
    }

    pub fn save_snapshot(&self, mut w: impl io::Write) -> Result<()> {
        // memory: [u32; ADDRESS_SPACE]
        w.write_all(bytemuck::cast_slice(&self.memory))?;

        // registers: [u32; REGISTER_COUNT]
        w.write_all(bytemuck::cast_slice(&self.registers))?;

        // pc: usize,
        w.write_all(&self.pc.to_ne_bytes())?;

        // stack: Stack<u32>
        w.write_all(bytemuck::cast_slice(&self.stack))?;

        Ok(())
    }

    pub fn load_snapshot(mut r: impl io::Read) -> Result<Self> {
        let mut this = Self {
            memory: [0; ADDRESS_SPACE],
            registers: [0; REGISTER_COUNT],
            stack: Stack::new(),
            pc: 0,
        };

        // memory: [u32; ADDRESS_SPACE]
        r.read_exact(bytemuck::cast_slice_mut(&mut this.memory))?;

        // registers: [u32; REGISTER_COUNT]
        r.read_exact(bytemuck::cast_slice_mut(&mut this.registers))?;

        // pc: usize,
        let mut pc_bytes = [0; size_of::<usize>()];
        r.read_exact(&mut pc_bytes)?;
        this.pc = usize::from_ne_bytes(pc_bytes);

        // stack: Stack<u32>
        let mut tos_bytes = [0; size_of::<u32>()];
        while let Ok(()) = r.read_exact(&mut tos_bytes) {
            this.stack.push(u32::from_ne_bytes(tos_bytes));
        }

        Ok(this)
    }

    fn next_argument(&mut self) -> u32 {
        let value = self.memory[self.pc];
        self.pc += 1;
        value
    }

    fn load(&self, address: u32) -> Result<u32> {
        // - numbers 0..32767 mean a literal value
        // - numbers 32768..32775 instead mean registers 0..7
        // - numbers 32776..65535 are invalid
        if address <= 32767 {
            Ok(address)
        } else if address <= 32775 {
            Ok(self.registers[(address - 32768) as usize])
        } else {
            Err(Error::InvalidLoad(address))
        }
    }

    fn set(&mut self, dest: u32, src: u32) -> Result<()> {
        let source = self.load(src)?;

        let destination = if (32768..=32775).contains(&dest) {
            &mut self.registers[(dest - 32768) as usize]
        } else {
            return Err(Error::InvalidStore(dest));
        };

        *destination = source;
        Ok(())
    }

    pub fn cycle(&mut self) -> Result<()> {
        let prev_pc = self.pc;
        match self.do_cycle() {
            Ok(()) => Ok(()),
            err @ Err(..) => {
                self.pc = prev_pc;
                err
            }
        }
    }

    fn do_cycle(&mut self) -> Result<()> {
        macro_rules! jmp {
            ($location:expr) => {
                self.pc = self.load($location)? as usize;
            };
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
                let b = self.load($b)?;
                let c = self.load($c)?;
                let result = (b $op c) % MAX_VALUE;
                self.set($a, result)?;
            }
        }

        macro_rules! halt {
            () => {
                return Err(Error::Halt)
            };
        }

        let opcode = self.memory[self.pc];
        self.pc += 1;

        match opcode {
            // halt: 0
            //   stop execution and terminate the program
            0 => halt!(),

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
                    return Err(Error::PopFromEmptyStack);
                }
            }

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
            }

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

                binary_operation!(a = b + c);
            }

            // mult: 10 a b c
            //   store into <a> the product of <b> and <c> (modulo 32768)
            10 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                binary_operation!(a = b * c);
            }

            // mod: 11 a b c
            //   store into <a> the remainder of <b> divided by <c>
            11 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                // XXX: Does this want Rust-style remainder or C-style modulus?
                binary_operation!(a = b % c);
            }

            // and: 12 a b c
            //   stores into <a> the bitwise and of <b> and <c>
            12 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                binary_operation!(a = b & c);
            }

            // or: 13 a b c
            //   stores into <a> the bitwise or of <b> and <c>
            13 => {
                let a = self.next_argument();
                let b = self.next_argument();
                let c = self.next_argument();

                binary_operation!(a = b | c);
            }

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
            15 => {
                let a = self.next_argument();
                let b = self.next_argument();

                let memory_value = self.memory[self.load(b)? as usize];
                self.set(a, memory_value)?;
            }

            // wmem: 16 a b
            //   write the value from <b> into memory at address <a>
            16 => {
                let a = self.next_argument();
                let b = self.next_argument();

                let memory_location = self.load(a)? as usize;
                let b_value = self.load(b)?;
                self.memory[memory_location] = b_value;
            }

            // call: 17 a
            //   write the address of the next instruction to the stack and jump to <a>
            17 => {
                let a = self.next_argument();
                self.stack.push(self.pc as u32);
                jmp!(a);
            }

            // ret: 18
            //   remove the top element from the stack and jump to it; empty stack = halt
            18 => {
                if let Some(tos) = self.stack.pop() {
                    jmp!(tos);
                } else {
                    halt!();
                }
            }

            // out: 19 a
            //   write the character represented by ascii code <a> to the terminal
            19 => {
                let a = self.next_argument();

                print!("{}", self.load(a)? as u8 as char);
            }

            // in: 20 a
            //   read a character from the terminal and write its ascii code to <a>; it can be assumed that once input starts, it will continue until a newline is encountered; this means that you can safely read whole lines from the keyboard and trust that they will be fully read
            20 => {
                let a = self.next_argument();

                let mut ch = 0;

                loop {
                    if let Err(..) = io::stdin().read_exact(std::slice::from_mut(&mut ch)) {
                        halt!();
                    }

                    // Skip over the CR in windows' line ending
                    if ch != b'\r' {
                        break;
                    }
                }

                self.set(a, u32::from(ch))?;
            }

            // noop: 21
            //   no operation
            21 => { /* do nothing */ }

            unknown_opcode => {
                return Err(Error::UnknownOpcode(unknown_opcode));
            }
        }

        Ok(())
    }
}
