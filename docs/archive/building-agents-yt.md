# Building AI Agents That Actually Work

> From Greg Eisenberg's podcast with Remy Gasparill — a beginner-friendly course on building AI agents.
> Covers: agent architecture, context engineering, memory systems, MCP tool integration, skills, and autonomous workflows.

---

## Table of Contents

1. [Chat Models vs. Agents](#1-chat-models-vs-agents)
2. [The Agent Loop](#2-the-agent-loop)
3. [The Four Components of an Agent](#3-the-four-components-of-an-agent)
4. [Agent Harnesses](#4-agent-harnesses)
5. [Context Engineering: AGENTS.md / CLAUDE.md](#5-context-engineering-agentsmd--claudemd)
6. [Memory: The Self-Improving Loop](#6-memory-the-self-improving-loop)
7. [MCP: Connecting Tools](#7-mcp-connecting-tools)
8. [Skills: SOPs for AI](#8-skills-sops-for-ai)
9. [Building Skills Live](#9-building-skills-live)
10. [Scheduled Tasks & Autonomous Workflows](#10-scheduled-tasks--autonomous-workflows)
11. [Global vs. Project-Level Configuration](#11-global-vs-project-level-configuration)
12. [The AI Operating System (AIOS)](#12-the-ai-operating-system-aios)
13. [Recommended Learning Path](#13-recommended-learning-path)

---

## 1. Chat Models vs. Agents

The fundamental distinction:

| Mode | Input | Output | Interaction |
|------|-------|--------|-------------|
| **Chat model** | Question | Answer | Ping-pong back and forth |
| **Agent** | Goal | Result | Plans, executes, delivers autonomously |

A chat model requires you to babysit each step. An agent takes a goal, plans the work, executes it, and delivers the result — all in one go.

**Example**: "Build me a website" → agent researches, plans, codes, deploys, and delivers the finished site.

> The founders and employees utilizing agents are 10–20x more productive. That compounds over days, weeks, and years.

---

## 2. The Agent Loop

Every agent operates through a continuous loop with three stages:

```
┌─────────────┐
│   Receive    │ ← User prompt/task
│   Prompt     │
└──────┬──────┘
       ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Observe   │────▶│    Think    │────▶│     Act     │
│             │     │             │     │             │
│ Check files │     │ What's the  │     │ Research,   │
│ Read context│     │ next step?  │     │ write code, │
│ Review data │     │             │     │ call tools  │
└──────▲──────┘     └─────────────┘     └──────┬──────┘
       │                                        │
       └────────────────────────────────────────┘
              (repeat until task complete)
```

### Concrete Example: "Build a portfolio site for Greg Eisenberg"

1. **Observe**: Check if there are any files in the workspace. No context found.
2. **Think**: "I need to know who Greg Eisenberg is. I should research him."
3. **Act**: Launch a web search agent to research Greg Eisenberg.
4. **Observe**: Research results loaded. Prompt received.
5. **Think**: "I have the research. Next step: create a plan for the website."
6. **Act**: Write up the plan.
7. **Observe**: Research + plan available.
8. **Think**: "Time to write the code."
9. **Act**: Build the HTML/CSS.
10. **Observe**: Code written, needs to be served.
11. **Think**: "User wanted it spun up in preview mode."
12. **Act**: Start local server. Take screenshot. Review screenshot. Task complete.

### How the Agent Knows When to Stop

The agent concludes the task is complete based on the parameters set in your prompt. For example: "Compile 10 sources and create a report as a PowerPoint" — once both conditions are met, the loop terminates and the output is delivered.

---

## 3. The Four Components of an Agent

Every agent is made up of four things:

| Component | Role | Example |
|-----------|------|---------|
| **LLM** | The brain | Claude Opus, GPT, Gemini |
| **Loop** | Continues until done (not one-shot) | Observe → Think → Act cycles |
| **Tools** | External capabilities | Web search, Gmail, Calendar, Stripe |
| **Context** | Knowledge about you and your business | AGENTS.md, memory files, documents |

A platform that facilitates this loop — connecting the LLM, tools, and context — is called an **agent harness**. All popular AI agent platforms (Claude Code, Codex, Anti-Gravity, Co-Work, OpenClaw, Manus) are agent harnesses.

### The Car Analogy

Learning to build agents is like learning to drive. You learn the concepts — steering, pedals, brakes — and then you can drive any car. Agent harnesses are different cars. Some have better features (seat warmers, cruise control), but once you know how to drive, you can jump into any of them.

---

## 4. Agent Harnesses

Popular harnesses and their characteristics:

| Harness | Difficulty | Notes |
|---------|-----------|-------|
| **Co-Work** | Easiest | Clean UI, great for beginners |
| **Claude Code** | Easy | Best at displaying the agent loop transparently |
| **Codex** | Easy | Similar workflow to Claude Code |
| **Anti-Gravity (Gemini)** | Easy | Good UI, solid loop display |
| **Manus** | Medium | More autonomous features |
| **OpenClaw** | Hardest | Most autonomous, wild west territory |
| **Perplexity Computer** | Easy | Simple, good for basics |

### Security

- Default harnesses are secure — built by large companies with reputations to protect
- You control what privileges and tool permissions the agent has access to
- Best practice: scope what the agent can access. For risky integrations (e.g., ad budgets), give read-only access where possible
- Worst case should always be limited by permission design

---

## 5. Context Engineering: AGENTS.md / CLAUDE.md

### The Shift from Prompt Engineering to Context Engineering

**Old paradigm** (prompt engineering): Craft the perfect prompt to get good output.
**New paradigm** (context engineering): Load the agent with so much context that prompts can be simple and still produce great results.

> "Write me a cold email" with zero context produces garbage. "Write me a cold email" with a well-configured AGENTS.md produces something you can send immediately.

### What is AGENTS.md / CLAUDE.md?

A **system prompt file** — a markdown file that gives the agent context about you and your business before it does anything. It's always loaded, always on.

By platform naming convention:
- Claude Code: `CLAUDE.md`
- Gemini: `GEMINI.md`
- Codex / OpenClaw: `AGENTS.md`

But they're all the same concept.

### What to Put in It

- **Who you are** — role, business, what you do
- **Working preferences** — communication style, tools you use, how you like things structured
- **Tools and integrations** — Notion for project management, Stripe for payments, etc.
- **Client/customer information** — who you serve, your offerings
- **Behavioral rules** — tone, formatting preferences, do's and don'ts

### How to Create One

1. Open a chat model (Claude, Co-Work, etc.)
2. Ask it: "Help me build an AGENTS.md file. Ask me interview-style questions to extract all the context about me and my business."
3. Answer the questions
4. It generates the file
5. Place it in your agent's working directory

### Scaling Context with a Context Folder

If you have a lot of context, create a `context/` folder with separate files:

```
executive-assistant/
├── CLAUDE.md
├── context/
│   ├── about-me.md
│   ├── brand-voice.md
│   ├── ideal-customer-profile.md
│   └── tools-and-processes.md
└── memory.md
```

Then in your CLAUDE.md, add:
```
Before answering any questions or doing any tasks, read my context folder to understand about myself and my business.
```

By default, just having files in the folder won't load them — you need to explicitly tell the agent to read them. This instruction in CLAUDE.md strings all your context together.

**Advanced**: Some people point their CLAUDE.md to their Obsidian vault, giving the agent access to their entire second brain for context.

---

## 6. Memory: The Self-Improving Loop

### The Problem

With chat models (ChatGPT, Claude), memory is built in automatically — the model saves important things to invisible cloud memory you can't control. Every conversation informs future ones.

With agents, **you must set up memory manually**. This is actually a benefit, not a limitation. Chat model auto-memory mixes context from different companies, personal conversations, and unrelated topics — creating noise.

### The Agent Memory Problem

Without a memory system:
- You tell the agent "my favorite color is lavender" → it notes it for that session
- New session → "What's my favorite color?" → "I have no idea"
- You correct the agent's email sign-off → next session, same mistake

The agent doesn't persist learning across sessions unless you set it up.

### The Solution: memory.md

Create a `memory.md` file and add this instruction to the top of your AGENTS.md:

```markdown
Read all files in context/ and memory.md.
This is what you've learned over time.
When I correct you or you learn something new, update the relevant section in memory.md.
Keep memory.md current. When something changes, update it in place and replace outdated info.
```

Then create `memory.md` with sections for:

```markdown
## Preferences
## Corrections
## Business Rules
## Client Notes
## Process Learnings
```

### How It Works

1. You say "my favorite color is lavender"
2. The agent writes it to memory.md
3. New session → agent reads memory.md → knows your favorite color
4. As you correct the agent over time, memory.md grows
5. Errors decrease, quality compounds over weeks and months

### What Gets Saved to Memory

- **Executive assistant**: How to sign off emails, communication channel preferences, client handling rules
- **Head of marketing**: Ad formatting preferences, brand guidelines, platform-specific rules
- **Project work**: UI preferences ("don't use dark mode"), technical constraints, architecture decisions

### Memory File Size

Best practice: keep AGENTS.md around 200 lines or fewer. Memory files can grow larger, but if rules start conflicting, do a manual cleanup. You can also scope memory.md to only save "substantial corrections" rather than every tiny preference.

> Some harnesses (OpenClaw, Manus) have built-in memory systems that do this automatically under the hood — but it's the same concept: persisting learnings across sessions.

---

## 7. MCP: Connecting Tools

### What is MCP?

**Model Context Protocol** — created by Anthropic as a standardized way to connect LLMs to external tools.

### The Problem MCP Solves

Before MCP, every tool spoke a different language. Claude speaks English, Notion speaks Spanish, Gmail speaks French, your browser speaks Japanese. Connecting required extensive custom development per tool.

MCP sits as a **universal translator** between your agent and your tools. Claude speaks English, tools speak their own languages, and MCP translates between them.

```
Agent (English) ←→ MCP (Universal Translator) ←→ Tool (Any Language)
```

### Connecting Tools in Practice

Most harnesses make this simple:

- **Claude Code / Co-Work**: Connectors menu → browse hundreds of apps → sign in
- **Codex**: Settings → similar connector browser
- **Manus**: Same pattern
- **Perplexity Computer**: Connectors panel

Tools you might connect:
- **Communication**: Gmail, Slack
- **Productivity**: Google Calendar, Notion, Google Drive
- **Finance**: Stripe
- **Meeting notes**: Granola
- **Research**: Perplexity
- **Ads**: Meta Ads Manager

### Real-World Example

With Gmail, Calendar, Granola, Notion, and Stripe connected:

> "Summarize my inbox from today. Then review my meeting notes with [client] from today, draft up the email sending the proposal, create the Stripe payment link, and go into Notion and set up the project."

The agent:
1. Summarizes inbox via Gmail MCP
2. Pulls meeting notes from Granola MCP
3. Creates a Stripe payment link via Stripe MCP
4. Sets up a Notion project via Notion MCP
5. Drafts and sends email via Gmail MCP

All from one prompt, in one place, without switching apps.

### The Productivity Gain

Even simple tasks become dramatically faster when you don't have to:
1. Open Gmail
2. Find the email
3. Copy meeting notes from Granola
4. Paste into a draft
5. Open Stripe
6. Create a payment link
7. Copy the link back
8. Open Notion
9. Create the project

Instead: one prompt, all tools connected, done.

> "Even if you can do something seven times faster without switching tools, that starts to compound. You fit a week in a day, 7 weeks in a week. Stack that over a year and you're miles ahead."

---

## 8. Skills: SOPs for AI

### What Are Skills?

**Skills = Standard Operating Procedures for AI.** Once you explain a process to the agent, you package it as a skill so you never have to explain it again.

### The Problem Skills Solve

Without skills:
1. You ask the agent to create a proposal
2. Back and forth: "Change the formatting", "Use this blue", "Put the price at the bottom"
3. 15–30 minutes later, you have a good proposal
4. Next week, same process, same back and forth — the agent forgot everything

With skills:
1. Create a `proposal.md` skill file documenting the exact process
2. Next time: invoke the skill → perfect proposal immediately

### Skill File Structure

```
.claude/skills/
├── proposal/
│   ├── skill.md          # The process description
│   └── references/       # Supporting materials
│       └── proposal-template.md
├── viral-hooks/
│   ├── skill.md
│   └── references/
│       └── hook-formulas.md
└── ads-analyst/
    ├── skill.md
    └── references/
        └── analysis-framework.md
```

The `skill.md` file is essentially a detailed markdown instruction set — like a memory.md file but for a specific job-to-be-done.

### How to Create Skills

**Method 1: Upfront creation with a course/resource**
1. Upload a transcript, course, or detailed notes to the agent
2. Ask: "Use your skill creator skill to create a [topic] skill based on this material"
3. Agent packages it up with skill.md and references

**Method 2: Retrospective creation from a manual process**
1. Go through a process manually with the agent (e.g., building a proposal)
2. After it's done, say: "Create a skill for what we just did"
3. Agent packages the entire process into a reusable skill

> Most agent harnesses have a built-in **skill creator skill** — you use a skill to create skills.

### Concrete Skill Examples

| Skill | What It Does | Time Saved |
|-------|-------------|------------|
| **Viral hooks** | Generates social media hooks using proven formulas | 30 min per batch |
| **Ads analyst** | Scrapes competitor ads library, screenshots landing pages, produces deep analysis report | 3–4 hours per competitor |
| **Proposal generator** | Creates client proposals in exact preferred format | 15–30 min per proposal |
| **Sebastian referral** | Drafts referral emails to a specific contact with correct email and context | 15 min per referral |
| **Daily brief** | Summarizes calendar, inbox, and project status for the morning | 20 min per day |
| **Meeting prep** | Researches guest, compiles talking points | 30 min per meeting |
| **Weekly research** | Scrapes Twitter and Reddit for AI news, compiles newsletter | 2 hours per week |

### Skill Chaining

Skills can reference and invoke other skills:

```
Morning Brief skill:
  1. Check calendar for today's meetings
  2. If podcast guest → use Podcast Research skill
  3. If client meeting → use Meeting Prep skill
  4. Summarize inbox via Gmail MCP
  5. Compile daily brief
```

This creates complex, multi-skill workflows from simple building blocks.

---

## 9. Building Skills Live

### Example: Referral Skill

1. Manual process: "Draft an email referring Moltoshi to my friend Sebastian who runs an AI automation agency. Sebastian's email is sebastian@example.com."
2. Agent pulls meeting notes from Granola, drafts the referral email
3. After it's done: "Use your skill creator skill and create a Sebastian-refer skill so that whenever I ask you to refer someone to Sebastian, you know exactly what to do and know his email address."
4. Agent creates the skill — now reusable forever

### Example: Ads Analyst Skill

Went through a 2-hour manual process:
1. "Go to this ads library URL"
2. "Scrape all the ads"
3. "Take screenshots of all landing pages"
4. "Do a full deep dive: visual analysis, copy analysis, why it worked, what could be improved"
5. "Break down all landing pages with screenshots"
6. "Create a master report"

After completion: "Use your skill creator skill to make a skill for ads analyzing and package up the entire process."

Result: Any time you want competitor analysis, invoke the skill → full report in minutes instead of hours.

### Meta Ads Manager (OpenClaw Example)

Built an entire autonomous media buyer:
1. Created `AGENTS.md`: "You are my meta ads media buyer. You do these processes..."
2. Built ~15 skills: ad creative, copywriting, campaign optimization, audience research, etc.
3. Connected tools via MCP: Meta Ads, Dropbox, analytics
4. Combined scheduled tasks (cron) with skills and context files
5. Result: autonomous agent managing Meta ads

---

## 10. Scheduled Tasks & Autonomous Workflows

Many harnesses now support **scheduled tasks** — the agent runs on a cron-like schedule without you prompting it.

### Examples

| Schedule | Skill | What It Does |
|----------|-------|-------------|
| Every morning at 9am | Daily Brief | Summarize calendar, inbox, projects |
| Every Thursday morning | Weekly Research | Scrape Twitter/Reddit for AI news |
| Every 3 hours | Car Search | Scrape car marketplaces for specific listing criteria |
| Every morning at 9am | Morning Brief + Podcast Research | If podcast guest today, research them |

### Setting Up Scheduled Tasks

In most harnesses:
1. Create new task
2. Set the prompt: "Run my morning briefing skill"
3. Set the schedule: "Every morning at 9am"
4. Done — automated workflow running daily

---

## 11. Global vs. Project-Level Configuration

Everything (skills, context files, MCPs) can be configured at two levels:

| Level | Scope | When to Use |
|-------|-------|-------------|
| **Global** | Applies to every project/session | Universal utilities you always need |
| **Project** | Only applies within that specific folder | Role-specific or task-specific tools |

### Examples

| Item | Global | Project |
|------|--------|---------|
| **Skill: Truncate** (makes text shorter) | Yes — useful everywhere | — |
| **Skill: Sebastian referral** | — | Yes — only executive assistant needs it |
| **Skill: Ads analyst** | — | Yes — only marketing team needs it |
| **MCP: Gmail** | Yes — most agents need email | — |
| **MCP: Meta Ads** | — | Yes — only marketing agent needs it |
| **CLAUDE.md** | Global version for universal rules | Project version for role-specific instructions |

### Why It Matters

Putting a project-level skill at global level clogs up context unnecessarily. If your head of marketing doesn't need the Sebastian referral skill, don't make it global — keep it scoped to the executive assistant project.

---

## 12. The AI Operating System (AIOS)

### The Vision

Everyone will have a personal **AI operating system** — a local folder structure with:

```
workspace/
├── executive-assistant/
│   ├── CLAUDE.md
│   ├── memory.md
│   ├── context/
│   ├── skills/
│   └── MCPs (Gmail, Calendar, Notion, Stripe)
├── head-of-marketing/
│   ├── CLAUDE.md
│   ├── memory.md
│   ├── skills/
│   │   ├── ads-analyst/
│   │   ├── content-calendar/
│   │   └── competitor-research/
│   └── MCPs (Meta Ads, Analytics)
├── content-team/
│   ├── CLAUDE.md
│   ├── skills/
│   │   └── viral-hooks/
│   └── MCPs
├── head-of-sales/
│   ├── CLAUDE.md
│   ├── memory.md
│   └── skills/
└── orchestrator/
    └── CLAUDE.md (manages all departments)
```

### How It Compounds

1. **Week 1**: Set up context files. Connect basic tools.
2. **Week 2–3**: Build 3–5 skills from daily manual processes.
3. **Month 2**: Skills compound. Errors drop. Scheduled tasks automate recurring workflows.
4. **Month 3+**: Entire manual processes automated. The AIOS handles most routine work.

> "Everyone's going to have an AI operating system they work in. People won't actually use these apps anymore — they'll sit in one central place."

### The 100x Employee

Everyone will come into their role with a pre-existing AI operating system, then build out skills for all their manual processes — similar to how employees used to create SOPs, but now the SOPs execute themselves.

---

## 13. Recommended Learning Path

1. **Start with a simple harness** — Co-Work, Claude Code, or Perplexity Computer
2. **Build your AGENTS.md** — interview-style with the agent to extract context
3. **Set up memory.md** — add the self-improving loop instruction
4. **Connect your tools** — Gmail, Calendar, Notion, whatever you use daily
5. **Build skills through daily use** — every time you do a manual process, create a skill afterward
6. **Chain skills together** — reference skills within other skills for complex workflows
7. **Add scheduled tasks** — automate recurring workflows on a cron schedule
8. **Graduate to OpenClaw** — once your skills and processes are proven, migrate to more autonomous harnesses

> "You build out the skills in Claude Code first, get everything working, then migrate to OpenClaw where it has that more autonomous nature."

---

*Source: [Greg Eisenberg podcast — Building AI Agents that actually work (Full Course)](https://www.youtube.com/watch?v=eA9Zf2-qYYM)*
