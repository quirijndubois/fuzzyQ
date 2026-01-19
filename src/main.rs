use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::time::Instant;

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

struct Suggestion {
    text: String,
    match_score: usize,
}

fn get_suggestions(search_term: &str, options: &[String]) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = options
        .iter()
        .map(|option| {
            let mut match_length = 0;
            for (c1, c2) in search_term.chars().zip(option.chars()) {
                if c1.to_ascii_lowercase() == c2.to_ascii_lowercase() {
                    match_length += 1;
                } else {
                    break;
                }
            }
            Suggestion {
                text: option.clone(),
                match_score: match_length,
            }
        })
        .collect();

    suggestions.sort_by(|a, b| b.match_score.cmp(&a.match_score));
    suggestions
}

fn main() -> io::Result<()> {
    let file = File::open("words.txt").expect("Could not open words.txt");
    let reader = BufReader::new(file);
    let sample_options: Vec<String> = reader.lines().filter_map(Result::ok).collect();

    let mut typed = String::new();
    let mut last_suggestion_count = 0;
    let mut stdout = io::stdout();

    let _guard = TerminalGuard::new()?;

    loop {
        if event::poll(std::time::Duration::from_millis(10))? {
            if let Event::Key(key_event) = event::read()? {
                // Correct Ctrl+C handling
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    break;
                }

                match key_event.code {
                    KeyCode::Enter | KeyCode::Esc => break,
                    KeyCode::Backspace => {
                        typed.pop();
                    }
                    KeyCode::Char(c) => {
                        typed.push(c);
                    }
                    _ => {}
                }
            }
        }

        let start_time = Instant::now();
        let suggestions = get_suggestions(&typed, &sample_options);
        let top_suggestions = &suggestions[..suggestions.len().min(20)];
        let delta_time_str = format!("{:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);

        // 1. Clear previous
        for _ in 0..last_suggestion_count {
            execute!(
                stdout,
                cursor::MoveDown(1),
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
        }
        if last_suggestion_count > 0 {
            execute!(stdout, cursor::MoveUp(last_suggestion_count as u16))?;
        }

        // 2. Write new
        for sug in top_suggestions {
            execute!(
                stdout,
                cursor::MoveDown(1),
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;

            let m_end = sug.match_score;
            execute!(
                stdout,
                SetForegroundColor(Color::Green),
                Print(&sug.text[..m_end]),
                SetForegroundColor(Color::Reset),
                Print(&sug.text[m_end..])
            )?;
        }

        if !top_suggestions.is_empty() {
            execute!(stdout, cursor::MoveUp(top_suggestions.len() as u16))?;
        }
        last_suggestion_count = top_suggestions.len();

        // 3. Header line
        let (width, _) = terminal::size().unwrap_or((80, 24));
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("Type here: "),
            Print(&typed),
            cursor::MoveToColumn(width.saturating_sub(delta_time_str.len() as u16)),
            SetForegroundColor(Color::DarkGrey),
            Print(&delta_time_str),
            SetForegroundColor(Color::Reset),
            cursor::MoveToColumn((typed.len() + 11) as u16)
        )?;

        stdout.flush()?;
    }

    Ok(())
}
