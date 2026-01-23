mod algorithms;
mod draw;
mod embedding;
mod structs;

use crate::structs::Suggestion;
use crate::structs::terminal_guard::TerminalGuard;

use fastembed::TextEmbedding;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

fn get_suggestions(query: &str, options: &[String]) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = options
        .iter()
        .filter_map(|opt| algorithms::fuzzy_match(query, opt))
        .collect();

    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions
}

fn semantic_match(
    query: &str,
    candidate: &str,
    query_embedding: &Vec<f32>,
    candidate_embedding: &Vec<f32>,
) -> Option<Suggestion> {
    let f_match = algorithms::fuzzy_match(query, candidate);
    Some(Suggestion {
        text: candidate.to_string(),
        match_indices: f_match.map_or(vec![], |m| m.match_indices),
        score: (algorithms::cosine_similarity(query_embedding, candidate_embedding) * 1000.0)
            as usize,
    })
}

fn get_semantic_suggestions(
    query: &str,
    option_embeddings: &[(String, Vec<f32>)],
    query_embedding: &Vec<f32>,
) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = option_embeddings
        .iter()
        .filter_map(|(opt, emb)| semantic_match(query, opt, query_embedding, emb))
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

fn main() -> io::Result<()> {
    let options_file_path = "words.txt";
    let embeddings_file_path = "word_embeddings.txt";

    let sample_options = read_file(options_file_path);

    let pattern = std::env::args().nth(1).unwrap_or_default();

    if pattern == "--generate-embeddings" {
        embedding::generate_embeddings_file(&sample_options, embeddings_file_path);
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
        embeddings = Some(embedding::get_embeddings_file(embeddings_file_path)?);
        model = Some(embedding::get_model());
    }

    draw::draw_header(&mut stdout, &typed, "0.0ms")?;
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

                let mut suggestions = get_suggestions(&typed, &sample_options);

                if semantic_search {
                    let typed_embed = model.as_mut().unwrap().embed(&[&typed], None).unwrap();
                    suggestions = get_semantic_suggestions(
                        &typed,
                        embeddings.as_ref().unwrap(),
                        &typed_embed[0],
                    );
                }

                let top_suggestions = &suggestions[..suggestions.len().min(20)];
                let delta_time_str =
                    format!("{:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);

                draw::clear_previous_suggestions(&mut stdout, last_suggestion_count)?;
                draw::draw_suggestions(&mut stdout, top_suggestions)?;
                draw::draw_header(&mut stdout, &typed, &delta_time_str)?;
                stdout.flush()?;

                last_suggestion_count = top_suggestions.len();
            }
        }
    }
    Ok(())
}
