use std::io::{self, BufRead};

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
    pub description: String,
    pub items: Vec<String>,
    pub exits: Vec<String>,
}

impl Room {
    pub fn parse(b: &mut io::Cursor<Vec<u8>>) -> Result<(String, Option<Self>)> {
        let mut this = Self {
            title: String::new(),
            description: String::new(),
            items: Vec::new(),
            exits: Vec::new(),
        };

        // read everything until the room start header and treat it as the
        // "prelude" to the room
        let mut prelude = Vec::new();
        b.read_until(b'=', &mut prelude)?;
        if prelude.last() == Some(&b'=') {
            prelude.pop();
        }
        let prelude = String::from_utf8(prelude)?;

        // read the room's title
        if b.read_line(&mut this.title)? == 0 {
            // if we've reached EOF, there's no room to be parsed
            return Ok((prelude, None));
        }

        // remove junk from title
        debug_assert!(this.title.starts_with("= "));
        debug_assert!(this.title.ends_with(" ==\n"));
        this.title.drain(..2);
        this.title.drain(this.title.len() - 4..);

        // read room description until empty line
        let mut header = String::new();
        loop {
            header.clear();
            b.read_line(&mut header)?;

            if header == "What do you do?" || header.ends_with(":\n") {
                break;
            }

            this.description.push_str(&header);
        }

        // remove junk from description
        if this.description.ends_with("\n\n") {
            this.description.drain(this.description.len() - 2..);
        }

        loop {
            if header == "What do you do?" {
                break;
            }

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
