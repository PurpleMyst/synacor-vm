use std::{
    convert::TryFrom,
    env, fs,
    io::{self, Seek, Write},
};

use crossterm::event::KeyCode;
use eyre::{bail, Result};

use tui::{layout::*, style::*, widgets::*};

type VM = synacor_vm::VM<io::Cursor<Vec<u8>>, io::Cursor<Vec<u8>>>;

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
    let mut vm = if let Some(snapshot) = env::args().nth(1) {
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
    };

    // Terminal initialization
    let mut terminal = {
        let stdout = io::stdout();
        let backend = tui::backend::CrosstermBackend::new(stdout);
        tui::Terminal::new(backend)?
    };

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

    let mut writes = Vec::new();

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
                    .iter()
                    .map(|(dest, src)| ListItem::new(format!("{:5} <- {}", dest, src)))
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title("Writes"));

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(85), Constraint::Percentage(25)])
                .split(regions[0]);

            let chunks2 = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
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
                    vm.input.get_mut().pop();
                }

                KeyCode::Enter => {
                    vm.output.seek(io::SeekFrom::End(0))?;
                    vm.append_input(b"\n")?;
                    writes.clear();
                    run_until_prompt(&mut vm, &mut writes)?;
                }

                KeyCode::Char(ch) => vm.append_input(&[ch as u8])?,

                KeyCode::Esc => break,

                KeyCode::F(_)
                | KeyCode::Null
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Delete
                | KeyCode::Insert => {}
            },
            crossterm::event::Event::Mouse(..) => unreachable!(),
            crossterm::event::Event::Resize(..) => {}
        }
    }

    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    vm.save_snapshot(fs::File::create("snapshot.bin")?)?;

    Ok(())
}
