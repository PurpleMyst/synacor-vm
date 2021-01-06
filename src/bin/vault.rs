use array_iterator::ArrayIterator;
use io::Cursor;
use priority_queue::PriorityQueue;
use std::{cmp::Reverse, collections::HashMap, fmt::Display, fs, io, str::FromStr};

use eyre::{eyre, Report, Result};

use synacor_vm::Room;

const GRID_SIDE: i64 = 4;
const TARGET_WEIGHT: i32 = 30;

type VM = synacor_vm::VM<Cursor<Vec<u8>>, Cursor<Vec<u8>>>;

#[derive(Clone, Copy, Debug)]
enum Cell {
    Num(i32),
    Add,
    Mul,
    Sub,
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::Num(n) => n.fmt(f),
            Cell::Add => f.pad("+"),
            Cell::Mul => f.pad("*"),
            Cell::Sub => f.pad("-"),
        }
    }
}

impl FromStr for Cell {
    type Err = Report;

    fn from_str(desc: &str) -> Result<Self> {
        Ok(if desc.contains('+') {
            Cell::Add
        } else if desc.contains('-') {
            Cell::Sub
        } else if desc.contains('*') {
            Cell::Mul
        } else {
            let start = desc.find('\'').ok_or_else(|| eyre!("weird desc"))?;
            let end = (start + 1)
                + desc[start + 1..]
                    .find('\'')
                    .ok_or_else(|| eyre!("weird desc"))?;
            Cell::Num(desc[start + 1..end].parse()?)
        })
    }
}
fn walk(
    grid: &mut HashMap<(i64, i64), Cell>,
    (x, y): (i64, i64),
    vm: Box<VM>,
    room: Room,
    len: usize,
) -> Result<()> {
    // don't revisit visited squares
    if grid.insert((x, y), room.description.parse()?).is_some() {
        return Ok(());
    }

    // the orb disappears at the vault door
    if room.title == "Vault Door" {
        return Ok(());
    }

    for exit in room.exits.into_iter() {
        // go into every exit
        let mut vm = vm.clone();
        vm.append_input(&exit)?;
        vm.append_input("\n")?;
        let (prelude, next_room) = vm.cycle_until_next_room()?;

        // calculate the next position
        let next_pos = match exit.as_ref() {
            "east" => (x + 1, y),
            "west" => (x - 1, y),
            "north" => (x, y + 1),
            "south" => (x, y - 1),
            // don't try to enter the vault
            "vault" => continue,
            _ => unreachable!(),
        };

        // if the orb shatters, we can't go in this direction
        if prelude.contains("shatter") {
            continue;
        }

        if let Some(next_room) = next_room {
            // avoid going outside the grid
            if !next_room.title.starts_with("Vault") || next_room.title == "Vault Antechamber" {
                continue;
            }

            // keep exploring from the next position
            walk(grid, next_pos, vm, next_room, len + 1)?;
        }
    }

    Ok(())
}

fn pathfind(graph: &mut HashMap<(i64, i64), Cell>) -> Vec<(i64, i64)> {
    // map nodes to the currently known shortest path to get there
    let mut dist = HashMap::new();
    dist.insert((0, 0, 22), 0);

    // map nodes to their parent on the currently known shortest path to get there
    let mut prev: HashMap<(i64, i64, i32), (i64, i64, i32)> = HashMap::new();

    // priority queue: we use Reverse() to turn "pop" into a "give me the one with the lowest distance"
    let mut q = PriorityQueue::new();
    q.push((0, 0, 22), Reverse(0));

    while let Some(((x, y, w), Reverse(d))) = q.pop() {
        if (x, y, w) == (GRID_SIDE - 1, GRID_SIDE - 1, TARGET_WEIGHT) {
            // if we've gotten to the door with the correct weight, reconstruct the path and return it
            let mut crumb = (x, y, w);
            let mut path = vec![(x, y)];
            while let Some(ncrumb) = prev.get(&crumb) {
                path.push((ncrumb.0, ncrumb.1));
                crumb = *ncrumb;
            }
            path.reverse();
            return path;
        } else if (x, y) == (GRID_SIDE - 1, GRID_SIDE - 1) {
            // otherwise, the orb disappaers in throne room
            continue;
        }

        let cell = graph[&(x, y)];

        let neighbors = ArrayIterator::new([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)])
            .filter_map(|(nx, ny)| {
                if (nx, ny) == (0, 0) {
                    // orb disappears in antechamber
                    return None;
                }

                let ncell = graph.get(&(nx, ny))?;

                let nw = match (cell, ncell) {
                    (Cell::Num(..), ..) => w,
                    (Cell::Add, Cell::Num(n)) => w + n,
                    (Cell::Mul, Cell::Num(n)) => w * n,
                    (Cell::Sub, Cell::Num(n)) => w - n,
                    _ => unreachable!(),
                };
                if !((0..4).contains(&nx) && (0..4).contains(&ny) && (0..32768).contains(&nw)) {
                    return None;
                }

                Some((nx, ny, nw))
            });

        for (nx, ny, nw) in neighbors {
            let alt = d + 1;
            if dist.get(&(nx, ny, nw)).map_or(true, |&x| alt < x) {
                dist.insert((nx, ny, nw), alt);
                prev.insert((nx, ny, nw), (x, y, w));
                q.push((nx, ny, nw), Reverse(alt));
            }
        }
    }

    unreachable!()
}

fn find_exits() -> Result<impl Iterator<Item = &'static str>> {
    let mut vm = VM::load_snapshot(
        io::Cursor::new(b"take orb\nlook\n".to_vec()),
        io::Cursor::new(Vec::new()),
        fs::File::open("snapshots/05_vault.snapshot.bin")?,
    )?;

    vm.cycle_until_next_room()?;

    let start = vm.cycle_until_next_room()?.1.unwrap();
    let mut graph = HashMap::new();
    walk(&mut graph, (0, 0), vm, start, 0)?;
    graph.insert((3, 3), Cell::Num(1));

    let mut path = pathfind(&mut graph).into_iter();
    let mut prev = path.next().unwrap();

    Ok(path.map(move |step| {
        let exit = match (step.0 - prev.0, step.1 - prev.1) {
            (1, 0) => "east",
            (-1, 0) => "west",
            (0, 1) => "north",
            (0, -1) => "south",
            _ => unreachable!(),
        };
        prev = step;
        exit
    }))
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let exits = find_exits()?;
    println!("{}", exits.collect::<Vec<_>>().join(" "));

    Ok(())
}
