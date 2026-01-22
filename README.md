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

After building the project you can generate the embeddings:
```sh
./target/release/fuzzyQ --generate-embeddings
```
Now you can run the executable with semantic search enabled:
```sh
./target/release/fuzzyQ --semantic
```
