use std::io::stdout;

use crossterm::{
    cursor::MoveTo,
    event::{self, Event::Key, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use ropey::Rope;
use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor};
use tree_sitter_json::HIGHLIGHTS_QUERY;

pub struct ColorInfo {
    start: usize,
    end: usize,
    fg: Color,
}

fn main() -> std::io::Result<()> {
    let source_code = include_str!("../input.json");
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_json::language())
        .expect("Error loading Rust grammar");
    let mut tree = parser.parse(source_code, None).unwrap();
    let mut rope = Rope::from_str(source_code);
    let query = Query::new(&tree_sitter_json::language(), HIGHLIGHTS_QUERY).unwrap();

    terminal::enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;

    let mut col: u16 = 0;
    let mut row: u16 = 0;

    loop {
        let mut rcol = 0;
        let mut rrow = 0;
        execute!(stdout(), Clear(ClearType::All),).unwrap();

        let mut colors = Vec::new();
        let mut cursor = QueryCursor::new();
        let slice = rope.slice(..rope.len_bytes()).as_str().unwrap();
        let matches = cursor.matches(&query, tree.root_node(), slice.as_bytes());

        let folds = [(5, 16)];

        for m in matches {
            for cap in m.captures {
                let node = cap.node;
                let start = node.start_byte();
                let end = node.end_byte();
                let capture_name = query.capture_names()[cap.index as usize];
                if capture_name == "string" {
                    colors.push(ColorInfo {
                        start,
                        end,
                        fg: Color::Blue,
                    });
                } else if capture_name == "number" {
                    colors.push(ColorInfo {
                        start,
                        end,
                        fg: Color::Green,
                    });
                }
            }
        }

        for (idx, char) in rope.chars().enumerate() {
            if let Some(fold) = folds.iter().find(|fold| fold.0 <= rrow && fold.1 >= rrow) {
                if rrow == fold.0 {
                    execute!(
                        stdout(),
                        MoveTo(rcol, rrow),
                        SetForegroundColor(Color::Grey),
                        Print(format!(" {} lines collapsed", fold.1 - fold.0))
                    )
                    .unwrap();
                } else {
                    if char.eq(&'\n') {
                        rrow += 1;
                        rcol = 0;
                    }
                    continue;
                }
            }
            if let Some(has_style) = colors.iter().find(|c| c.start <= idx && c.end >= idx) {
                execute!(
                    stdout(),
                    MoveTo(rcol, rrow),
                    SetForegroundColor(has_style.fg),
                    Print(char)
                )
                .unwrap();
            } else {
                execute!(
                    stdout(),
                    MoveTo(rcol, rrow),
                    SetForegroundColor(Color::White),
                    Print(char)
                )
                .unwrap();
            }

            rcol += 1;
            execute!(stdout(), MoveTo(rcol, rrow)).unwrap();
            if char.eq(&'\n') {
                rrow += 1;
                rcol = 0;
            }
        }

        execute!(stdout(), MoveTo(col, row)).unwrap();
        match event::read()? {
            Key(KeyEvent {
                code: KeyCode::Char('h'),
                ..
            }) => col = col.saturating_sub(1),
            Key(KeyEvent {
                code: KeyCode::Char('j'),
                ..
            }) => row += 1,
            Key(KeyEvent {
                code: KeyCode::Char('k'),
                ..
            }) => row = row.saturating_sub(1),
            Key(KeyEvent {
                code: KeyCode::Char('l'),
                ..
            }) => col += 1,
            Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => break,
            Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }) => {
                let line = rope.line_to_char(row.into());
                let start = line + col as usize;
                rope.insert_char(start, c);
                col += 1;
                tree.edit(&InputEdit {
                    start_byte: start - 1,
                    old_end_byte: start - 1,
                    new_end_byte: start + 1,
                    start_position: Point::new(start - 1, start),
                    old_end_position: Point::new(start - 1, start),
                    new_end_position: Point::new(start - 1, start),
                });
                let slice = rope.slice(..rope.len_bytes()).as_str().unwrap();
                tree = parser.parse(slice, Some(&tree)).unwrap();
            }
            Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) => {
                let line = rope.line_to_char(row.into());
                let start = line + col as usize;
                rope.remove(start - 1..start);
                tree.edit(&InputEdit {
                    start_byte: start - 1,
                    old_end_byte: start - 1,
                    new_end_byte: start - 1,
                    start_position: Point::new(start - 1, start),
                    old_end_position: Point::new(start - 1, start),
                    new_end_position: Point::new(start - 1, start),
                });
                let slice = rope.slice(..rope.len_bytes()).as_str().unwrap();
                tree = parser.parse(slice, Some(&tree)).unwrap();
                col -= 1;
            }
            _ => {}
        }

        execute!(stdout(), MoveTo(col, row))?;
    }

    terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    Ok(())
}
