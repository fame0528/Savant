# Let's Build GPT: From Scratch, In Code, Spelled Out

> Based on Andrej Karpathy's hands-on coding lecture — building a Transformer language model from scratch.
> Covers the full implementation: tokenization, batching, self-attention, multi-head attention, feed-forward networks,
> residual connections, layer normalization, and the path to ChatGPT.

**Source code**: [nanogpt on GitHub](https://github.com/karpathy/nanoGPT) — two files, ~300 lines each.

---

## Table of Contents

1. [What We're Building](#1-what-were-building)
1. [Dataset & Tokenization](#2-dataset--tokenization)
1. [Data Loading: Batches & Context Windows](#3-data-loading-batches--context-windows)
1. [Bigram Language Model (Baseline)](#4-bigram-language-model-baseline)
1. [Training & Generation Basics](#5-training--generation-basics)
1. [The Self-Attention Trick](#6-the-self-attention-trick)
1. [Self-Attention: Keys, Queries, Values](#7-self-attention-keys-queries-values)
1. [Scaled Dot-Product Attention](#8-scaled-dot-product-attention)
1. [Multi-Head Attention](#9-multi-head-attention)
1. [Feed-Forward Networks](#10-feed-forward-networks)
1. [Residual Connections](#11-residual-connections)
1. [Layer Normalization](#12-layer-normalization)
1. [Dropout](#13-dropout)
1. [The Full Transformer](#14-the-full-transformer)
1. [Encoder vs. Decoder & Cross-Attention](#15-encoder-vs-decoder--cross-attention)
1. [Scaling Results](#16-scaling-results)
1. [From This to ChatGPT](#17-from-this-to-chatgpt)

---

## 1. What We're Building

A **character-level Transformer language model** trained on Tiny Shakespeare (~1 million characters). The goal is to understand the architecture that powers ChatGPT by building it from scratch.

**GPT** = Generatively Pre-trained Transformer — from the 2017 paper ["Attention Is All You Need"](https://arxiv.org/abs/1706.03762). That paper proposed the Transformer for machine translation, but the architecture went on to dominate all of AI.

### What NanoGPT Proves

- Two files, ~300 lines each (model.py + train.py)
- Reproduces GPT-2 (124M parameters) performance when trained on OpenWebText
- Architecturally nearly identical to GPT-3 — just ~10,000–1,000,000× smaller

---

## 2. Dataset & Tokenization

### Tiny Shakespeare

~1 million characters of concatenated Shakespeare works. A character-level language model predicts the next character given a context window.

### Character-Level Tokenizer

The simplest possible tokenization scheme:

```python
# Build vocabulary from unique characters
chars = sorted(list(set(text)))
vocab_size = len(chars)  # 65 for Shakespeare

# Character ↔ integer mappings
stoi = {ch: i for i, ch in enumerate(chars)}
itos = {i: ch for i, ch in enumerate(chars)}

encode = lambda s: [stoi[c] for c in s]      # "hi there" → [46, 47, ...]
decode = lambda l: ''.join([itos[i] for i in l])  # [46, 47, ...] → "hi there"
```

### Tradeoff: Vocabulary Size vs. Sequence Length

| Tokenizer | Vocab Size | "hi there" encodes to |
|-----------|-----------|----------------------|
| Character-level (ours) | 65 | 9 integers |
| GPT-2 (tiktoken, BPE) | 50,257 | 3 integers |

Small vocabulary → long sequences. Large vocabulary → short sequences. Production models use subword tokenization (BPE) as a middle ground.

---

## 3. Data Loading: Batches & Context Windows

### Train/Validation Split

- **90%** training data, **10%** validation (held out to detect overfitting)

### Context Windows (Block Size)

We never feed the entire text at once. Instead, we sample **chunks** of length `block_size` (e.g., 8 characters).

A chunk of 9 characters contains **8 individual training examples**:

```text
Context: [18]              → Target: 47
Context: [18, 47]          → Target: 56
Context: [18, 47, 56]      → Target: 57
...
Context: [18, 47, 56, ...] → Target: 58
```

This trains the model to handle context lengths from 1 up to `block_size`, which is critical during inference when we start with minimal context.

### Batching

Multiple chunks are stacked into a batch tensor for GPU parallelism:

```text
Input X:  (batch_size × block_size)  — e.g., 4×8
Targets Y: (batch_size × block_size) — offset by 1
```

Each row is an independent sequence. Batch elements never interact — they're processed in parallel but independently.

---

## 4. Bigram Language Model (Baseline)

The simplest possible neural language model — predicts the next token based solely on the current token's identity, with no context.

```python
class BigramLanguageModel(nn.Module):
    def __init__(self, vocab_size):
        super().__init__()
        self.token_embedding_table = nn.Embedding(vocab_size, vocab_size)

    def forward(self, idx, targets=None):
        logits = self.token_embedding_table(idx)  # (B, T, C)
        if targets is None:
            loss = None
        else:
            # Reshape for cross_entropy: expects (B*T, C) and (B*T,)
            B, T, C = logits.shape
            logits = logits.view(B * T, C)
            targets = targets.view(B * T)
            loss = F.cross_entropy(logits, targets)
        return logits, loss

    def generate(self, idx, max_new_tokens):
        for _ in range(max_new_tokens):
            logits, _ = self(idx)
            logits = logits[:, -1, :]           # last time step
            probs = F.softmax(logits, dim=-1)
            idx_next = torch.multinomial(probs, num_samples=1)
            idx = torch.cat([idx, idx_next], dim=1)
        return idx
```

**Key idea**: Each token looks up a row in the embedding table and uses it as logits (scores for next token). With 65 possible characters, the initial loss should be ~`-ln(1/65)` ≈ **4.17**. The model starts worse (4.87) because its predictions aren't uniform.

---

## 5. Training & Generation Basics

### Optimizer

**Adam** (lr=1e-3) — more advanced than SGD, works well for typical neural networks.

### Training Loop

```python
optimizer = torch.optim.AdamW(model.parameters(), lr=1e-3)

for steps in range(10000):
    xb, yb = get_batch('train')       # sample random batch
    logits, loss = model(xb, yb)      # forward pass
    optimizer.zero_grad(set_to_none=True)
    loss.backward()                    # compute gradients
    optimizer.step()                   # update parameters
```

### Loss Estimation

Averaging loss over multiple batches (using `@torch.no_grad()`) gives a less noisy estimate than single-batch loss.

### Generation

Start with a newline character (token 0), generate 100 tokens. The Bigram model produces gibberish — tokens aren't communicating with each other yet.

---

## 6. The Self-Attention Trick

Before implementing attention, we need to understand a mathematical trick: **using matrix multiplication for weighted aggregation**.

### The Goal

Each token at position `t` needs to aggregate information from all preceding tokens (positions 0 through `t`). The simplest approach: **average all previous tokens**.

### Naive Implementation (Slow)

```python
xbow = torch.zeros((B, T, C))
for b in range(B):
    for t in range(T):
        xprev = x[b, :t+1]       # all tokens up to and including t
        xbow[b, t] = xprev.mean(0)
```

### Vectorized with Matrix Multiply

```python
# Lower-triangular matrix of ones
a = torch.tril(torch.ones(T, T))
a = a / a.sum(1, keepdim=True)   # normalize rows to sum to 1

# Weighted aggregation via matrix multiply
xbow2 = a @ x  # (T, T) @ (B, T, C) → (B, T, C)
```

**Why this works**: Each row of `a` specifies how much of each preceding token to include. The lower-triangular structure ensures tokens only attend to the past.

### Softmax Version (The One We'll Use)

```python
tril = torch.tril(torch.ones(T, T))
wei = torch.zeros((T, T))
wei = wei.masked_fill(tril == 0, float('-inf'))  # block future
wei = F.softmax(wei, dim=-1)                      # normalize
xbow3 = wei @ x
```

**Key insight**: `wei` starts at zero (uniform affinities). The `masked_fill` prevents future tokens from contributing. Softmax normalizes into a valid probability distribution. In self-attention, the zeros become **data-dependent** — tokens decide how much to attend to each other.

---

## 7. Self-Attention: Keys, Queries, Values

Each token emits three vectors:

| Vector | Purpose |
|--------|---------|
| **Query** | "What am I looking for?" |
| **Key** | "What do I contain?" |
| **Value** | "What will I communicate if you attend to me?" |

Affinities between tokens are computed as **dot products** of keys and queries. High dot product → high attention weight → more information aggregated.

### Single Head Implementation

```python
class Head(nn.Module):
    def __init__(self, head_size):
        super().__init__()
        self.key = nn.Linear(n_embd, head_size, bias=False)
        self.query = nn.Linear(n_embd, head_size, bias=False)
        self.value = nn.Linear(n_embd, head_size, bias=False)
        self.register_buffer('tril', torch.tril(torch.ones(block_size, block_size)))

    def forward(self, x):
        B, T, C = x.shape
        k = self.key(x)    # (B, T, head_size)
        q = self.query(x)  # (B, T, head_size)

        # Compute attention scores
        wei = q @ k.transpose(-2, -1) * C**-0.5  # (B, T, T)
        wei = wei.masked_fill(self.tril[:T, :T] == 0, float('-inf'))
        wei = F.softmax(wei, dim=-1)

        # Aggregate values
        v = self.value(x)  # (B, T, head_size)
        out = wei @ v       # (B, T, head_size)
        return out
```

### Properties of Attention

- **Communication mechanism**: Nodes in a directed graph aggregate information from nodes that point to them, in a data-dependent manner
- **No notion of space**: Tokens are a set of vectors with no inherent position — that's why we add positional embeddings
- **Batch independence**: Tokens across batch elements never communicate
- **Decoder block**: The triangular mask prevents future-to-past communication (autoregressive). Without the mask, it's an **encoder block** (bidirectional)

---

## 8. Scaled Dot-Product Attention

From the "Attention Is All You Need" paper:

```text
Attention(Q, K, V) = softmax(QK^T / √d_k) V
```

The scaling factor `1/√head_size` prevents the dot products from growing too large, which would cause softmax to sharpen into near one-hot vectors.

**Why it matters**: If `Q` and `K` are unit Gaussian, their dot product has variance = `head_size`. Scaling back to variance = 1 keeps softmax diffuse at initialization, allowing the model to aggregate information from multiple tokens rather than just one.

---

## 9. Multi-Head Attention

Multiple attention heads running **in parallel**, each with its own key/query/value projections, then concatenated:

```python
class MultiHeadAttention(nn.Module):
    def __init__(self, num_heads, head_size):
        super().__init__()
        self.heads = nn.ModuleList([Head(head_size) for _ in range(num_heads)])
        self.proj = nn.Linear(n_embd, n_embd)

    def forward(self, x):
        out = torch.cat([h(x) for h in self.heads], dim=-1)
        out = self.proj(out)
        return out
```

**Analogy to group convolution**: Instead of one large attention (32-dim), use four parallel 8-dim attention heads. Each head can learn different communication patterns (e.g., one looks for consonants, another for vowels at specific positions).

**Results**: Validation loss improved from 2.4 → 2.28.

---

## 10. Feed-Forward Networks

After tokens communicate (attention), they need to **think individually** about what they learned:

```python
class FeedForward(nn.Module):
    def __init__(self, n_embd):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(n_embd, 4 * n_embd),
            nn.ReLU(),
            nn.Linear(4 * n_embd, n_embd),  # projection back to residual pathway
        )

    def forward(self, x):
        return self.net(x)
```

- Operates **per-token** (no cross-token communication)
- The inner layer is 4× wider than the embedding dimension (following the original paper)
- Projects back to `n_embd` for the residual connection

**Results**: Validation loss improved from 2.28 → 2.24.

---

## 11. Residual Connections

Deep networks suffer from optimization difficulties. **Skip connections** (from [He et al., 2015](https://arxiv.org/abs/1512.03385)) solve this:

```text
x = x + self.attention(self.ln1(x))   # fork → communicate → add back
x = x + self.feed_forward(self.ln2(x)) # fork → compute → add back
```

**Why it works**: During backpropagation, addition distributes gradients equally to both branches. This creates a **gradient superhighway** from the loss directly to the input, unimpeded. The residual blocks are initialized to contribute near-zero output, then gradually come online during training.

---

## 12. Layer Normalization

**Batch Norm** normalizes across the batch dimension for each neuron. **Layer Norm** normalizes across the feature dimension for each individual token:

```python
class LayerNorm:
    def __call__(self, x):
        # x: (B, T, C) — normalize over C dimension for each (B, T) position
        mean = x.mean(-1, keepdim=True)
        std = x.std(-1, keepdim=True)
        xhat = (x - mean) / std
        out = gamma * xhat + beta  # learnable scale and shift
        return out
```

**Pre-norm formulation** (modern convention): Apply LayerNorm *before* the transformation, not after. This departs from the original paper but is now standard.

---

## 13. Dropout

Randomly zeroes out neurons during training, effectively training an **ensemble of sub-networks**:

```python
self.dropout = nn.Dropout(dropout)  # e.g., p=0.2
```

Applied at:

- After the residual connection (before adding back)
- After multi-head attention output
- After attention softmax (preventing some token-to-token communication)

At test time, everything is fully enabled — all sub-networks merge into a single ensemble.

---

## 14. The Full Transformer

### Hyperparameters (Scaled Up)

| Parameter | Value |
|-----------|-------|
| Batch size | 64 |
| Block size | 256 |
| Learning rate | 3e-4 |
| Embedding dimension | 384 |
| Number of heads | 6 |
| Head size | 64 (384 / 6) |
| Number of layers | 6 |
| Dropout | 0.2 |

### Architecture

```text
Input tokens
  → Token embeddings + Positional embeddings
  → [Multi-Head Self-Attention → Feed-Forward] × 6 layers
     (with residual connections and pre-norm LayerNorm)
  → Final LayerNorm
  → Linear projection (lm_head)
  → Softmax → Next token probabilities
```

### Results

| Model | Validation Loss |
|-------|----------------|
| Bigram (baseline) | ~2.5 |
| + Single attention head | 2.4 |
| + Multi-head attention | 2.28 |
| + Feed-forward | 2.24 |
| + Residual connections | 2.08 |
| + Layer normalization | 2.06 |
| **Full Transformer (scaled)** | **1.48** |

The scaled model generates recognizable Shakespeare-like text (nonsensical but structurally correct) after ~15 minutes on an A100 GPU.

---

## 15. Encoder vs. Decoder & Cross-Attention

### What We Built: Decoder-Only

- Triangular attention mask (autoregressive)
- No conditioning input
- Text completion: babble that looks like the training data
- Used by GPT series

### The Original Paper: Encoder-Decoder

The "Attention Is All You Need" paper was a **machine translation** paper:

```text
Encoder: French tokens → bidirectional self-attention → encoded representation
Decoder: English tokens → masked self-attention + cross-attention to encoder → translation
```

**Cross-attention**: Queries come from the decoder; keys and values come from the encoder. This conditions generation on an external source.

```text
Q = decoder tokens
K, V = encoder output
```

### Why We Skip It

We have nothing to condition on — just a text file to imitate. GPT is decoder-only. Encoder-decoder models (like T5, BART) are used for tasks that require conditioning on input.

---

## 16. Scaling Results

### Our Model vs. GPT-3

| Parameter | Our Model | GPT-3 |
|-----------|-----------|-------|
| Parameters | ~10 million | 175 billion |
| Training tokens | ~1 million | 300 billion |
| Architecture | Nearly identical | Nearly identical |
| Scale factor | 1× | ~10,000–1,000,000× |

The architecture hasn't changed significantly in 5+ years. What changed: scale (parameters, data, compute).

---

## 17. From This to ChatGPT

Training ChatGPT involves **two major stages**:

### Stage 1: Pre-Training

Exactly what we did — train a decoder-only Transformer on a massive text corpus to predict the next token. This produces a **document completer**, not an assistant. Given a question, it might respond with more questions, or try to complete a news article.

- GPT-3: 175B parameters, 300B tokens, thousands of GPUs
- This is the expensive part (months of training, millions of dollars)

### Stage 2: Fine-Tuning / Alignment

Transforms the document completer into a helpful assistant:

1. **Supervised Fine-Tuning (SFT)**: Train on curated question-answer pairs written by human labelers
1. **Reward Modeling**: Human raters rank multiple model responses to train a reward model that predicts desirability
1. **RLHF (PPO)**: Reinforcement learning optimizes the model to generate responses that score high on the reward model

This stage uses far less data (thousands of examples, not trillions of tokens) but is critical for making the model useful and safe.

> **Note**: The fine-tuning data and process are largely internal to model providers and much harder to replicate than pre-training. NanoGPT focuses on the pre-training stage only.

---

## Key Takeaways

1. **Attention is a communication mechanism**: Tokens aggregate information from other tokens in a data-dependent way
1. **Self-attention**: Keys, queries, and values all come from the same source — tokens attend to each other
1. **The Transformer block** = communicate (attention) + compute (feed-forward), repeated many times
1. **Residual connections** enable training deep networks by creating gradient superhighways
1. **Layer normalization** stabilizes training by normalizing feature distributions
1. **Scaling** (more parameters, more data, more compute) is what makes these models powerful — the architecture is essentially unchanged since 2017
1. **Pre-training** gives you a document completer; **fine-tuning** gives you an assistant

---

*Source: [Andrej Karpathy — Let's build GPT: from scratch, in code, spelled out](https://www.youtube.com/watch?v=kCc8FmEb1nY)*
