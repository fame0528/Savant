# LLM Parameters Guide

This guide explains the AI model parameters you can configure for each agent in Savant. These settings control how the AI responds to messages.

## Quick Reference

| Parameter | What It Does | Recommended Default |
|-----------|--------------|---------------------|
| Temperature | How creative the AI is | 0.7 |
| Top-P | How many word choices to consider | 1.0 |
| Frequency Penalty | Discourages repeating words | 0.0 |
| Presence Penalty | Encourages exploring new topics | 0.0 |
| Max Tokens | How long responses can be | 4096 |
| Stop Sequences | Words that make the AI stop | (empty) |

## Detailed Explanations

### Temperature (0.0 - 2.0)

**What it does:** Controls how creative or predictable the AI's responses are.

**Simple explanation:** Think of it like a "creativity dial":
- **Low (0.0-0.3):** The AI gives very predictable, safe answers. Good for coding, math, or factual information.
- **Medium (0.4-0.7):** Balanced between being reliable and interesting. Good for most conversations.
- **High (0.8-2.0):** The AI takes more risks with its word choices. More creative but may make mistakes.

**Common use cases:**
- **Coding/Math/Science:** 0.0-0.3 (you want correct answers)
- **General Chat:** 0.5-0.7 (balanced)
- **Creative Writing:** 0.8-1.2 (more variety)
- **Brainstorming:** 1.0-1.5 (maximum creativity)

---

### Top-P (0.0 - 1.0)

**What it does:** Limits which words the AI is allowed to consider when generating responses.

**Simple explanation:** 
- **1.0:** The AI considers all possible words (default, most common setting)
- **0.5:** The AI only considers the top 50% most likely words
- **0.1:** The AI only considers the top 10% most likely words (very safe)

**When to adjust this:**
Usually, you should adjust **Temperature** OR **Top-P**, but not both. If you're already changing Temperature, leave Top-P at 1.0.

**Common use cases:**
- **Standard usage:** 1.0 (leave it alone)
- **Very focused answers:** 0.5-0.9
- **Maximum safety:** 0.1-0.5

---

### Frequency Penalty (-2.0 - 2.0)

**What it does:** Makes the AI less likely to repeat words it has already used.

**Simple explanation:** If the AI keeps saying the same word or phrase over and over, increase this value. It's like telling the AI "use more variety in your vocabulary."

**Common use cases:**
- **Normal conversation:** 0.0 (default)
- **If AI repeats itself too much:** 0.3-0.8
- **Creative writing with varied language:** 0.8-1.5

**Note:** Setting this too high (above 1.0) can make responses feel unnatural.

---

### Presence Penalty (-2.0 - 2.0)

**What it does:** Encourages the AI to talk about new topics instead of staying on the same subject.

**Simple explanation:** This is different from Frequency Penalty. While Frequency Penalty stops the AI from repeating the *same words*, Presence Penalty encourages it to explore *completely new ideas*.

**Common use cases:**
- **Normal conversation:** 0.0 (default)
- **Brainstorming multiple ideas:** 0.3-0.8
- **Exploring many different topics:** 0.8-1.5

**Note:** Setting this too high may make the conversation feel unfocused or scattered.

---

### Max Tokens (1 - 128,000)

**What it does:** Limits how long the AI's response can be.

**Important:** One "token" is approximately 3/4 of a word in English. So:
- 1,000 tokens ≈ 750 words
- 4,096 tokens ≈ 3,000 words (about 6 pages)
- 16,384 tokens ≈ 12,000 words (about 24 pages)
- 131,072 tokens ≈ 98,000 words (about 200 pages)
- 1,000,000 tokens ≈ 750,000 words (about 1,500 pages)

**Common use cases:**
- **Short answers/chat:** 1,024-2,048 tokens
- **Normal responses:** 4,096 tokens (default)
- **Detailed explanations:** 8,192-16,384 tokens
- **Long documents/reports:** 32,768-131,072 tokens
- **Massive outputs (1M models):** 500,000+ tokens

**Note:** Longer responses:
- Take more time to generate
- Cost more to process
- May not be supported by all AI models (check your model's limits)

---

### Stop Sequences

**What it does:** Tells the AI to stop generating text when it encounters specific words or phrases.

**Simple explanation:** You can list words that will make the AI immediately stop writing. This is useful for controlling the format of responses.

**Examples:**
- `\n` - Stop at a new line
- `END` - Stop when it writes "END"
- `###` - Stop at section markers

**When to use this:**
Most users should leave this empty (default). Only use stop sequences if you need very specific formatting control.

**How to add multiple sequences:**
Enter each stop sequence on a separate line in the configuration.

---

## Recommended Settings by Use Case

### For Factual Q&A / Research
```
Temperature: 0.2
Top-P: 1.0
Frequency Penalty: 0.0
Presence Penalty: 0.0
Max Tokens: 4096
```

### For Creative Writing
```
Temperature: 0.9
Top-P: 1.0
Frequency Penalty: 0.5
Presence Penalty: 0.3
Max Tokens: 8192
```

### For Coding / Technical Tasks
```
Temperature: 0.1
Top-P: 1.0
Frequency Penalty: 0.0
Presence Penalty: 0.0
Max Tokens: 8192
```

### For Brainstorming
```
Temperature: 1.0
Top-P: 1.0
Frequency Penalty: 0.3
Presence Penalty: 0.5
Max Tokens: 4096
```

### For Customer Support
```
Temperature: 0.4
Top-P: 1.0
Frequency Penalty: 0.1
Presence Penalty: 0.0
Max Tokens: 2048
```

---

## Frequently Asked Questions

**Q: Should I change multiple parameters at once?**
A: Start by changing only Temperature. Most other parameters are fine at their defaults. Only adjust Frequency Penalty or Presence Penalty if you notice specific problems (like repetition).

**Q: What happens if I set Temperature to 0?**
A: The AI becomes very predictable and may give the same answer to the same question every time. This is good for tasks requiring consistency.

**Q: What happens if I set Temperature above 1.0?**
A: The AI becomes very creative but may produce nonsensical or incorrect responses. Use with caution.

**Q: Do these settings work with all AI models?**
A: Most settings work with all major AI providers (OpenAI, Anthropic, Google, etc.). However, some models may have different limits or may ignore certain parameters.

**Q: Will changing these settings cost more money?**
A: No, these settings don't affect pricing directly. However, higher Max Tokens means longer responses, which use more tokens and cost more.
