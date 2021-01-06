use std::{
    convert::TryFrom,
    env, fs,
    io::{self, Seek, Write},
};

use crossterm::event::KeyCode;
use eyre::{bail, Result};

use crossterm::write_ansi_code;

use tui::{
    layout::*,
    style::*,
    text::{Span, Spans},
    widgets::*,
};

type VM = synacor_vm::VM<io::Cursor<Vec<u8>>, io::Cursor<Vec<u8>>>;

const NOTABLE_ADDRESSES: &[u32] = &[3952];

fn run_until_prompt(vm: &mut VM, writes: &mut Vec<(u32, u32)>) -> Result<()> {
    let pos = usize::try_from(vm.output.position())?;

    while !vm.output.get_ref()[pos..].ends_with(b"What do you do?") {
        if vm.memory[vm.pc] == 16 {
            let dest = vm.load(vm.memory[vm.pc + 1])?;
            let src = vm.load(vm.memory[vm.pc + 2])?;
            writes.push((dest, src));
        }

        match vm.cycle() {
            Ok(()) => {}
            Err(err) => {
                if let Some(synacor_vm::Error::Halt) = err.downcast_ref::<synacor_vm::Error>() {
                    break;
                }

                bail!(err);
            }
        }
    }

    vm.output.set_position(pos as u64);

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut vm = Box::new(if let Some(snapshot) = env::args().nth(1) {
        VM::load_snapshot(
            io::Cursor::new(Vec::new()),
            io::Cursor::new(Vec::new()),
            fs::File::open(snapshot)?,
        )?
    } else {
        VM::load_program(
            io::Cursor::new(Vec::new()),
            io::Cursor::new(Vec::new()),
            include_bytes!("challenge.bin"),
        )
    });

    // Terminal initialization
    let mut terminal = {
        let stdout = io::stdout();
        let backend = tui::backend::CrosstermBackend::new(stdout);
        tui::Terminal::new(backend)?
    };

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    scopeguard::defer! {
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    };

    let mut writes = io::Cursor::new(Vec::new());

    loop {
        terminal.draw(|frame| {
            let regions = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(90), Constraint::Length(1)].as_ref())
                .split(frame.size());

            let output = {
                Paragraph::new(
                    std::str::from_utf8(&vm.output.get_ref()[vm.output.position() as usize..])
                        .unwrap()
                        .trim(),
                )
                .block(Block::default().borders(Borders::ALL).title("Output"))
                .wrap(Wrap { trim: true })
            };

            let mut rows = vec![Row::new(vec![
                Cell::from("pc").style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(vm.pc.to_string()),
            ])];

            for (idx, register) in vm.registers.iter().enumerate() {
                rows.push(Row::new(vec![
                    Cell::from(format!("r{}", idx))
                        .style(Style::default().add_modifier(Modifier::BOLD)),
                    Cell::from(register.to_string()),
                ]))
            }

            let state = Table::new(rows)
                .block(Block::default().borders(Borders::ALL).title("State"))
                .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)]);

            let writes_w = List::new(
                writes
                    .get_ref()
                    .iter()
                    .skip(writes.position() as usize)
                    .map(|(dest, src)| {
                        let is_notable = NOTABLE_ADDRESSES.binary_search(&dest).is_ok();

                        let mut dest = Span::from(format!("{:5}", dest));

                        if is_notable {
                            dest.style = dest.style.fg(Color::LightRed);
                        }

                        ListItem::new(Spans::from(vec![
                            dest,
                            Span::from(" <- "),
                            Span::from(format!("{}", src)),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title("Writes"));

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(85), Constraint::Percentage(25)])
                .split(regions[0]);

            let chunks2 = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(11), Constraint::Percentage(100)])
                .split(chunks[1]);

            frame.render_widget(output, chunks[0]);

            frame.render_widget(state, chunks2[0]);
            frame.render_widget(writes_w, chunks2[1]);

            // Render the prompt at the end
            let prompt = {
                Paragraph::new(
                    std::str::from_utf8(&vm.input.get_ref()[vm.input.position() as usize..])
                        .unwrap(),
                )
                .block(Block::default().borders(Borders::ALL).title("Input"))
                .wrap(Wrap { trim: true })
            };
            frame.render_widget(prompt, regions[1]);
        })?;

        match crossterm::event::read()? {
            crossterm::event::Event::Key(evt) => match evt.code {
                KeyCode::Backspace => {
                    if !vm.input.get_ref().is_empty() {
                        vm.input.get_mut().pop();
                    }
                }

                KeyCode::Enter => {
                    vm.output.seek(io::SeekFrom::End(0))?;
                    vm.append_input(b"\n")?;
                    writes.get_mut().clear();
                    writes.set_position(0);
                    run_until_prompt(&mut vm, writes.get_mut())?;
                    vm.input.seek(io::SeekFrom::End(0))?;

                    writes
                        .get_mut()
                        .sort_by_key(|&(dest, _)| NOTABLE_ADDRESSES.binary_search(&dest).is_err());
                }

                KeyCode::Char(ch) => vm.append_input(&[ch as u8])?,

                KeyCode::Esc => break,

                KeyCode::PageUp => {
                    let new_pos = writes.position().saturating_sub(1);
                    writes.set_position(new_pos);
                }

                KeyCode::PageDown => {
                    let new_pos = writes.position() + 1;
                    writes.set_position(new_pos);
                }

                KeyCode::F(..)
                | KeyCode::Null
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Delete
                | KeyCode::Insert => {}
            },
            crossterm::event::Event::Mouse(..) => unreachable!(),
            crossterm::event::Event::Resize(..) => {}
        }
    }

    vm.save_snapshot(fs::File::create("snapshot.bin")?)?;

    Ok(())
}
