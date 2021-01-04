use std::{collections::HashMap, fs, io};

use eyre::Result;
use rayon::prelude::*;

use synacor_vm::VM;

fn powmod(x: u32, mut y: u32, m: u32) -> u32 {
    let mut t = 1;
    let mut tmp = x % m;
    while y > 0 {
        if y & 1 > 0 {
            t = t * tmp % m;
        }

        tmp = (tmp * tmp) % m;
        y >>= 1;
    }
    t
}

struct Ackermann {
    r7: u32,
    memo: HashMap<(u32, u32), u32>,
}

impl Ackermann {
    fn new(r7: u32) -> Self {
        Self {
            r7,
            memo: HashMap::new(),
        }
    }

    fn ack(&mut self, r0: u32, r1: u32) -> u32 {
        #[allow(clippy::map_entry)]
        if !self.memo.contains_key(&(r0, r1)) {
            // print!("\x1b[{}m{}\x1b[0m", r0 + 31, r0);

            let v = match (r0, r1) {
                (0, r1) => r1 + 1,

                (r0, 0) => self.ack(r0 - 1, self.r7),

                // first optimization
                // A(1, n) => A(0, A(1, n - 1)) => A(1, n - 1) + 1
                // A(1, n) = A(1, n - 1) + 1
                // second optimization
                // A(1, n) => A(1, n - 1) + 1 => A(1, 0) + n
                (1, r1) => self.ack(1, 0) + r1,

                // first optimization
                // A(2, n) => A(1, A(2, n - 1)) => A(1, A(2, n - 1) - 1) + 1 =>
                // A(1, A(2, n - 1) - 2) + 2 => ... => A(1, 0) + A(2, n - 1)
                // A(2, n) = A(1, 0) + A(2, n - 1)
                // second optimization
                // A(2, n) = A(1, 0) + A(2, n - 1) = 2 * A(1, 0) + A(2, n - 2) => n * A(1, 0) + A(2, 0)
                // A(2, n) = n * A(1, 0) + A(2, 0)
                (2, r1) => r1 * self.ack(1, 0) + self.ack(2, 0),

                // A(3, n) = A(2, A(3, n - 1)) = A(3, n - 1) * A(1, 0) + A(2, 0)
                // = (A(3, n - 2) * A(1, 0) + A(2, 0)) * A(1, 0) + A(2, 0)
                // = A(3, n - 2) * A(1,0)^2 + (A(1,0)+1)*A(2,0)
                // = (A(3, n - 3) * A(1,0) + A(2, 0)) * A(1,0)^2 + (A(1,0)+1)*A(2,0)
                // = A(3, n - 3) * A(1,0)^3 + (A(1,0)^2+A(1,0)+1)*A(2,0)
                // maybe:
                // A(3, n) = A(3, 0) * A(1,0)^n + (A(1,0)^(n-1)+...+A(1,0)^0)*A(2,0)
                (3, r1) => {
                    self.ack(3, 0) * powmod(self.ack(1, 0), r1, 32768)
                        + self.ack(2, 0)
                            * (0..r1)
                                .map(|pwr| powmod(self.ack(1, 0), pwr, 32768))
                                .fold(0, |acc, item| (acc + item) % 32768)
                }

                (r0, r1) => {
                    let y = self.ack(r0, r1 - 1);
                    self.ack(r0 - 1, y)
                }
            } % 32768;

            self.memo.insert((r0, r1), v);
        }

        *self.memo.get(&(r0, r1)).unwrap()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<()> {
    // Load in the snapshot with the teleporter
    let mut vm = VM::load_snapshot(
        io::Cursor::new(Vec::new()),
        io::sink(),
        fs::File::open("snapshots/03_teleporter.snapshot.bin")?,
    )?;

    // Set register 7 to a bogus value
    vm.registers[7] = 0xCA;

    // Use the teleporter and cycle until the ackermann test
    vm.append_input("use teleporter\n")?;
    while vm.pc != 5483 {
        vm.cycle()?;
    }

    // Fetch the ackermann parameters
    let r0 = vm.memory[vm.pc + 2];
    let r1 = vm.memory[vm.pc + 3 + 2];

    // Skip the call to the ackermann function
    vm.pc = 5491;

    // See what we're comparing to
    let target = vm.memory[vm.pc + 3];

    // Calculate the correct r7
    let r7 = (0..32768u32)
        .into_par_iter()
        .find_first(|&r7| Ackermann::new(r7).ack(r0, r1) == target)
        .unwrap();

    // And set the registers appropiately
    vm.registers[0] = target;
    vm.registers[7] = r7;

    // Now save the modified snapshot
    vm.save_snapshot(&mut fs::File::create(
        "snapshots/04_teleporter_patched.snapshot.bin",
    )?)?;

    Ok(())
}
