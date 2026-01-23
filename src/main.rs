mod algorithms;
mod draw;
mod embedder;
mod file_manager;
mod structs;

use crate::structs::Suggestion;
use crate::structs::terminal_guard::TerminalGuard;

use fastembed::TextEmbedding;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::io::{self, Write};
use std::time::Instant;

fn get_fuzzy_suggestions(query: &str, options: &[String]) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = options
        .iter()
        .filter_map(|opt| algorithms::fuzzy_match(query, opt))
        .collect();

    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions
}

fn get_semantic_suggestions(
    query: &str,
    option_embeddings: &[(String, Vec<f32>)],
    query_embedding: &Vec<f32>,
) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = option_embeddings
        .iter()
        .filter_map(|(opt, emb)| algorithms::semantic_match(query, opt, query_embedding, emb))
        .collect();

    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions
}

fn main() -> io::Result<()> {
    let options_file_path = "words.txt";
    let embeddings_file_path = "word_embeddings.txt";

    let sample_options = file_manager::read_file(options_file_path);

    let pattern = std::env::args().nth(1).unwrap_or_default();

    if pattern == "--generate-embeddings" {
        let option_embeddings = embedder::generate_embeddings_file(&sample_options);
        file_manager::write_embeddings(&sample_options, option_embeddings, embeddings_file_path);
        return Ok(());
    }

    let semantic_search = pattern == "--semantic";

    let mut typed = String::new();
    let mut last_suggestion_count = 0;
    let mut stdout = io::stdout();

    let _guard = TerminalGuard::new()?;

    let mut embeddings: Option<Vec<(String, Vec<f32>)>> = None;
    let mut model: Option<TextEmbedding> = None;

    if semantic_search {
        embeddings = Some(file_manager::read_embeddings_file(embeddings_file_path)?);
        model = Some(embedder::get_model());
    }

    draw::draw_header(&mut stdout, &typed, 0 as f64)?;
    draw::clear_previous_suggestions(&mut stdout, last_suggestion_count)?;

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
                    KeyCode::Backspace => {
                        typed.pop();
                    }
                    KeyCode::Char(c) => typed.push(c),
                    _ => {}
                }

                let start_time = Instant::now();

                let mut suggestions = get_fuzzy_suggestions(&typed, &sample_options);

                if semantic_search {
                    let typed_embed = model.as_mut().unwrap().embed(&[&typed], None).unwrap();
                    suggestions = get_semantic_suggestions(
                        &typed,
                        embeddings.as_ref().unwrap(),
                        &typed_embed[0],
                    );
                }

                let top_suggestions = &suggestions[..suggestions.len().min(20)];
                draw::clear_previous_suggestions(&mut stdout, last_suggestion_count)?;
                draw::draw_suggestions(&mut stdout, top_suggestions)?;
                draw::draw_header(&mut stdout, &typed, start_time.elapsed().as_secs_f64())?;
                stdout.flush()?;

                last_suggestion_count = top_suggestions.len();
            }
        }
    }
    Ok(())
}
