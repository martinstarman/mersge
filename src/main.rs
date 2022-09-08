use crossterm::{event, terminal};
use tui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Style},
  text::{Span, Spans},
  widgets::{Block, Borders, Paragraph},
};

#[derive(PartialEq)]
enum Column {
  Left,
  Middle,
  Right,
}

#[derive(Debug, PartialEq)]
enum Change {
  None,
  Addition,
  Deletion,
}

struct Line {
  value: String,
  change: Change,
}

struct Context {
  file_name: String,
  local_changes: Vec<Line>,
  incoming_changes: Vec<Line>,
  result: Vec<Line>,
  current_line: usize,
}

fn main() -> Result<(), std::io::Error> {
  let args: Vec<String> = std::env::args().collect();
  if args.len() != 2 {
    println!("Usage: mersge <filename>");
    return Ok(());
  }

  terminal::enable_raw_mode()?;
  let mut buffer = std::io::stdout();

  crossterm::execute!(
    buffer,
    terminal::EnterAlternateScreen,
    event::EnableMouseCapture,
  )?;

  let backend = tui::backend::CrosstermBackend::new(buffer);
  let mut terminal = tui::Terminal::new(backend)?;

  let mut ctx = Context {
    file_name: args[1].clone(),
    local_changes: vec![],
    incoming_changes: vec![],
    result: vec![],
    current_line: 0,
  };

  parse_input_file(&mut ctx);

  loop {
    if !handle_events(&mut ctx) {
      break;
    }
    render(&mut terminal, &mut ctx);
  }

  terminal::disable_raw_mode()?;
  crossterm::execute!(
    terminal.backend_mut(),
    terminal::LeaveAlternateScreen,
    event::DisableMouseCapture
  )?;

  Ok(())
}

fn parse_input_file(ctx: &mut Context) {
  let file = std::fs::read_to_string(&ctx.file_name).expect("Could not read a input file!");
  let mut column = Column::Middle;

  for line in file.lines() {
    if line.starts_with("<<<<<<<") {
      column = Column::Left;
      continue;
    }
    if line.starts_with("=======") {
      column = Column::Right;
      continue;
    }
    if line.starts_with(">>>>>>>") {
      column = Column::Middle;
      continue;
    }

    match column {
      Column::Left => {
        ctx.local_changes.push(Line {
          value: String::from(line),
          change: Change::Addition,
        });
        ctx.result.push(Line {
          value: String::from("#"),
          change: Change::None,
        });
        ctx.incoming_changes.push(Line {
          value: String::from("-"),
          change: Change::Deletion,
        });
      }
      Column::Middle => {
        ctx.local_changes.push(Line {
          value: String::from(line),
          change: Change::None,
        });
        ctx.result.push(Line {
          value: String::from(line),
          change: Change::None,
        });
        ctx.incoming_changes.push(Line {
          value: String::from(line),
          change: Change::None,
        });
      }
      Column::Right => {
        ctx.local_changes.push(Line {
          value: String::from("-"),
          change: Change::Deletion,
        });
        ctx.result.push(Line {
          value: String::from("#"),
          change: Change::None,
        });
        ctx.incoming_changes.push(Line {
          value: String::from(line),
          change: Change::Addition,
        });
      }
    }
  }
}

