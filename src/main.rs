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

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

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

fn get_model() -> TextEmbedding {
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    )
    .unwrap();
    model
}

fn get_embeddings(model: &mut TextEmbedding, documents: Vec<&str>) -> Vec<Vec<f32>> {
    let embeddings = model.embed(documents, None).unwrap();
    embeddings
}

fn normalize_embeddings(embeddings: &mut [Vec<f32>]) {
    for emb in embeddings.iter_mut() {
        let norm = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    // we assume normalized vector to apply function simplification (not dividing by norms)
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    dot
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
            score: 100 + query.len(), // give exact matches very high score
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

fn semantic_match(
    candidate: &str,
    query_embedding: &Vec<f32>,
    candidate_embedding: &Vec<f32>,
) -> Option<Suggestion> {
    Some(Suggestion {
        text: candidate.to_string(),
        match_indices: vec![],
        score: (cosine_similarity(query_embedding, candidate_embedding) * 1000.0) as usize,
    })
}

fn get_semantic_suggestions(
    option_embeddings: &[(String, Vec<f32>)],
    query_embedding: &Vec<f32>,
) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = option_embeddings
        .iter()
        .filter_map(|(opt, emb)| semantic_match(opt, query_embedding, emb))
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

fn clear_previous_suggestions(
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

fn draw_suggestions(stdout: &mut io::Stdout, suggestions: &[Suggestion]) -> io::Result<()> {
    let longest_suggestion = suggestions
        .iter()
        .map(|sug| sug.text.len())
        .max()
        .unwrap_or(0);
    let highest_score = suggestions.iter().map(|sug| sug.score).max().unwrap_or(1);
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

fn draw_header(stdout: &mut io::Stdout, typed: &str, delta_time_str: &str) -> io::Result<()> {
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

fn generate_option_embedding_file(options: &[String], path: &str) {
    println!("Loading embedding model...");
    let mut model = get_model();
    println!("Generating option embeddings...");
    let mut option_embeddings =
        get_embeddings(&mut model, options.iter().map(String::as_str).collect());
    println!("Normalizing embeddings...");
    normalize_embeddings(&mut option_embeddings);
    println!("Saving embeddings to file...");
    let mut file = File::create(path).expect("Could not create embedding file");
    for (opt, emb) in options.iter().zip(option_embeddings.iter()) {
        let emb_str: Vec<String> = emb.iter().map(|v| v.to_string()).collect();
        let line = format!("{}\t{}\n", opt, emb_str.join(","));
        file.write_all(line.as_bytes())
            .expect("Could not write to embedding file");
    }
    println!("Embeddings saved to {}", path);
}

fn get_option_embedding_file(path: &str) -> io::Result<Vec<(String, Vec<f32>)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut embeddings = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let mut parts = line.splitn(2, '\t');
        if let (Some(opt), Some(emb_str)) = (parts.next(), parts.next()) {
            let emb: Vec<f32> = emb_str
                .split(',')
                .filter_map(|s| s.parse::<f32>().ok())
                .collect();
            embeddings.push((opt.to_string(), emb));
        }
    }
    Ok(embeddings)
}

fn main() -> io::Result<()> {
    let options_file_path = "words.txt";
    let embeddings_file_path = "word_embeddings.txt";

    let sample_options = read_file(options_file_path);

    let pattern = std::env::args().nth(1).unwrap_or_default();

    if pattern == "--generate-embeddings" {
        generate_option_embedding_file(&sample_options, embeddings_file_path);
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
        embeddings = Some(get_option_embedding_file(embeddings_file_path)?);
        model = Some(get_model());
    }

    draw_header(&mut stdout, &typed, "0.0ms")?;
    clear_previous_suggestions(&mut stdout, last_suggestion_count)?;

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
                    let norm = typed_embed[0].iter().map(|x| x * x).sum::<f32>().sqrt();
                    let mut typed_embed = typed_embed;
                    if norm > 0.0 {
                        for v in typed_embed[0].iter_mut() {
                            *v /= norm;
                        }
                    }
                    suggestions =
                        get_semantic_suggestions(embeddings.as_ref().unwrap(), &typed_embed[0]);
                }

                let top_suggestions = &suggestions[..suggestions.len().min(20)];
                let delta_time_str =
                    format!("{:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);

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
