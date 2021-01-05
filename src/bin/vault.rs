use array_iterator::ArrayIterator;
use io::Cursor;
use std::{
    cmp::Reverse,
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::Display,
    fs, io,
    str::FromStr,
};

use eyre::{bail, eyre, Report, Result};

use synacor_vm::Room;

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

    fn from_str(desc: &str) -> Result<Self, Self::Err> {
        Ok(if desc.contains('+') {
            Cell::Add
        } else if desc.contains('-') {
            Cell::Sub
        } else if desc.contains('*') {
            Cell::Mul
        } else {
            let start = desc.find('\'').ok_or_else(|| eyre!("weird desc"))?;
            let end = start
                + 1
                + desc[start + 1..]
                    .find('\'')
                    .ok_or_else(|| eyre!("weird desc"))?;
            Cell::Num(desc[start + 1..end].parse()?)
        })
    }
}

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// enum Color {
//     Green,
//     Yellow,
//     Red,
// }

// impl FromStr for Color {
//     type Err = Report;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let flashes = s.find("flashes ").ok_or_else(|| eyre!("no flashes"))?;
//         Ok(match s[flashes..].splitn(3, ' ').nth(1) {
//             Some("green.") => Self::Green,
//             Some("yellow.") => Self::Yellow,
//             Some("red.") => Self::Red,
//             color => bail!("unknown color {:?}", color),
//         })
//     }
// }

fn walk(
    grid: &mut HashMap<(i64, i64), Cell>,
    (x, y): (i64, i64),
    vm: Box<VM>,
    prelude: String,
    room: Room,
    len: usize,
) -> Result<()> {
    match grid.entry((x, y)) {
        Entry::Occupied(entry) => {
            return Ok(());
        }

        Entry::Vacant(entry) => {
            entry.insert(room.description.parse()?);
        }
    }

    if room.title == "Vault Door" {
        return Ok(());
    }

    for exit in room.exits.into_iter() {
        let mut vm = vm.clone();
        vm.append_input(&exit)?;
        vm.append_input("\n")?;

        let (prelude, next_room) = vm.cycle_until_next_room()?;

        let next_pos = match exit.as_ref() {
            "east" => (x + 1, y),
            "west" => (x - 1, y),
            "north" => (x, y + 1),
            "south" => (x, y - 1),
            "vault" => continue,
            _ => unreachable!("{:?}", exit),
        };

        // print!(
        //     "{:?} => {:?} ({:?})",
        //     (x, y),
        //     next_pos,
        //     prelude.parse::<Color>().ok()
        // );

        // print!(
        //     "\x1b[{}m{}\x1b[0m ",
        //     match prelude.parse::<Color>().ok() {
        //         Some(Color::Red) => 31,
        //         Some(Color::Green) => 32,
        //         Some(Color::Yellow) => 33,
        //         None => 30,
        //     },
        //     len + 1
        // );

        if prelude.contains("shatter") {
            // print!(" SHATTER");
            continue;
        }
        // println!()

        if let Some(next_room) = next_room {
            if !next_room.title.starts_with("Vault") || next_room.title == "Vault Antechamber" {
                continue;
            }

            walk(grid, next_pos, vm, prelude, next_room, len + 1)?;
        }

        // println!();
    }

    Ok(())
}

fn pathfind(graph: &mut HashMap<(i64, i64), Cell>) -> Vec<(i64, i64)> {
    //  1  function Dijkstra(Graph, source):
    //  2      dist[source] ← 0                           // Initialization
    //  3
    //  4      create vertex priority queue Q
    //  5
    //  6      for each vertex v in Graph:
    //  7          if v ≠ source
    //  8              dist[v] ← INFINITY                 // Unknown distance from source to v
    //  9              prev[v] ← UNDEFINED                // Predecessor of v
    // 10
    // 11         Q.add_with_priority(v, dist[v])
    // 12
    // 13
    // 14     while Q is not empty:                      // The main loop
    // 15         u ← Q.extract_min()                    // Remove and return best vertex
    // 16         for each neighbor v of u:              // only v that are still in Q
    // 17             alt ← dist[u] + length(u, v)
    // 18             if alt < dist[v]
    // 19                 dist[v] ← alt
    // 20                 prev[v] ← u
    // 21                 Q.decrease_priority(v, alt)
    // 22
    // 23     return dist, prev

    let mut dist = HashMap::new();
    dist.insert((0, 0, 22), 0);

    let mut prev: HashMap<(i64, i64, i32), (i64, i64, i32)> = HashMap::new();

    let mut q = priority_queue::PriorityQueue::new();
    q.push((0, 0, 22), Reverse(0));

    while let Some(((x, y, w), Reverse(d))) = q.pop() {
        if (x, y, w) == (3, 3, 30) {
            let mut crumb = (x, y, w);
            let mut path = vec![(x, y)];
            while let Some(ncrumb) = prev.get(&crumb) {
                println!("{:?}", ncrumb);
                path.push((ncrumb.0, ncrumb.1));
                crumb = *ncrumb;
            }
            path.reverse();
            return path;
        } else if (x, y) == (3, 3) {
            // orb disappaers in throne room
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

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut vm = Box::new(VM::load_snapshot(
        io::Cursor::new(b"take orb\nlook\n".to_vec()),
        io::Cursor::new(Vec::new()),
        fs::File::open("snapshots/05_vault.snapshot.bin")?,
    )?);

    vm.cycle_until_next_room()?;

    let (prelude, start) = vm.cycle_until_next_room()?;
    let mut graph = HashMap::new();
    walk(&mut graph, (0, 0), vm, prelude, start.unwrap(), 0)?;
    println!();
    graph.insert((3, 3), Cell::Num(1));

    let path = pathfind(&mut graph);

    for x in path {
        print!(" {}", graph[&x]);
    }
    println!();

    Ok(())
}
