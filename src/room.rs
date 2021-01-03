use std::io;

use eyre::{bail, eyre, Result};

/*
== Foothills ==
You find yourself standing at the base of an enormous mountain.  At its base to the north, there is a massive doorway.  A sign nearby reads "Keep out!  Definitely no treasure within!"

Things of interest here:
- tablet

There are 2 exits:
- doorway
- south

What do you do?
*/

#[derive(Debug)]
pub struct Room {
    pub title: String,
    pub flavor: String,
    pub items: Vec<String>,
    pub exits: Vec<String>,
}

impl Room {
    pub fn parse(mut b: impl io::BufRead) -> Result<(String, Option<Self>)> {
        let mut this = Self {
            title: String::new(),
            flavor: String::new(),
            items: Vec::new(),
            exits: Vec::new(),
        };

        // Find room start header
        let mut ch = 0;
        let mut prelude = String::new();
        loop {
            if let Err(..) = b.read_exact(std::slice::from_mut(&mut ch)) {
                return Ok((prelude, None));
            }

            if ch == b'=' {
                break;
            } else {
                prelude.push(ch as char);
            }
        }

        // read title
        b.read_line(&mut this.title)?;

        // remove junk from title
        debug_assert!(this.title.starts_with("= "));
        debug_assert!(this.title.ends_with(" ==\n"));
        this.title.drain(..2);
        this.title.drain(this.title.len() - 4..);

        // Read flavor text until empty line
        let mut header = String::new();
        loop {
            header.clear();
            b.read_line(&mut header)?;

            if header == "What do you do?" || header.ends_with(":\n") {
                break;
            }

            this.flavor.push_str(&header);
        }

        // remove junk from flavor
        if this.flavor.ends_with("\n\n") {
            this.flavor.drain(this.flavor.len() - 2..);
        }

        loop {
            // prompt, bail
            if header == "What do you do?" {
                break;
            }

            // exits
            let list = if header.starts_with("There") {
                &mut this.exits
            } else if header.starts_with("Things") {
                &mut this.items
            } else {
                bail!(eyre!("unknown header {:?}", header));
            };

            loop {
                let mut item = String::new();
                b.read_line(&mut item)?;
                if item == "\n" {
                    break;
                }
                // remove junk from item
                item.drain(..2);
                item.drain(item.len() - 1..);
                list.push(item);
            }

            header.clear();
            b.read_line(&mut header)?;
        }

        Ok((prelude, Some(this)))
    }
}
