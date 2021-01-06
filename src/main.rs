use std::{
    convert::TryFrom,
    env, fs,
    io::{self, Cursor, Seek, Write},
};

use crossterm::{
    event::{Event, KeyCode},
    write_ansi_code,
};
use eyre::{bail, Result};

use tui::{
    layout::*,
    style::*,
    text::{Span, Spans},
    widgets::*,
};

type VM = synacor_vm::VM<Cursor<Vec<u8>>, Cursor<Vec<u8>>>;

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

    let first_nonws_offset = vm.output.get_ref()[pos..]
        .iter()
        .position(|ch| !ch.is_ascii_whitespace())
        .unwrap_or(0);
    vm.output.set_position((pos + first_nonws_offset) as u64);

    Ok(())
}

fn make_output_widget(vm: &VM) -> Paragraph {
    Paragraph::new(
        std::str::from_utf8(&vm.output.get_ref()[vm.output.position() as usize..]).unwrap(),
    )
    .block(Block::default().borders(Borders::ALL).title("Output"))
    .wrap(Wrap { trim: true })
}

fn make_writes_widget(writes: &Cursor<Vec<(u32, u32)>>) -> List {
    List::new(
        writes
            .get_ref()
            .iter()
            .skip(writes.position() as usize)
            .map(|(dest, src)| {
                ListItem::new(Spans::from(vec![
                    Span::from(format!("{:5}", dest)),
                    Span::from(" <- "),
                    Span::from(format!("{}", src)),
                ]))
            })
            .collect::<Vec<_>>(),
    )
    .block(Block::default().borders(Borders::ALL).title("Writes"))
}

fn make_state_widget(vm: &VM) -> Table {
    let mut rows = vec![Row::new(vec![
        Cell::from("pc").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from(vm.pc.to_string()),
    ])];

    for (idx, register) in vm.registers.iter().enumerate() {
        rows.push(Row::new(vec![
            Cell::from(format!("r{}", idx)).style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from(register.to_string()),
        ]))
    }

    Table::new(rows)
        .block(Block::default().borders(Borders::ALL).title("State"))
        .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)])
}

fn make_prompt_widget(vm: &VM) -> Paragraph {
    Paragraph::new(
        std::str::from_utf8(&vm.input.get_ref()[vm.input.position() as usize..]).unwrap(),
    )
    .block(Block::default().borders(Borders::ALL).title("Input"))
    .wrap(Wrap { trim: true })
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut writes = Cursor::new(Vec::new());
    let mut vm;

    if let Some(snapshot) = env::args().nth(1) {
        vm = VM::load_snapshot(
            Cursor::new(Vec::new()),
            Cursor::new(Vec::new()),
            fs::File::open(snapshot)?,
        )?;
    } else {
        vm = VM::load_program(
            Cursor::new(Vec::new()),
            Cursor::new(Vec::new()),
            include_bytes!("challenge.bin"),
        );
        run_until_prompt(&mut vm, writes.get_mut())?;
    }

    // Initialize our tui::Terminal
    let mut terminal = {
        let stdout = io::stdout();
        let backend = tui::backend::CrosstermBackend::new(stdout);
        tui::Terminal::new(backend)?
    };

    // Set the terminal into raw mode and go into the alternate screen so we
    // don't mess up the scrollback. Also, on exit, return the terminal into its normal state.
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    scopeguard::defer! {
        let _ = crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        let _ = crossterm::terminal::disable_raw_mode();
    };

    loop {
        terminal.draw(|frame| {
            let output_n_input = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(90), Constraint::Length(1)].as_ref())
                .split(frame.size());

            let output_n_debug = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(85), Constraint::Percentage(25)])
                .split(output_n_input[0]);

            let state_n_writes = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(11), Constraint::Percentage(100)])
                .split(output_n_debug[1]);

            frame.render_widget(make_output_widget(&vm), output_n_debug[0]);
            frame.render_widget(make_state_widget(&vm), state_n_writes[0]);
            frame.render_widget(make_writes_widget(&writes), state_n_writes[1]);
            frame.render_widget(make_prompt_widget(&vm), output_n_input[1]);
        })?;

        match crossterm::event::read()? {
            Event::Key(evt) => match evt.code {
                KeyCode::Backspace => {
                    if !matches!(
                        vm.input.get_ref().get(vm.input.position() as usize),
                        Some(b'\n') | None
                    ) {
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

                KeyCode::Up => {
                    // Get the output offscreen
                    let offscreen = &vm.output.get_ref()[..vm.output.position() as usize];

                    // Iterate over the offscreen lines, starting from the one
                    // above the highest currently shown line
                    let mut lines_above = offscreen
                        .iter()
                        .enumerate()
                        .filter(|&(_, &ch)| ch == b'\n')
                        .map(|(idx, _)| idx)
                        .rev()
                        .peekable();

                    if let Some(mut end) = lines_above.next() {
                        if lines_above.peek().is_some() {
                            // If there's lines above, find the first non-empty
                            // line above the highest currently shown line
                            for start in lines_above {
                                if !vm.output.get_ref()[start + 1..end].is_empty() {
                                    end = start + 1;
                                    break;
                                }

                                end = start;
                            }

                            vm.output.set_position(end as u64);
                        } else {
                            // Otherwise, we must be showing the second-highest
                            // line so we'll set the position to the start of
                            // hte output
                            vm.output.set_position(0);
                        }
                    } else {
                        // If there's no lines above, just set the position to 0
                        vm.output.set_position(0);
                    }
                }

                KeyCode::Down => {
                    let onscreen = &vm.output.get_ref()[vm.output.position() as usize..];

                    let mut lines_below = onscreen
                        .iter()
                        .enumerate()
                        .filter(|&(_, &ch)| ch == b'\n')
                        .map(|(idx, _)| idx);

                    if let Some(mut start) = lines_below.next() {
                        for end in lines_below {
                            if !vm.output.get_ref()[start + 1..end].is_empty() {
                                break;
                            }

                            start = end;
                        }

                        vm.output
                            .set_position(vm.output.position() + start as u64 + 1);
                    }
                }

                KeyCode::F(..)
                | KeyCode::Null
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Delete
                | KeyCode::Insert => {}
            },

            Event::Mouse(..) => unreachable!(),

            Event::Resize(..) => {}
        }
    }

    vm.save_snapshot(fs::File::create("snapshot.bin")?)?;

    Ok(())
}
