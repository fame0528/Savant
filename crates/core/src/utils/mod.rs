pub mod embeddings;
pub mod io;
pub mod parsing;

/// Token count utility.
///
/// Returns the number of tokens in a string.
pub fn token_count(_text: &str) -> usize {
    unimplemented!("Implement token count using tiktoken-rs")
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
