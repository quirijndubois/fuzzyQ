use crossterm::{
    cursor, execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};

use std::io;

use crate::structs::Suggestion;

pub fn clear_previous_suggestions(
    stdout: &mut io::Stdout,
    last_suggestion_count: usize,
) -> io::Result<()> {
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

pub fn draw_suggestions(stdout: &mut io::Stdout, suggestions: &[Suggestion]) -> io::Result<()> {
    let longest_suggestion = suggestions
        .iter()
        .map(|sug| sug.text.len())
        .max()
        .unwrap_or(0);
    let lowest_score = suggestions.iter().map(|sug| sug.score).min().unwrap_or(0);
    let terminal_width = terminal::size().unwrap_or((80, 24)).0 as usize;
    let bar_width = terminal_width - longest_suggestion - 10;
    for sug in suggestions {
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
        let score_ratio = (sug.score as f32 - lowest_score as f32) / 1000 as f32;
        let score_value_string = format!(" {}", sug.score as f32);
        let score_bar_string = "█".repeat((score_ratio * bar_width as f32).round() as usize);
        execute!(
            stdout,
            cursor::MoveToColumn(longest_suggestion as u16 + 2),
            SetForegroundColor(Color::DarkGrey),
            Print(score_bar_string + &score_value_string),
        )?;
    }

    if !suggestions.is_empty() {
        execute!(stdout, cursor::MoveUp(suggestions.len() as u16))?;
    }
    Ok(())
}

pub fn draw_header(stdout: &mut io::Stdout, typed: &str, delta_time_str: &str) -> io::Result<()> {
    let (width, _) = terminal::size().unwrap_or((80, 24));
    let query_hint = "Search query: ";
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        SetForegroundColor(Color::Reset),
        Print(query_hint),
        Print(&typed),
        cursor::MoveToColumn(width.saturating_sub(delta_time_str.len() as u16)),
        SetForegroundColor(Color::DarkGrey),
        Print(&delta_time_str),
        SetForegroundColor(Color::Reset),
        cursor::MoveToColumn((typed.len() + query_hint.len()) as u16)
    )?;
    Ok(())
}