fn render(
  terminal: &mut tui::Terminal<tui::backend::CrosstermBackend<std::io::Stdout>>,
  ctx: &mut Context,
) {
  terminal
    .draw(|frame| {
      let Rect { height, .. } = frame.size();

      let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(height - 3), Constraint::Min(3)].as_ref())
        .split(frame.size());

      let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
          [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
          ]
          .as_ref(),
        )
        .split(rows[0]);

      let current_line_style = Style::default().bg(Color::Yellow);
      let add_style = Style::default().fg(Color::Green);
      let remove_style = Style::default().fg(Color::Red);
      let control_style = Style::default().fg(Color::LightBlue);

      let mut local_changes: Vec<Spans> = vec![];
      let mut incoming_changes: Vec<Spans> = vec![];
      let mut result: Vec<Spans> = vec![];

      for i in 0..ctx.local_changes.len() {
        let mut style = Style::default();

        if i == ctx.current_line {
          style = style.patch(current_line_style);
        }

        style = match ctx.local_changes[i].change {
          Change::None => style,
          Change::Addition => style.patch(add_style),
          Change::Deletion => style.patch(remove_style),
        };

        let span_text = pad(
          ctx.local_changes[i].value.clone(),
          columns[0].width as usize,
        );

        local_changes.push(Spans::from(Span::styled(String::from(span_text), style)));
      }

      for i in 0..ctx.result.len() {
        let mut style = Style::default();

        if i == ctx.current_line {
          style = style.patch(current_line_style);
        }

        let span_text = pad(ctx.result[i].value.clone(), columns[1].width as usize);

        if ctx.result[i].change != Change::Deletion {
          result.push(Spans::from(Span::styled(String::from(span_text), style)));
        }
      }

      for i in 0..ctx.incoming_changes.len() {
        let mut style = Style::default();

        if i == ctx.current_line {
          style = style.patch(current_line_style);
        }

        style = match ctx.incoming_changes[i].change {
          Change::None => style,
          Change::Addition => style.patch(add_style),
          Change::Deletion => style.patch(remove_style),
        };

        let span_text = pad(
          ctx.incoming_changes[i].value.clone(),
          columns[2].width as usize,
        );

        incoming_changes.push(Spans::from(Span::styled(String::from(span_text), style)));
      }

      let row_top = Block::default();

      let row_bottom = Block::default().borders(Borders::ALL);

      let block_left = Block::default()
        .title("Local changes")
        .borders(Borders::ALL);

      let block_middle = Block::default().title("Result").borders(Borders::ALL);

      let block_right = Block::default()
        .title("Incoming changes")
        .borders(Borders::ALL);

      let text_left = Paragraph::new(local_changes).block(block_left);

      let text_middle = Paragraph::new(result).block(block_middle);

      let text_right = Paragraph::new(incoming_changes).block(block_right);

      let controls = Paragraph::new(vec![Spans::from(vec![
        Span::styled("[Up] ", control_style),
        Span::from("Move up "),
        Span::styled("[Down] ", control_style),
        Span::from("Move down "),
        Span::styled("[L] ", control_style),
        Span::from("Accept local "),
        Span::styled("[R] ", control_style),
        Span::from("Accept incoming "),
        Span::styled("[W] ", control_style),
        Span::from("Write "),
        Span::styled("[Q] ", control_style),
        Span::from("Quit "),
      ])])
      .block(row_bottom);

      frame.render_widget(row_top, rows[0]);
      frame.render_widget(controls, rows[1]);

      frame.render_widget(text_left, columns[0]);
      frame.render_widget(text_middle, columns[1]);
      frame.render_widget(text_right, columns[2]);
    })
    .unwrap();
}

fn handle_events(ctx: &mut Context) -> bool {
  let mut is_running = true;

  match event::read().unwrap() {
    event::Event::Key(event) => {
      match event.code {
        event::KeyCode::Char('q') => is_running = false,
        event::KeyCode::Char('l') => process_change(Column::Left, ctx),
        event::KeyCode::Char('r') => process_change(Column::Right, ctx),
        event::KeyCode::Char('w') => write_file(ctx),
        event::KeyCode::Down => move_down(ctx),
        event::KeyCode::Up => move_up(ctx),
        _ => (),
      };
    }

    event::Event::Mouse(_) => {}

    event::Event::Resize(_, _) => {}
  };

  return is_running;
}

