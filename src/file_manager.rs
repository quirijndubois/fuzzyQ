use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub fn read_file(path: &str) -> Vec<String> {
    let file = File::open(path).expect("Could not open words.txt");
    let reader = BufReader::new(file);
    let sample_options: Vec<String> = reader.lines().filter_map(Result::ok).collect();
    return sample_options;
}

pub fn write_embeddings(options: &[String], option_embeddings: Vec<Vec<f32>>, path: &str) {
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

pub fn read_embeddings_file(path: &str) -> io::Result<Vec<(String, Vec<f32>)>> {
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
