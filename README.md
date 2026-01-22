simple terminal fuzzy finder written in rust. Right now just searches entries in words.txt. 

# Running (debug)
With rust/cargo installed run:

```sh
cargo run
```

# Build
```
cargo build --release
```
 
then run with:
```sh
./target/release/fuzzyQ
```

# Semantic search
Semantic search uses a local machine learning model to generate vector embeddings for each result option ahead of runtime. If the embeddings are generated the program can perform semantic search by generating a new semantic vector embedding for the search string, then comparing to each word with a cosine similarity function. Right now it is not properly optimized and only runs realtime on performant CPU's. This functionality uses the 'fastembed' cargo package.  

After building the project you can generate the embeddings:
```sh
./target/release/fuzzyQ --generate-embeddings
```
Now you can run the executable with semantic search enabled:
```sh
./target/release/fuzzyQ --semantic
```
