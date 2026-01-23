use crate::algorithms;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

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

pub fn generate_embeddings_file(options: &[String]) -> Vec<Vec<f32>> {
    println!("Loading embedding model...");
    let mut model = get_model();
    println!("Generating option embeddings...");
    let mut option_embeddings =
        generate_embeddings(&mut model, options.iter().map(String::as_str).collect());
    println!("Normalizing embeddings...");
    algorithms::normalize_embeddings(&mut option_embeddings);
    option_embeddings
}
