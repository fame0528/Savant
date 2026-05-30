# Deep Dive into LLMs like ChatGPT

> Based on Andrej Karpathy's comprehensive lecture on large language models.
> Covers the full pipeline: pre-training, post-training, inference, tool use, and emergent model psychology.

---

## Table of Contents

1. [Pre-Training Stage](#1-pre-training-stage)
1. [Tokenization](#2-tokenization)
1. [Neural Network Training](#3-neural-network-training)
1. [Neural Network Internals](#4-neural-network-internals)
1. [Inference](#5-inference)
1. [Post-Training: From Base Model to Assistant](#6-post-training-from-base-model-to-assistant)
1. [Hallucinations & Mitigations](#7-hallucinations--mitigations)
1. [Tool Use & Web Search](#8-tool-use--web-search)
1. [Knowledge: Parameters vs. Context Window](#9-knowledge-parameters-vs-context-window)
1. [Model Psychology & Self-Knowledge](#10-model-psychology--self-knowledge)
1. [Thinking Models & Reinforcement Learning](#11-thinking-models--reinforcement-learning)
1. [Practical Takeaways](#12-practical-takeaways)

---

## 1. Pre-Training Stage

The first stage of building an LLM like ChatGPT is **pre-training** — downloading and processing the internet at massive scale.

### Data Collection

Organizations like [Hugging Face](https://huggingface.co) curate datasets such as **FineWeb**, a representative example of what production-grade training data looks like:

- **Source**: Common Crawl — an organization indexing the internet since 2007, with 2.7 billion+ web pages
- **Volume**: ~44 terabytes of disk space, ~15 trillion tokens
- **Quality**: Aggressively filtered for high-quality, diverse documents

### Data Processing Pipeline

Raw web data undergoes multiple filtering stages:

| Stage | Description |
|-------|-------------|
| **URL Filtering** | Blocklists remove malware, spam, marketing, adult, and racist domains |
| **Text Extraction** | Raw HTML is stripped to content text only — no navigation, CSS, or markup |
| **Language Filtering** | Language classifiers filter by threshold (e.g., >65% English) |
| **Deduplication** | Removes duplicate content across the dataset |
| **PII Removal** | Detects and removes personally identifiable information (addresses, SSNs) |

### Design Tradeoffs

- **Language balance**: Filtering out a language (e.g., Spanish) reduces the model's capability in that language
- **Quality vs. quantity**: High-quality sources like Wikipedia are often oversampled, leading to memorization (see [Hallucinations](#7-hallucinations--mitigations))

---

## 2. Tokenization

Neural networks require a **one-dimensional sequence of symbols** from a finite vocabulary. Tokenization converts raw text into this format.

### The Tokenization Pipeline

```text
Raw Text → UTF-8 Bytes → Byte Pair Encoding (BPE) → Token Sequence
```

**Step 1 — Bytes**: Raw text is encoded as a sequence of bytes (256 possible symbols). This trades symbol count for sequence length.

**Step 2 — Byte Pair Encoding (BPE)**: The BPE algorithm iteratively merges frequently co-occurring byte pairs into new symbols. For example, if bytes `116` and `32` frequently appear together, they merge into a single token (ID 256).

### Production Tokenizer Settings

| Model | Vocabulary Size |
|-------|----------------|
| GPT-4 | 100,277 tokens |

### Key Properties

- **Case-sensitive**: `Hello` and `hello` produce different tokens
- **Whitespace-sensitive**: `hello world` vs `hello  world` tokenizes differently
- **Context-dependent**: Common sequences compress into fewer tokens than rare ones

> Tool: Explore tokenization interactively at [tiktokenizer.vercel.app](https://tiktokenizer.vercel.app) using the `cl100k_base` tokenizer (GPT-4).

---

## 3. Neural Network Training

### The Training Loop

The core objective is **next-token prediction**: given a window of tokens, predict the token that follows.

```text
Input tokens (context) → Neural Network → Probability distribution over vocabulary
```

1. **Sample** a window of tokens from the training data (variable length, up to a max like 8,000–16,000 tokens)
1. **Forward pass**: Feed tokens through the network, producing a probability for each possible next token
1. **Compute loss**: The correct next token (the label) gets a low probability initially — the loss measures this gap
1. **Update parameters**: Adjust the network's weights so the correct token's probability increases
1. **Repeat** across millions of windows in parallel batches

### What the Network Learns

The parameters are like **knobs on a DJ set** — training discovers a setting where the network's predictions match the statistical patterns of internet text. The output is a **base model**: a token-level internet document simulator.

### Scale

| Parameter | GPT-2 (2019) | Llama 3.1 |
|-----------|-------------|-----------|
| Parameters | 1.6 billion | 405 billion |
| Context length | 1,024 tokens | ~128,000 tokens |
| Training tokens | ~100 billion | 15 trillion |
| Estimated training cost (2019) | ~$40,000 | Hundreds of millions |

Costs have dropped dramatically — GPT-2 reproduction now costs ~$100–$600 thanks to better data, faster hardware, and optimized software.

### The Compute Stack

Training requires massive GPU clusters:

```text
Single GPU → 8× GPUs per node → Multiple nodes → Data center
```

- **Hardware**: NVIDIA H100 GPUs (~$3/GPU/hour on-demand cloud pricing)
- **Why GPUs**: Matrix operations in neural networks exhibit massive parallelism — GPUs are purpose-built for this
- **At scale**: Companies like xAI deploy 100,000+ GPUs in a single data center, all collaborating to predict the next token

---

## 4. Neural Network Internals

### Architecture: The Transformer

Modern LLMs use the **Transformer** architecture — a mathematical function parameterized by billions of weights.

```text
Token Sequence → Embedding → [Attention Block → MLP Block] × N layers → Logits → Softmax → Probabilities
```

**Key components:**

- **Embedding**: Each token maps to a vector (a distributed representation)
- **Attention blocks**: Allow tokens to communicate with each other across the sequence
- **MLP blocks**: Per-token transformations
- **Layer normalization**: Stabilizes intermediate values

### What It Actually Is

- A **fixed mathematical expression** from input to output — no memory, purely stateless
- A sequence of simple operations: matrix multiplications, softmax, layer normalization
- Individual "neurons" are far simpler than biological neurons — think of it as **synthetic brain tissue**, but without dynamics or memory

> Visualization: [bbycroft.net/llm](https://bbycroft.net/llm) provides an excellent interactive diagram of the Transformer architecture.

---

## 5. Inference

Inference is the process of **generating new token sequences** from a trained model.

### The Generation Loop

```text
Prefix tokens → Network → Probability distribution → Sample token → Append → Repeat
```

1. Start with a prefix (your prompt)
1. Feed tokens into the network
1. Get a probability distribution over the next token
1. **Sample** a token (stochastic — like flipping a biased coin)
1. Append the sampled token to the sequence
1. Repeat from step 2

### Key Properties — Autoregressive Sampling

- **Stochastic**: The same prefix can produce different outputs each time
- **Remix, not copy**: Outputs are *inspired by* training data, not verbatim reproductions (usually)
- **Local coherence**: Early in training, output is gibberish; at convergence, it's coherent English

### When You Use ChatGPT

The model is already trained (parameters are fixed). You provide tokens (your prompt), and the model completes the sequence. There's no further training happening — it's **pure inference**.

---

## 6. Post-Training: From Base Model to Assistant

A base model is an **internet document simulator** — it doesn't answer questions. Post-training transforms it into a helpful assistant.

### The Conversation Format

Conversations are encoded as special token sequences:

```text
<|im_start|>user<|im_sep|>What is 2+2?<|im_end|>
<|im_start|>assistant<|im_sep|>2+2 is 4.<|im_end|>
```

Special tokens (`<|im_start|>`, `<|im_sep|>`, `<|im_end|>`) are **new tokens introduced during post-training** — they weren't in the pre-training vocabulary.

### SFT (Supervised Fine-Tuning)

1. **Create conversation datasets**: Human labelers (or LLMs with human editing) write ideal assistant responses
1. **Labeling instructions**: Companies provide guidelines — typically "helpful, truthful, harmless" with hundreds of pages of specifics
1. **Fine-tune**: Continue training the base model on these conversation datasets, swapping internet documents for conversations
1. **Short training**: Post-training takes ~3 hours vs. ~3 months for pre-training

### The SFT Dataset Ecosystem

| Era | Approach |
|-----|----------|
| **Early (InstructGPT, 2022)** | Human labelers write all responses from scratch |
| **Modern** | LLMs generate drafts; humans edit and curate at scale |

Example datasets: UltraChat (millions of mostly-synthetic conversations spanning diverse topics).

### What You're Actually Talking To

> You're not talking to a magical AI. You're talking to a **statistical simulation of a human labeler** — someone hired by the model provider, following company-written labeling instructions, who would have written the ideal response in this situation.

---

## 7. Hallucinations & Mitigations

### Why Hallucinations Happen

In the training set, questions of the form "Who is X?" are **always answered confidently**. The model learns this pattern: "who is" → confident answer. When asked about someone who doesn't exist, it still produces a confident (but fabricated) response because that's the statistical pattern it learned.

### Mitigation 1: Knowledge Boundary Probing

Used by Meta for Llama 3:

1. **Generate questions** from documents in the training set (using an LLM)
1. **Interrogate the model**: Ask it the same question multiple times
1. **Compare answers** to ground truth using an LLM judge
1. **If the model doesn't know**: Add training examples where "I don't know" is the correct response

This teaches the model to recognize its own uncertainty — wiring internal confidence signals to verbalized refusal.

### Mitigation 2: Tool Use (see next section)

Instead of just refusing, the model can **look up** information it doesn't know.

---

## 8. Tool Use & Web Search

### The Mechanism

Models emit **special tokens** to invoke tools:

```text
<|search_start|>query text<|search_end|>
```

When the inference engine encounters `<|search_end|>`:

1. Pauses generation
1. Executes the search (e.g., via Bing)
1. Pastes the results into the context window
1. Resumes generation with the search results available

### Training Tool Use

A few thousand conversation examples demonstrating correct tool usage teach the model when and how to invoke tools. The model's pre-existing understanding of "what a web search is" makes this surprisingly effective with minimal examples.

### In Practice (ChatGPT)

When ChatGPT encounters a question it can't confidently answer from memory, it:

1. Emits search tokens
1. Retrieves web results
1. Cites sources in its response

---

## 9. Knowledge: Parameters vs. Context Window

This is one of the most important mental models for working with LLMs:

| Storage | Analogy | Properties |
|---------|---------|------------|
| **Parameters** (weights) | Vague recollection — something you read a month ago | Compressed, lossy, probabilistic |
| **Context window** (tokens) | Working memory — what you just experienced | Direct access, high fidelity |

### Practical Implication

**Always provide the source material in the prompt.** Instead of asking the model to recall from its parameters, paste the relevant text directly into the context window.

Example:
```text
# Weak prompt (relies on parameter memory)
"Summarize Chapter 1 of Pride and Prejudice"

# Strong prompt (uses context window)
"Summarize Chapter 1 of Pride and Prejudice.
I'm attaching it below for your reference.
---
[paste Chapter 1 here]"
```

The second prompt produces significantly higher quality output because the model has **direct access** rather than relying on compressed recollection.

---

## 10. Model Psychology & Self-Knowledge

### No Persistent Self

LLMs have **no persistent existence**. Each conversation:

- Boots up
- Processes tokens
- Shuts off

There's no continuous "self" — asking "who are you?" produces statistically plausible but often fabricated self-descriptions.

### Self-Identity Confabulation

When a base model says "I was built by OpenAI" or "I am ChatGPT," it's not because it was trained on OpenAI's data. It's because:

- During pre-training, it saw many conversations where the assistant identified as ChatGPT
- During SFT, it took on a generic "helpful assistant" persona
- It doesn't have a ground-truth label for its own identity — it makes one up

### Overriding Self-Knowledge

Developers can override self-identity through:

- **System prompts**: Explicitly define who the model is
- **Training data**: Include self-identification examples specific to the deployment
- **Fine-tuning**: Train on conversations with the desired identity

---

## 11. Thinking Models & Reinforcement Learning

Models like o3-mini represent a significant departure from pure imitation:

### How They Differ

| Aspect | Standard LLM (GPT-4) | Thinking Model (o3-mini) |
|--------|----------------------|--------------------------|
| Training | SFT + light RLHF | SFT + extensive RL on verifiable problems |
| Behavior | Imitates human labelers | Develops emergent thinking strategies |
| Reasoning | Straightforward pattern matching | Internal chain-of-thought reasoning |
| Novelty | Remixes training data | Can produce genuinely novel reasoning (analogous to AlphaGo's "Move 37") |

### Open Questions

- Do thinking strategies developed in **verifiable domains** (math, code) **transfer** to unverifiable domains (creative writing)?
- How far can RL-based reasoning go in open-domain problem solving?

These models are still **primordial** — they represent the early hints of a fundamentally new capability, but they're early.

---

## 12. Practical Takeaways

1. **LLMs are tools, not oracles.** They're statistical systems that will randomly hallucinate, fail at mental arithmetic, or can't count letters.

1. **Always verify.** Use LLMs for inspiration and first drafts, but check their work and own the product.

1. **Provide context, don't rely on recall.** Paste source material into the context window rather than asking the model to remember from its parameters.

1. **Understand the training set bias.** The model's behavior reflects what human labelers would produce following the company's labeling instructions.

1. **Tools amplify capability.** Web search, code execution, and other tools compensate for the model's inability to access current information.

1. **Thinking models are exciting but early.** They show genuine reasoning capability in verifiable domains, but transfer to open-ended tasks remains uncertain.

---

*Source: [Andrej Karpathy — Deep Dive into LLMs like ChatGPT](https://www.youtube.com/watch?v=7xTGNNLPyMI)*
