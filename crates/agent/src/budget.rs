#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub limit: usize,
    pub used: usize,
}

impl TokenBudget {
    /// Creates a new TokenBudget constraint.
    pub fn new(limit: usize) -> Self {
        Self { limit, used: 0 }
    }

    /// Deducts a number of tokens, returning true if budget limit is reached.
    pub fn deduct(&mut self, amount: usize) -> bool {
        self.used += amount;
        self.used >= self.limit
    }

    /// Evaluates if context summarization should occur to preserve budget.
    #[must_use]
    pub fn should_summarize(&self) -> bool {
        self.used > (self.limit * 80) / 100
    }
}

#[cfg(test)]
mod benches {
    // criterion benchmark stub
}