fn process_change(column: Column, ctx: &mut Context) {
  let line: &Line = match column {
    Column::Left => Some(&ctx.local_changes[ctx.current_line]),
    Column::Right => Some(&ctx.incoming_changes[ctx.current_line]),
    _ => None,
  }
  .unwrap();

  match line.change {
    Change::Addition => {
      ctx.result[ctx.current_line].value = line.value.clone();
      ctx.result[ctx.current_line].change = Change::Addition;
    }
    Change::Deletion => {
      ctx.result[ctx.current_line].change = Change::Deletion;
    }
    Change::None => (),
  };
}

fn write_file(ctx: &Context) {
  let mut content = String::new();

  for i in 0..ctx.result.len() {
    if ctx.result[i].change != Change::Deletion {
      content.push_str(ctx.result[i].value.clone().as_str());
      content.push('\n');
    }
  }

  std::fs::write(&ctx.file_name, content).unwrap();
}

fn move_down(ctx: &mut Context) {
  if ctx.current_line < ctx.result.len() - 1 {
    ctx.current_line += 1;
  }
}

fn move_up(ctx: &mut Context) {
  if ctx.current_line > 0 {
    ctx.current_line -= 1;
  }
}

fn pad(mut string: String, len: usize) -> String {
  loop {
    if string.len() >= len {
      break;
    }
    string.push(' ');
  }

  string
}

#[cfg(test)]
mod tests {
  #[test]
  fn parse_input_file() {
    // TODO: implement me
    // TODO: move file reading outside this fn for easier testing
  }

  #[test]
  fn process_change() {
    let mut ctx = crate::Context {
      file_name: String::new(),
      local_changes: vec![
        crate::Line {
          value: String::from("L1"),
          change: crate::Change::Addition,
        },
        crate::Line {
          value: String::from("L2"),
          change: crate::Change::Addition,
        },
      ],
      incoming_changes: vec![
        crate::Line {
          value: String::from("R1"),
          change: crate::Change::Deletion,
        },
        crate::Line {
          value: String::from("R2"),
          change: crate::Change::Addition,
        },
      ],
      result: vec![
        crate::Line {
          value: String::new(),
          change: crate::Change::None,
        },
        crate::Line {
          value: String::new(),
          change: crate::Change::None,
        },
      ],
      current_line: 0,
    };

    crate::process_change(crate::Column::Right, &mut ctx);
    assert_eq!(ctx.result[0].value, "");
    assert_eq!(ctx.result[0].change, crate::Change::Deletion);

    ctx.current_line = 1;

    crate::process_change(crate::Column::Left, &mut ctx);
    assert_eq!(ctx.result[1].value, "L2");
    assert_eq!(ctx.result[1].change, crate::Change::Addition);
  }

  #[test]
  fn move_down() {
    let mut ctx = crate::Context {
      file_name: String::new(),
      local_changes: vec![],
      incoming_changes: vec![],
      result: vec![
        crate::Line {
          value: String::new(),
          change: crate::Change::None,
        },
        crate::Line {
          value: String::new(),
          change: crate::Change::None,
        },
      ],
      current_line: 0,
    };

    crate::move_down(&mut ctx);
    assert_eq!(ctx.current_line, 1);

    crate::move_down(&mut ctx);
    assert_eq!(ctx.current_line, 1);
  }

  #[test]
  fn move_up() {
    let mut ctx = crate::Context {
      file_name: String::new(),
      local_changes: vec![],
      incoming_changes: vec![],
      result: vec![
        crate::Line {
          value: String::new(),
          change: crate::Change::None,
        },
        crate::Line {
          value: String::new(),
          change: crate::Change::None,
        },
      ],
      current_line: 1,
    };

    crate::move_up(&mut ctx);
    assert_eq!(ctx.current_line, 0);

    crate::move_up(&mut ctx);
    assert_eq!(ctx.current_line, 0);
  }

  #[test]
  fn pad() {
    let mut s1 = String::from("ABC");

    s1 = crate::pad(s1, 5);
    assert_eq!(s1.len(), 5);

    s1 = crate::pad(s1, 3);
    assert_eq!(s1.len(), 5);
  }
}
