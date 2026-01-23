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

fn generate_embeddings(model: &mut TextEmbedding, documents: Vec<&str>) -> Vec<Vec<f32>> {
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
    let q = query.to_lowercase();
    let c = candidate.to_lowercase();

    let mut score: usize = 0;
    let mut match_indices: Vec<usize> = Vec::new();

    // 1. Exact match
    if q == c {
        score = 1000;
        match_indices = (0..q.len()).collect();
        return Some(Suggestion {
            text: candidate.to_string(),
            match_indices,
            score,
        });
    }

    // 2. Substring match
    if let Some(pos) = c.find(&q) {
        score += 200;
        score += q.len() * 10;
        score += 100usize.saturating_sub(pos); // earlier is better
        match_indices = (pos..pos + q.len()).collect();
    }

    // 3. Prefix bonus
    if c.starts_with(&q) {
        score += 150;
    }

    // 4. Subsequence match (always attempt)
    let mut last = 0;
    let mut gaps = 0;

    for qc in q.chars() {
        if let Some(pos) = c[last..].find(qc) {
            let real = last + pos;
            if let Some(prev) = match_indices.last() {
                gaps += real.saturating_sub(*prev + 1);
            }
            match_indices.push(real);
            last = real + 1;
        }
    }

    let matched = match_indices.len();
    if matched > 0 {
        score += matched * 10;
        score += 50usize.saturating_sub(gaps);
    }

    // 5. Edit distance bonus (handles "heyp" -> "hey")
    let dist = levenshtein(&q, &c);
    if dist <= 2 {
        score += (3 - dist) * 30;
    }

    // 6. clamp score to 0 - 1000
    if score > 1000 {
        score = 1000;
    }

    Some(Suggestion {
        text: candidate.to_string(),
        match_indices,
        score,
    })
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut costs: Vec<usize> = (0..=b.len()).collect();

    for (i, ca) in a.chars().enumerate() {
        let mut last = i;
        costs[0] = i + 1;

        for (j, cb) in b.chars().enumerate() {
            let new = if ca == cb {
                last
            } else {
                1 + last.min(costs[j]).min(costs[j + 1])
            };
            last = costs[j + 1];
            costs[j + 1] = new;
        }
    }

    costs[b.len()]
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
    query: &str,
    candidate: &str,
    query_embedding: &Vec<f32>,
    candidate_embedding: &Vec<f32>,
) -> Option<Suggestion> {
    let f_match = fuzzy_match(query, candidate);
    Some(Suggestion {
        text: candidate.to_string(),
        match_indices: f_match.map_or(vec![], |m| m.match_indices),
        score: (cosine_similarity(query_embedding, candidate_embedding) * 1000.0) as usize,
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

fn generate_embedding_file(options: &[String], path: &str) {
    println!("Loading embedding model...");
    let mut model = get_model();
    println!("Generating option embeddings...");
    let mut option_embeddings =
        generate_embeddings(&mut model, options.iter().map(String::as_str).collect());
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

fn get_embeddings_file(path: &str) -> io::Result<Vec<(String, Vec<f32>)>> {
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
        generate_embedding_file(&sample_options, embeddings_file_path);
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
        embeddings = Some(get_embeddings_file(embeddings_file_path)?);
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
                    suggestions = get_semantic_suggestions(
                        &typed,
                        embeddings.as_ref().unwrap(),
                        &typed_embed[0],
                    );
                }

                let top_suggestions = &suggestions[..suggestions.len().min(20)];
                let delta_time_str =
                    format!("{:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);

                clear_previous_suggestions(&mut stdout, last_suggestion_count)?;
                draw_suggestions(&mut stdout, top_suggestions)?;
                draw_header(&mut stdout, &typed, &delta_time_str)?;
                stdout.flush()?;

                last_suggestion_count = top_suggestions.len();
            }
        }
    }
    Ok(())
}
