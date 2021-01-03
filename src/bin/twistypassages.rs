use io::Cursor;
use std::{collections::HashSet, fs, io};

use eyre::{bail, eyre, Result};

use synacor_vm::Room;

type VM = synacor_vm::VM<Cursor<Vec<u8>>, Cursor<Vec<u8>>>;

fn find_can(visited: &mut HashSet<String>, mut vm: Box<VM>, room: Room) -> Result<Option<Box<VM>>> {
    if !room.items.is_empty() {
        debug_assert!(room.items.len() == 1 && room.items[0] == "can");
        vm.append_input("take can\nuse can\nuse lantern\n")?;
        return Ok(Some(vm));
    }

    for exit in room.exits.into_iter() {
        if exit == "ladder" {
            continue;
        }

        let mut vm = vm.clone();
        vm.append_input(&exit)?;
        vm.append_input("\n")?;

        let next_room = vm.cycle_until_next_room()?.1;
        if let Some(next_room) = next_room {
            if visited.insert(next_room.flavor.clone()) {
                if let Some(can) = find_can(visited, vm, next_room)? {
                    return Ok(Some(can));
                }
            }
        }
    }

    Ok(None)
}

fn walk(visited: &mut HashSet<String>, vm: Box<VM>, room: Room) -> Result<()> {
    for exit in room.exits.into_iter() {
        if exit == "ladder" {
            continue;
        }

        let mut vm = vm.clone();
        vm.append_input(&exit)?;
        vm.append_input("\n")?;

        let (prelude, next_room) = vm.cycle_until_next_room()?;

        let prelude = prelude.trim();

        if prelude.is_empty() {
            /* do nothing */
        } else if prelude.starts_with("Chiseled") {
            eprintln!("{}", prelude);
            vm.save_snapshot(&mut fs::File::create("chiseled.snapshot.bin")?)?;
        } else {
            bail!(eyre!("Unknown prelude: {:?}", prelude));
        }

        if let Some(next_room) = next_room {
            if visited.insert(next_room.flavor.clone()) {
                walk(visited, vm, next_room)?;
            }
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut vm = Box::new(VM::load_snapshot(
        io::Cursor::new(b"look\n".to_vec()),
        io::Cursor::new(Vec::new()),
        fs::File::open("snapshots/00_twistypassages.snapshot.bin")?,
    )?);

    let start = vm.cycle_until_next_room()?.1.unwrap();

    let mut visited = HashSet::new();
    let mut vm = find_can(&mut visited, vm, start)?.unwrap();

    // skip taken message
    vm.cycle_until_next_room()?;

    // skip use can message
    vm.cycle_until_next_room()?;

    // use lantern, for whatever reason, prints the room
    let start = vm.cycle_until_next_room()?.1.unwrap();

    vm.save_snapshot(fs::File::create("snapshots/01_lit_lantern.snapshot.bin")?)?;

    // walk to find chiseled code
    visited.clear();
    walk(&mut visited, vm, start)?;

    Ok(())
}
