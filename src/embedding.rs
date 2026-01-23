use crate::algorithms;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub fn get_model() -> TextEmbedding {
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
    )
    .unwrap();
    model
}

pub fn generate_embeddings(model: &mut TextEmbedding, documents: Vec<&str>) -> Vec<Vec<f32>> {
    let embeddings = model.embed(documents, None).unwrap();
    embeddings
}

pub fn generate_embeddings_file(options: &[String], path: &str) {
    println!("Loading embedding model...");
    let mut model = get_model();
    println!("Generating option embeddings...");
    let mut option_embeddings =
        generate_embeddings(&mut model, options.iter().map(String::as_str).collect());
    println!("Normalizing embeddings...");
    algorithms::normalize_embeddings(&mut option_embeddings);
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

pub fn get_embeddings_file(path: &str) -> io::Result<Vec<(String, Vec<f32>)>> {
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
