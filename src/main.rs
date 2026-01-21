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
    match_indices: Vec<usize>,
    score: usize,
}

fn fuzzy_match(query: &str, candidate: &str) -> Option<Suggestion> {
    if query.is_empty() {
        return Some(Suggestion {
            text: candidate.to_string(),
            match_indices: vec![],
            score: 0,
        });
    }

    let query_lower = query.to_lowercase();
    let candidate_lower = candidate.to_lowercase();

    if let Some(pos) = candidate_lower.find(&query_lower) {
        let match_indices = (pos..pos + query.len()).collect();
        return Some(Suggestion {
            text: candidate.to_string(),
            match_indices,
            score: 1000 + query.len(), // give exact matches very high score
        });
    }

    let mut match_indices = Vec::new();
    let mut last_pos = 0;
    for qc in query_lower.chars() {
        if let Some(pos) = candidate_lower[last_pos..].find(qc) {
            let real_pos = last_pos + pos;
            match_indices.push(real_pos);
            last_pos = real_pos + 1;
        } else {
            return None; // character not found → no match
        }
    }

    let score = if match_indices.is_empty() {
        0
    } else {
        let gaps: usize = match_indices.windows(2).map(|w| w[1] - w[0] - 1).sum();
        candidate.len() - gaps - match_indices.len()
    };

    Some(Suggestion {
        text: candidate.to_string(),
        match_indices,
        score,
    })
}

fn get_suggestions(query: &str, options: &[String]) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = options
        .iter()
        .filter_map(|opt| fuzzy_match(query, opt))
        .collect();

    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions
}

fn read_file(path: &str) -> Vec<String> {
    let file = File::open(path).expect("Could not open words.txt");
    let reader = BufReader::new(file);
    let sample_options: Vec<String> = reader.lines().filter_map(Result::ok).collect();
    return sample_options;
}

fn clear_previous_suggestions(stdout: &mut io::Stdout, last_suggestion_count: usize) -> io::Result<()> {
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
    Ok(()) 
}

fn draw_suggestions(stdout: &mut io::Stdout, top_suggestions: &[Suggestion]) -> io::Result<()> {
    for sug in top_suggestions {
        execute!(
            stdout,
            cursor::MoveDown(1),
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine)
        )?;

        let mut last_idx = 0;
        for &idx in &sug.match_indices {
            if idx > last_idx {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Reset),
                    Print(&sug.text[last_idx..idx])
                )?;
            }
            execute!(
                stdout,
                SetForegroundColor(Color::Green),
                Print(&sug.text[idx..idx + 1])
            )?;
            last_idx = idx + 1;
        }
        if last_idx < sug.text.len() {
            execute!(
                stdout,
                SetForegroundColor(Color::Reset),
                Print(&sug.text[last_idx..])
            )?;
        }
    }

    if !top_suggestions.is_empty() {
        execute!(stdout, cursor::MoveUp(top_suggestions.len() as u16))?;
    }
    Ok(())
}

fn draw_header(
    stdout: &mut io::Stdout,
    typed: &str,
    delta_time_str: &str,
) -> io::Result<()> {
    let (width, _) = terminal::size().unwrap_or((80, 24));
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(Color::Reset),
        Print("Type here: "),
        Print(&typed),
        cursor::MoveToColumn(width.saturating_sub(delta_time_str.len() as u16)),
        SetForegroundColor(Color::DarkGrey),
        Print(&delta_time_str),
        SetForegroundColor(Color::Reset),
        cursor::MoveToColumn((typed.len() + 11) as u16)
    )?;
    Ok(())
}

fn main() -> io::Result<()> {
    let sample_options = read_file("words.txt");
    let mut typed = String::new();
    let mut last_suggestion_count = 0;
    let mut stdout = io::stdout();

    let _guard = TerminalGuard::new()?;

    loop {
        if event::poll(std::time::Duration::from_millis(10))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.modifiers.contains(KeyModifiers::CONTROL)
                    && key_event.code == KeyCode::Char('c')
                {
                    break;
                }

                match key_event.code {
                    KeyCode::Enter | KeyCode::Esc => break,
                    KeyCode::Backspace => {typed.pop();}
                    KeyCode::Char(c) => typed.push(c), _ => {}
                }

                let start_time = Instant::now();
                let suggestions = get_suggestions(&typed, &sample_options);
                let top_suggestions = &suggestions[..suggestions.len().min(20)];
                let delta_time_str = format!("{:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);

                // ===== Clear previous suggestions =====
                clear_previous_suggestions(&mut stdout, last_suggestion_count)?;

                // ===== Draw suggestions =====
                draw_suggestions(&mut stdout, top_suggestions)?;
                last_suggestion_count = top_suggestions.len();

                // ===== Draw header =====
                draw_header(&mut stdout, &typed, &delta_time_str)?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}
