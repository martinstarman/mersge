use crossterm::{event, terminal};
use tui::{
  layout::{Constraint, Direction, Layout},
  text::{Span, Spans},
};

#[derive(PartialEq)]
enum Column {
  Left,
  Middle,
  Right,
}

#[derive(PartialEq)]
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
  local_changes: Vec<Line>,
  incoming_changes: Vec<Line>,
  result: Vec<Line>,
  current_line: usize,
  terminal: tui::Terminal<tui::backend::CrosstermBackend<std::io::Stdout>>,
}

fn main() -> Result<(), std::io::Error> {
  let mut buffer = std::io::stdout();

  terminal::enable_raw_mode()?;
  crossterm::execute!(
    buffer,
    terminal::EnterAlternateScreen,
    event::EnableMouseCapture,
  )?;

  let mut ctx = Context {
    local_changes: vec![],
    incoming_changes: vec![],
    result: vec![],
    current_line: 0,
    terminal: tui::Terminal::new(tui::backend::CrosstermBackend::new(buffer))?,
  };

  let args: Vec<String> = std::env::args().collect();

  if args.len() != 2 {
    println!("Usage: \n\tmersge <filename>");
    return Ok(());
  }

  parse(&args[1], &mut ctx);

  loop {
    render(&mut ctx);
    handle_events(&mut ctx);
  }
}

fn parse(file_name: &str, ctx: &mut Context) {
  let file = std::fs::read_to_string(file_name).expect("Could not read a input file!");
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

fn render(ctx: &mut Context) {
  let current_line_style = tui::style::Style::default().bg(tui::style::Color::Yellow);
  let add_style = tui::style::Style::default().fg(tui::style::Color::Green);
  let remove_style = tui::style::Style::default().fg(tui::style::Color::Red);

  let mut local_changes: Vec<Spans> = vec![];
  let mut incoming_changes: Vec<Spans> = vec![];
  let mut result: Vec<Spans> = vec![];

  for i in 0..ctx.local_changes.len() {
    let mut style = tui::style::Style::default();

    if i == ctx.current_line {
      style = style.patch(current_line_style);
    }

    style = match ctx.local_changes[i].change {
      Change::None => style,
      Change::Addition => style.patch(add_style),
      Change::Deletion => style.patch(remove_style),
    };

    local_changes.push(Spans::from(Span::styled(
      String::from(ctx.local_changes[i].value.clone()),
      style,
    )));
  }

  for i in 0..ctx.incoming_changes.len() {
    let mut style = tui::style::Style::default();

    if i == ctx.current_line {
      style = style.patch(current_line_style);
    }

    style = match ctx.incoming_changes[i].change {
      Change::None => style,
      Change::Addition => style.patch(add_style),
      Change::Deletion => style.patch(remove_style),
    };

    incoming_changes.push(Spans::from(Span::styled(
      String::from(ctx.incoming_changes[i].value.clone()),
      style,
    )));
  }

  for i in 0..ctx.result.len() {
    let mut style = tui::style::Style::default();

    if i == ctx.current_line {
      style = style.patch(current_line_style);
    }

    if ctx.result[i].change != Change::Deletion {
      result.push(Spans::from(Span::styled(
        String::from(ctx.result[i].value.clone()),
        style,
      )));
    }
  }

  ctx
    .terminal
    .draw(|frame| {
      let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
          [
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
          ]
          .as_ref(),
        )
        .split(frame.size());

      let column_left = tui::widgets::Block::default()
        .title("Local changes (read only)")
        .borders(tui::widgets::Borders::ALL);

      let column_middle = tui::widgets::Block::default()
        .title("Result")
        .borders(tui::widgets::Borders::ALL);

      let column_right = tui::widgets::Block::default()
        .title("Incoming changes (read only)")
        .borders(tui::widgets::Borders::ALL);

      let text_left = tui::widgets::Paragraph::new(local_changes)
        .block(column_left)
        .wrap(tui::widgets::Wrap { trim: true });

      let text_middle = tui::widgets::Paragraph::new(result)
        .block(column_middle)
        .wrap(tui::widgets::Wrap { trim: true });

      let text_right = tui::widgets::Paragraph::new(incoming_changes)
        .block(column_right)
        .wrap(tui::widgets::Wrap { trim: true });

      frame.render_widget(text_left, chunks[0]);
      frame.render_widget(text_middle, chunks[1]);
      frame.render_widget(text_right, chunks[2]);
    })
    .unwrap();
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

fn handle_events(ctx: &mut Context) {
  match event::read().unwrap() {
    event::Event::Key(event) => {
      match event.code {
        event::KeyCode::Char('q') => exit(ctx),
        event::KeyCode::Char('l') => process_change(Column::Left, ctx),
        event::KeyCode::Char('r') => process_change(Column::Right, ctx),
        event::KeyCode::Down => {
          if ctx.current_line < ctx.result.len() - 1 {
            ctx.current_line += 1;
          }
        }
        event::KeyCode::Up => {
          if ctx.current_line > 0 {
            ctx.current_line -= 1;
          }
        }
        _ => (),
      };
    }

    event::Event::Mouse(_) => {}

    event::Event::Resize(_, _) => {}
  };
}

fn exit(ctx: &mut Context) {
  terminal::disable_raw_mode().unwrap();
  crossterm::execute!(
    ctx.terminal.backend_mut(),
    terminal::LeaveAlternateScreen,
    event::DisableMouseCapture
  )
  .unwrap();
  std::process::exit(0);
}
