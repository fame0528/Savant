use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use tracing::info;

use savant_memory::engine::MemoryEnclave;
use savant_memory::models::{AgentMessage, MemoryEntry, MessageRole};

use crate::config::ObsidianConfig;
use crate::error::VaultError;

const MAX_CONTENT_PREVIEW: usize = 400;

pub struct VaultWriter {
    vault_path: PathBuf,
    enclave: Option<Arc<MemoryEnclave>>,
    config: ObsidianConfig,
    agent_name: String,
}

impl VaultWriter {
    pub fn new(
        vault_path: PathBuf,
        enclave: Option<Arc<MemoryEnclave>>,
        config: ObsidianConfig,
        agent_name: String,
    ) -> Self {
        Self {
            vault_path,
            enclave,
            config,
            agent_name,
        }
    }

    pub fn ensure_structure(&self) -> Result<(), VaultError> {
        let dirs = [
            &self.vault_path,
            &self.vault_path.join(".obsidian"),
            &self.vault_path.join("Episodic"),
            &self.vault_path.join("Semantic"),
            &self.vault_path.join("Identity").join("Evolution"),
            &self.vault_path.join("Themes"),
            &self.vault_path.join("Working"),
            &self.vault_path.join("Dashboard"),
            &self.vault_path.join(".stale"),
        ];
        for dir in &dirs {
            fs::create_dir_all(dir)?;
        }

        let appearance = self.vault_path.join(".obsidian").join("appearance.json");
        if !appearance.exists() {
            let json = "{\"accentColor\":\"#00FFBB\",\"baseTheme\":\"obsidian\",\"interfaceFontFamily\":\"Inter\",\"textFontFamily\":\"Inter\",\"monospaceFontFamily\":\"JetBrains Mono\",\"translucency\":false,\"native\":false,\"enabledCssSnippets\":[],\"cssTheme\":\"\"}";
            atomic_write(&appearance, json)?;
        }

        let stale_gi = self.vault_path.join(".stale").join(".gitignore");
        if !stale_gi.exists() {
            atomic_write(&stale_gi, "*\n")?;
        }
        Ok(())
    }

    // ─── INDEX ────────────────────────────────────────────────────────────

    pub fn write_index(&self, stats: &VaultStats) -> Result<(), VaultError> {
        let today = Utc::now().format("%Y-%m-%d");
        let t = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let content = format!(
            "# {name}'s Memory Tree\n\n\
             > *Last synced: {t}*\n\
             > *Agent stage: {stage} | Evolution score: {score} | \
             {sessions} sessions, {memories} memories, {vectors} vectors*\n\n\
             ---\n\n\
             ## Episodic\n\n\
             Recent: [[Episodic/{today}]] | All sessions stored in LSM+HNSW\n\n\
             ## Semantic\n\n\
             Concepts: [[Semantic/Concepts]] | Relations: [[Semantic/Relations]]\n\
             Triplets: [[Semantic/Triplets]] | Entities: [[Semantic/Entities]]\n\n\
             ## Identity\n\n\
             SOUL: [[Identity/SOUL]] | Personality: [[Identity/Personality]]\n\
             Evolution: [[Identity/Evolution]]\n\n\
             ## Themes\n\n\
             [[Themes/]]\n\n\
             ## Dashboard\n\n\
             Recent: [[Dashboard/Recent]] | Health: [[Dashboard/Health]]\n\n\
             ---\n\n\
             *This vault is a bidirectional projection of Savant's LSM+HNSW memory substrate. \
             Edits to Semantic/ and Identity/ files are synced back to the agent. \
             Edits to Episodic/ are logged as correction nodes.*\n",
            name = self.agent_name, t = t, today = today,
            stage = stats.stage, score = stats.evolution_score,
            sessions = stats.session_count, memories = stats.memory_count,
            vectors = stats.vector_count,
        );
        atomic_write(&self.vault_path.join("INDEX.md"), &content)
    }

    // ─── EPISODIC ─────────────────────────────────────────────────────────

    pub fn write_episodic(&self, date: &NaiveDate) -> Result<(), VaultError> {
        let filename = format!("{}.md", date.format("%Y-%m-%d"));
        let path = self.vault_path.join("Episodic").join(&filename);

        let date_start = date.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp()).unwrap_or(0);
        let date_end = date_start + 86400;

        let mut content = format!(
            "# Episodic — {date}\n\n> Daily transcript. Append-only. \
             Edited at {ts}.\n\n---\n\n",
            date = date.format("%Y-%m-%d"),
            ts = Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        );

        let mut session_map: BTreeMap<String, Vec<AgentMessage>> = BTreeMap::new();
        if let Some(enclave) = &self.enclave {
            let lsm = enclave.lsm();
            for sid in lsm.session_keys() {
                let msgs = lsm.fetch_session_tail(&sid, 200);
                let day_msgs: Vec<AgentMessage> = msgs
                    .into_iter()
                    .filter(|m| {
                        let ts: i64 = m.timestamp.into();
                        ts >= date_start && ts < date_end
                    })
                    .collect();
                if !day_msgs.is_empty() {
                    session_map.insert(sid.clone(), day_msgs);
                }
            }
        }

        if session_map.is_empty() {
            content.push_str("*No sessions recorded for this date.*\n");
            atomic_write(&path, &content)?;
            return Ok(());
        }

        content.push_str(&format!("**{n} session(s)**\n\n", n = session_map.len()));

        for (sid, msgs) in &session_map {
            content.push_str(&format!(
                "### Session: `{sid}`\n\n| Role | Content |\n|------|--------|\n"
            ));
            for msg in msgs {
                let role = match msg.role {
                    MessageRole::User => "**You**",
                    MessageRole::Assistant => "**Agent**",
                    MessageRole::Tool => "*Tool*",
                    MessageRole::System => "System",
                };
                let truncated: String = msg.content.chars().take(MAX_CONTENT_PREVIEW).collect();
                let ellipsis = if msg.content.len() > MAX_CONTENT_PREVIEW { "…" } else { "" };
                let escaped = truncate_to_line(truncated.trim()).replace('|', "\\|");
                content.push_str(&format!("| {role} | {escaped}{ellipsis} |\n"));
                for tc in &msg.tool_calls {
                    content.push_str(&format!(
                        "| → | `{name}(…{args}…)` |\n",
                        name = tc.tool_name,
                        args = truncate_to_line(&tc.arguments.chars().take(80).collect::<String>()),
                    ));
                }
            }
            content.push('\n');
        }

        atomic_write(&path, &content)
    }

    // ─── SEMANTIC ─────────────────────────────────────────────────────────

    pub fn write_concepts(&self) -> Result<(), VaultError> {
        let path = self.vault_path.join("Semantic").join("Concepts.md");
        let mut content = String::from(
            "# Concept Graph\n\n> Concepts are distilled from agent interactions. \
             [[Semantic/Relations]] | [[Semantic/Entities]] | [[Semantic/Triplets]]\n\n---\n\n",
        );

        let mut by_category: BTreeMap<String, Vec<MemoryEntry>> = BTreeMap::new();
        if let Some(enclave) = &self.enclave {
            if let Ok(entries) = enclave.lsm().iter_metadata() {
                for entry in entries {
                    let cat = if entry.category.is_empty() {
                        "uncategorized".to_string()
                    } else {
                        entry.category.clone()
                    };
                    by_category.entry(cat).or_default().push(entry);
                }
            }
        }

        if by_category.is_empty() {
            content.push_str("*No concepts extracted yet. Concepts are generated by the DistillationPipeline during background consolidation.*\n");
            atomic_write(&path, &content)?;
            return Ok(());
        }

        content.push_str("## Concepts by Category\n\n");
        for (cat, entries) in &by_category {
            content.push_str(&format!("### {cat}\n\n| Concept | Importance | Mentions | Source |\n|--------|-----------|----------|--------|\n"));
            for entry in entries {
                let snippet: String = entry.content.chars().take(100).collect();
                let ellipsis = if entry.content.len() > 100 { "…" } else { "" };
                let anchor = slugify(&snippet.chars().take(30).collect::<String>());
                content.push_str(&format!(
                    "| [[Semantic/Concepts#{anchor}|{snippet}{ellipsis}]] | {imp} | {hits} | [[Episodic/]] |\n",
                    anchor = anchor,
                    snippet = truncate_to_line(&snippet),
                    imp = entry.importance,
                    hits = u32::from(entry.hit_count),
                ));
            }
            content.push('\n');
        }

        atomic_write(&path, &content)
    }

    pub fn write_relations(&self) -> Result<(), VaultError> {
        let path = self.vault_path.join("Semantic").join("Relations.md");
        let mut content = String::from(
            "# Relation Graph\n\n> Typed edges between concepts. \
             [[Semantic/Concepts]] | [[Semantic/Entities]] | [[Semantic/Triplets]]\n\n---\n\n",
        );

        let mut relation_count = 0usize;
        if let Some(enclave) = &self.enclave {
            if let Ok(entries) = enclave.lsm().iter_metadata() {
                for entry in &entries {
                    if !entry.related_to.is_empty() {
                        relation_count += entry.related_to.len();
                    }
                }
            }
        }

        if relation_count == 0 {
            content.push_str("*No relations discovered yet. Relations emerge from shared session context and the DistillationPipeline.*\n");
            atomic_write(&path, &content)?;
            return Ok(());
        }

        content.push_str("## Ontology\n\n| Category | Relation Types |\n|----------|---------------|\n\
                          | Hierarchical | is_a, part_of, subclass_of |\n\
                          | Social | works_for, knows, founded, advises |\n\
                          | Temporal | superseded_by, evolved_into, prior_state |\n\
                          | Epistemic | contradicts, supports, derived_from |\n\
                          | Operational | requires, generates, modifies |\n\n");

        if let Some(enclave) = &self.enclave {
            if let Ok(entries) = enclave.lsm().iter_metadata() {
                content.push_str("## Active Edges\n\n| Source | Relation | Target |\n|--------|----------|--------|\n");
                for entry in &entries {
                    for rel_id in &entry.related_to {
                        let rel_u64: u64 = (*rel_id).into();
                        if let Ok(Some(target)) = enclave.lsm().get_metadata(rel_u64) {
                            let src_snippet: String = entry.content.chars().take(60).collect();
                            let tgt_snippet: String = target.content.chars().take(60).collect();
                            content.push_str(&format!(
                                "| {src}… | related_to | {tgt}… |\n",
                                src = truncate_to_line(&src_snippet),
                                tgt = truncate_to_line(&tgt_snippet),
                            ));
                        }
                    }
                }
            }
        }

        atomic_write(&path, &content)
    }

    pub fn write_triplets(&self) -> Result<(), VaultError> {
        let path = self.vault_path.join("Semantic").join("Triplets.md");
        let mut content = String::from(
            "# Distilled Knowledge (Triplets)\n\n> Subject-predicate-object extractions. \
             [[Semantic/Concepts]] | [[Semantic/Relations]] | [[Semantic/Entities]]\n\n---\n\n",
        );

        if let Some(enclave) = &self.enclave {
            let facts = enclave.lsm().iter_facts();
            if !facts.is_empty() {
                content.push_str("| Subject | Predicate | Object | Confidence |\n");
                content.push_str("|---------|-----------|--------|------------|\n");
                for (subj, pred, obj, conf) in &facts {
                    content.push_str(&format!("| {subj} | {pred} | {obj} | {conf:.2} |\n"));
                }
                content.push('\n');
                content.push_str(&format!("**{n} triplets** extracted by the DistillationPipeline.\n", n = facts.len()));
            } else {
                content.push_str("*No triplets extracted yet. Triplets are generated by the DistillationPipeline.*\n");
            }
        } else {
            content.push_str("*No triplets extracted yet. Triplets are generated by the DistillationPipeline.*\n");
        }

        atomic_write(&path, &content)
    }

    pub fn write_entities(&self) -> Result<(), VaultError> {
        let path = self.vault_path.join("Semantic").join("Entities.md");
        let mut content = String::from(
            "# Entity Catalog\n\n> People, projects, services, and tools tracked across sessions. \
             [[Semantic/Concepts]] | [[Semantic/Relations]]\n\n---\n\n",
        );

        let mut entities: BTreeMap<String, (u32, u32, i64, i64)> = BTreeMap::new();
        if let Some(enclave) = &self.enclave {
            let lsm = enclave.lsm();
            for sid in lsm.session_keys() {
                let msgs = lsm.fetch_session_tail(&sid, 100);
                for msg in &msgs {
                    extract_entities_from_text(&msg.content, &mut entities);
                    for tc in &msg.tool_calls {
                        extract_entities_from_text(&tc.tool_name, &mut entities);
                        extract_entities_from_text(&tc.arguments, &mut entities);
                    }
                }
            }
        }

        if entities.is_empty() {
            content.push_str("*No entities extracted yet. Entity extraction uses Schema.org/FOAF patterns with LLM fallback.*\n");
        } else {
            content.push_str("| Entity | Type | Mentions | Sessions | First Seen | Last Seen |\n");
            content.push_str("|--------|------|----------|----------|------------|-----------|\n");
            for (name, (count, sessions, first, last)) in &entities {
                let first_dt = chrono::DateTime::from_timestamp(*first, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                let last_dt = chrono::DateTime::from_timestamp(*last, 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                content.push_str(&format!(
                    "| {name} | entity | {count} | {sessions} | {first_dt} | {last_dt} |\n"
                ));
            }
        }

        atomic_write(&path, &content)
    }

    // ─── IDENTITY ─────────────────────────────────────────────────────────

    pub fn write_soul(&self, workspace_root: &Path) -> Result<(), VaultError> {
        let target = self.vault_path.join("Identity").join("SOUL.md");
        let source = workspace_root.join("SOUL.md");
        if source.exists() {
            atomic_write(&target, &fs::read_to_string(&source)?)?;
        } else {
            let content = format!(
                "# SOUL.md — {name}\n\n> No SOUL.md found. Use the Evolution system \
                 to generate one.\n\n## Terminal Mantra\n\nOperate with precision, \
                 security, and autonomy.\n",
                name = self.agent_name,
            );
            atomic_write(&target, &content)?;
        }
        Ok(())
    }

    pub fn write_personality(&self, workspace_root: &Path) -> Result<(), VaultError> {
        let path = self.vault_path.join("Identity").join("Personality.md");
        let agent_json_path = workspace_root.join("agent.json");

        let mut o = "—".to_string();
        let mut c = "—".to_string();
        let mut e = "—".to_string();
        let mut a = "—".to_string();
        let mut n = "—".to_string();

        if agent_json_path.exists() {
            if let Ok(content) = fs::read_to_string(&agent_json_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(pt) = val.get("personality_traits") {
                        o = pt.get("openness").and_then(|v| v.as_f64()).map(|v| format!("{:.2}", v)).unwrap_or_else(|| o);
                        c = pt.get("conscientiousness").and_then(|v| v.as_f64()).map(|v| format!("{:.2}", v)).unwrap_or_else(|| c);
                        e = pt.get("extraversion").and_then(|v| v.as_f64()).map(|v| format!("{:.2}", v)).unwrap_or_else(|| e);
                        a = pt.get("agreeableness").and_then(|v| v.as_f64()).map(|v| format!("{:.2}", v)).unwrap_or_else(|| a);
                        n = pt.get("neuroticism").and_then(|v| v.as_f64()).map(|v| format!("{:.2}", v)).unwrap_or_else(|| n);
                    }
                }
            }
        }

        let content = format!(
            "# Personality — OCEAN Traits\n\n> **{name}** — [[Identity/Evolution]] | [[Identity/SOUL]]\n\n\
             ---\n\n## Current Scores\n\n| Trait | Score | Range |\n|-------|-------|-------|\n\
             | **Openness** | {o} | 0.0–1.0 |\n\
             | **Conscientiousness** | {c} | 0.0–1.0 |\n\
             | **Extraversion** | {e} | 0.0–1.0 |\n\
             | **Agreeableness** | {a} | 0.0–1.0 |\n\
             | **Neuroticism** | {n} | 0.0–1.0 |\n\n\
             Personality is managed through the Evolution system. Mutations approved \
             via the dashboard update these traits.\n",
            name = self.agent_name, o = o, c = c, e = e, a = a, n = n,
        );
        atomic_write(&path, &content)
    }

    pub fn write_evolution_index(&self, workspace_root: &Path) -> Result<(), VaultError> {
        let path = self.vault_path.join("Identity").join("Evolution").join("INDEX.md");
        let evolution_path = workspace_root.join("EVOLUTION.jsonl");
        let agent_json_path = workspace_root.join("agent.json");

        let mutations: Vec<serde_json::Value> = if evolution_path.exists() {
            fs::read_to_string(&evolution_path)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        } else {
            Vec::new()
        };

        let (stage, score, _mutation_count) = if agent_json_path.exists() {
            fs::read_to_string(&agent_json_path)
                .ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .and_then(|v| v.get("evolution_state").cloned())
                .map(|es| {
                    let count = es.get("mutation_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let score = es.get("evolution_score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let stage = es.get("stage").and_then(|v| v.as_str()).unwrap_or("Seedling").to_string();
                    (stage, score, count)
                })
                .unwrap_or_else(|| ("Seedling".to_string(), 0.0, 0))
        } else {
            ("Seedling".to_string(), 0.0, 0)
        };

        let approved_count = mutations.iter()
            .filter(|m| m.get("status").and_then(|v| v.as_str()) == Some("approved"))
            .count();

        let mut content = format!(
            "# Evolution Timeline\n\n> **{name}** \u{2014} [[Identity/Personality]] | [[Identity/SOUL]]\n\n             ---\n\n## Status\n\n| Metric | Value |\n|--------|-------|
             | **Stage** | {stage} |\n| **Score** | {score:.2} |\n             | **Total Mutations** | {total} |\n             | **Approved** | {approved} |\n             | **Pending** | {pending} |\n\n",
            name = self.agent_name,
            stage = stage,
            score = score,
            total = mutations.len(),
            approved = approved_count,
            pending = mutations.len() - approved_count,
        );

        let reports_dir = self.vault_path.join("Identity").join("Evolution").join("reports");
        let _ = fs::create_dir_all(&reports_dir);
        for m in &mutations {
            if let Err(e) = self.write_mutation_report(m, &reports_dir) {
                tracing::warn!("[obsidian] Failed to write mutation report: {}", e);
            }
        }

        if mutations.is_empty() {
            content.push_str("*No mutations recorded. The Evolution system proposes SOUL.md edits based on interaction patterns.*\n");
        } else {
            content.push_str("## Timeline\n\n| Date | Type | Section | Status | Evidence |\n|------|------|---------|--------|----------|\n");
            for m in &mutations {
                let ts = m.get("proposed_at").and_then(|v| v.as_i64()).unwrap_or(0);
                let date = if ts > 0 {
                    chrono::DateTime::from_timestamp(ts / 1000, 0)
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "\u{2014}".to_string())
                } else {
                    "\u{2014}".to_string()
                };
                let mtype = m.get("mutation_type").and_then(|v| v.as_str()).unwrap_or("\u{2014}");
                let section = m.get("target_section").and_then(|v| v.as_str()).unwrap_or("\u{2014}");
                let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
                let report_id = m.get("mutation_id").and_then(|v| v.as_str()).unwrap_or("");
                let evidence_count = m.get("source_evidence")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                let evidence_link = if evidence_count > 0 {
                    format!("[[reports/{}|{} evidence]]", report_id, evidence_count)
                } else {
                    "\u{2014}".to_string()
                };
                content.push_str(&format!(
                    "| {} | {} | {} | {} | {} |\n",
                    date, mtype, section, status, evidence_link
                ));
            }
        }

        atomic_write(&path, &content)
    }

    fn write_mutation_report(&self, mutation: &serde_json::Value, reports_dir: &Path) -> Result<(), VaultError> {
        let report_id = mutation.get("mutation_id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let path = reports_dir.join(format!("{}.md", report_id));

        let mtype = mutation.get("mutation_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let section = mutation.get("target_section").and_then(|v| v.as_str()).unwrap_or("unknown");
        let status = mutation.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
        let confidence = mutation.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let reasoning = mutation.get("reasoning").and_then(|v| v.as_str()).unwrap_or("");
        let proposed = mutation.get("proposed_content").and_then(|v| v.as_str()).unwrap_or("");
        let before = mutation.get("before_content").and_then(|v| v.as_str()).unwrap_or("");

        let ts = mutation.get("proposed_at").and_then(|v| v.as_i64()).unwrap_or(0);
        let date = if ts > 0 {
            chrono::DateTime::from_timestamp(ts / 1000, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "\u{2014}".to_string())
        } else {
            "\u{2014}".to_string()
        };

        let evidence: Vec<String> = mutation.get("source_evidence")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(String::from).collect())
            .unwrap_or_default();

        let mut content = format!(
            "# Mutation Report \u{2014} {report_id}\n\n             > **Type:** {mtype} | **Section:** {section} | **Status:** {status} | **Confidence:** {confidence:.2}\n             > **Proposed:** {date}\n\n             ---\n\n             ## Reasoning\n\n             {reasoning}\n\n",
            report_id = report_id, mtype = mtype, section = section, status = status,
            confidence = confidence, date = date, reasoning = reasoning,
        );

        if !before.is_empty() {
            content.push_str("## Diff\n\n```diff\n");
            content.push_str(&format!("- {}\n", before.replace('\n', "\n- ")));
            content.push_str(&format!("+ {}\n", proposed.replace('\n', "\n+ ")));
            content.push_str("```\n\n");
        } else {
            content.push_str("## Proposed Content\n\n");
            content.push_str(proposed);
            content.push_str("\n\n");
        }

        if !evidence.is_empty() {
            content.push_str("## Evidence\n\n");
            for ev in &evidence {
                content.push_str(&format!("- [[Episodic/{}]]\n", ev));
            }
            content.push('\n');
        }

        content.push_str("## Navigation\n\n[[Identity/Evolution]] | [[Identity/SOUL]] | [[Identity/Personality]]\n");

        atomic_write(&path, &content)
    }

    pub fn write_themes_index(&self) -> Result<(), VaultError> {
        let path = self.vault_path.join("Themes").join("INDEX.md");
        let content = "# Themes — Cross-Domain Patterns\n\n> Concept clusters discovered by the Dream Engine \
             during REM cycles. [[Semantic/Concepts]] | [[Semantic/Relations]]\n\n---\n\n\
             *Themes are generated during Dream REM consolidation. The engine explores HNSW latent \
             space to find disparate concept nodes, abstracts them into higher-level themes, and ranks \
             clusters by Vendi Score. No clusters found yet.*\n".to_string();
        atomic_write(&path, &content)
    }

    // ─── WORKING ──────────────────────────────────────────────────────────

    pub fn clear_working(&self) -> Result<(), VaultError> {
        let working = self.vault_path.join("Working");
        if working.exists() {
            for entry in fs::read_dir(&working)? {
                let path = entry?.path();
                if path.is_file() {
                    fs::remove_file(&path)?;
                }
            }
        }
        let readme = working.join("README.md");
        atomic_write(&readme, "# Working\n\n> Transient scratchpad. Cleared on task completion.\n")
    }

    // ─── DASHBOARD ────────────────────────────────────────────────────────

    pub fn write_dashboard_recent(&self) -> Result<(), VaultError> {
        let path = self.vault_path.join("Dashboard").join("Recent.md");
        let today = Utc::now().format("%Y-%m-%d");
        let _cutoff = Utc::now().timestamp() - 86400;
        let mut session_count = 0u64;
        let mut msg_count = 0u64;

        if let Some(enclave) = &self.enclave {
            let lsm = enclave.lsm();
            let recent = lsm.iter_recent_messages(24);
            let mut sessions_seen: Vec<String> = Vec::new();
            for msg in &recent {
                msg_count += 1;
                if !sessions_seen.contains(&msg.session_id) {
                    sessions_seen.push(msg.session_id.clone());
                }
            }
            session_count = sessions_seen.len() as u64;
        }

        let content = format!(
            "# Recent Activity\n\n> Last 24 hours — {date}\n> [[Dashboard/Health]]\n\n---\n\n\
             ## Summary\n\n| Metric | Value |\n|--------|-------|\n\
             | **Active Sessions** | {sessions} |\n| **Messages** | {messages} |\n\n\
             Full session data: [[Episodic/{date}]]\n",
            date = today, sessions = session_count, messages = msg_count,
        );
        atomic_write(&path, &content)
    }

    pub fn write_dashboard_health(&self, stats: &VaultStats) -> Result<(), VaultError> {
        let path = self.vault_path.join("Dashboard").join("Health.md");
        let content = format!(
            "# Memory Health\n\n> **{name}** — [[Dashboard/Recent]]\n\n---\n\n\
             ## Storage\n\n| Metric | Value |\n|--------|-------|\n\
             | **Sessions** | {sessions} |\n| **Messages** | {memories} |\n\
             | **Vectors** | {vectors} |\n| **Vault Files** | {files} |\n\
             | **Stage** | {stage} |\n\n## Projection\n\n| Metric | Value |\n\
             |--------|-------|\n| **Sync Interval** | {sync}s |\n\
             | **Max Files** | {max} |\n| **Cold Storage** | >{cold}d |\n\
             | **Last Sync** | {ts} |\n\n\
             *Memory substrate: CortexaDB LSM + ruvector-core HNSW. \
             The vault is a read-write projection of the canonical store.*\n",
            name = self.agent_name, sessions = stats.session_count,
            memories = stats.memory_count, vectors = stats.vector_count,
            files = stats.vault_file_count, stage = stats.stage,
            sync = self.config.sync_interval_secs,
            max = self.config.max_files,
            cold = self.config.cold_storage_days,
            ts = Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        );
        atomic_write(&path, &content)
    }

    /// Writes a delegation artifact to the vault as a markdown file.
    ///
    /// Creates a structured markdown document under `Delegation/` containing:
    /// - Task metadata (task_id, parent_agent, target_agent, priority, deadline)
    /// - Artifact parts (text, JSON data, file references) rendered as sections
    /// - Task state and timing information
    ///
    /// This provides the Obsidian vault as a secondary rich communication path
    /// for inter-agent delegation results, complementing the fast shared-memory
    /// A2A protocol.
    #[allow(clippy::too_many_arguments)]
    pub fn write_delegation_artifact(
        &self,
        task_id: &str,
        parent_agent_id: &str,
        target_agent_id: &str,
        artifact: &savant_ipc::a2a::protocol::Artifact,
        parts: &[savant_ipc::a2a::protocol::ArtifactPart],
        state: savant_ipc::a2a::protocol::TaskState,
        token_budget: u32,
        priority: u8,
        deadline_timestamp: u64,
    ) -> Result<(), VaultError> {
        let delegation_dir = self.vault_path.join("Delegation");
        fs::create_dir_all(&delegation_dir)?;

        let path = delegation_dir.join(format!("{}.md", task_id));
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

        let deadline_str = if deadline_timestamp > 0 {
            let dt = chrono::DateTime::from_timestamp(deadline_timestamp as i64 / 1000, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                .unwrap_or_else(|| "Expired".to_string());
            dt
        } else {
            "No deadline".to_string()
        };

        let mut content = format!(
            "# Delegation Artifact — {task_id}\n\n\
             > **State:** {state} | **Created:** {now}\n\n\
             ---\n\n\
             ## Task Metadata\n\n\
             | Field | Value |\n\
             |-------|-------|\n\
             | **Task ID** | `{task_id}` |\n\
             | **Parent Agent** | `{parent_agent_id}` |\n\
             | **Target Agent** | `{target_agent_id}` |\n\
             | **Priority** | {priority} |\n\
             | **Token Budget** | {token_budget} |\n\
             | **Deadline** | {deadline} |\n\n\
             ## Artifact Parts\n\n\
             **{part_count} part(s)**\n\n",
            task_id = task_id,
            state = state,
            now = now,
            parent_agent_id = parent_agent_id,
            target_agent_id = target_agent_id,
            priority = priority,
            token_budget = token_budget,
            deadline = deadline_str,
            part_count = artifact.part_count,
        );

        for (idx, part) in parts.iter().enumerate() {
            let type_name = match part.part_type {
                savant_ipc::a2a::protocol::ArtifactPartType::Text => "Text",
                savant_ipc::a2a::protocol::ArtifactPartType::Json => "JSON",
                savant_ipc::a2a::protocol::ArtifactPartType::FileReference => "File Reference",
            };
            content.push_str(&format!(
                "### Part {} — {} (offset: {}, len: {})\n\n",
                idx + 1,
                type_name,
                part.data_offset,
                part.data_len
            ));

            match part.part_type {
                savant_ipc::a2a::protocol::ArtifactPartType::Text => {
                    content.push_str(&format!(
                        "```text\n[Text content at shared memory offset {} — {} bytes]\n```\n\n",
                        part.data_offset, part.data_len
                    ));
                }
                savant_ipc::a2a::protocol::ArtifactPartType::Json => {
                    content.push_str(&format!(
                        "```json\n[JSON data at shared memory offset {} — {} bytes]\n```\n\n",
                        part.data_offset, part.data_len
                    ));
                }
                savant_ipc::a2a::protocol::ArtifactPartType::FileReference => {
                    content.push_str(&format!(
                        "[File reference at shared memory offset {} — {} bytes]\n\n",
                        part.data_offset, part.data_len
                    ));
                }
            }
        }

        content.push_str("## Navigation\n\n[[Dashboard/Recent]] | [[Dashboard/Health]]\n");

        atomic_write(&path, &content)?;

        info!(
            task_id = %task_id,
            parts = artifact.part_count,
            "Delegation artifact written to vault"
        );
        Ok(())
    }

    // ─── FULL SYNC ────────────────────────────────────────────────────────

    pub fn run_full_sync(&self, workspace_root: &Path) -> Result<VaultStats, VaultError> {
        self.ensure_structure()?;
        let stats = self.collect_stats(workspace_root);
        let today = Utc::now().date_naive();

        self.write_index(&stats)?;
        self.write_episodic(&today)?;
        self.write_concepts()?;
        self.write_relations()?;
        self.write_triplets()?;
        self.write_entities()?;
        self.write_soul(workspace_root)?;
        self.write_personality(workspace_root)?;
        self.write_evolution_index(workspace_root)?;
        self.write_themes_index()?;
        self.clear_working()?;
        self.write_dashboard_recent()?;
        self.write_dashboard_health(&stats)?;

        info!(
            "[obsidian] Full vault sync: {} files, {} sessions, {} memories",
            stats.vault_file_count, stats.session_count, stats.memory_count,
        );
        Ok(stats)
    }

    fn collect_stats(&self, workspace_root: &Path) -> VaultStats {
        let mut stats = VaultStats {
            agent_name: self.agent_name.clone(),
            ..Default::default()
        };

        if let Some(enclave) = &self.enclave {
            let lsm = enclave.lsm();
            if let Ok(s) = lsm.stats() {
                stats.session_count = s.total_sessions;
                stats.memory_count = s.total_messages;
            }
            stats.vector_count = enclave.vector_count() as u64;
        }

        if self.vault_path.exists() {
            stats.vault_file_count = count_md_files(&self.vault_path);
        }

        let evolution_path = workspace_root.join("EVOLUTION.jsonl");
        if evolution_path.exists() {
            if let Ok(content) = fs::read_to_string(&evolution_path) {
                stats.mutation_count = content.lines().count() as u64;
            }
        }

        stats
    }
}

#[derive(Debug, Clone)]
pub struct VaultStats {
    pub agent_name: String,
    pub session_count: u64,
    pub memory_count: u64,
    pub vector_count: u64,
    pub vault_file_count: usize,
    pub mutation_count: u64,
    pub evolution_score: f32,
    pub stage: String,
}

impl Default for VaultStats {
    fn default() -> Self {
        Self {
            agent_name: String::new(),
            session_count: 0,
            memory_count: 0,
            vector_count: 0,
            vault_file_count: 0,
            mutation_count: 0,
            evolution_score: 0.0,
            stage: "Seedling".to_string(),
        }
    }
}

// ─── HELPERS ─────────────────────────────────────────────────────────────

fn atomic_write(path: &Path, content: &str) -> Result<(), VaultError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

fn count_md_files(path: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                count += count_md_files(&p);
            } else if p.extension().is_some_and(|e| e == "md") {
                count += 1;
            }
        }
    }
    count
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .trim()
        .replace(' ', "-")
}

fn truncate_to_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

fn extract_entities_from_text(text: &str, entities: &mut BTreeMap<String, (u32, u32, i64, i64)>) {
    let now = Utc::now().timestamp();
    // Title-case multi-word patterns (potential project/person names)
    for word in text.split_whitespace() {
        let cleaned: String = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        if cleaned.len() >= 3
            && cleaned.chars().next().is_some_and(|c| c.is_uppercase())
            && cleaned.chars().skip(1).any(|c| c.is_lowercase())
        {
            let entry = entities.entry(cleaned).or_insert((0, 0, now, now));
            entry.0 += 1;
        }
    }
    // URL-like patterns (potential services)
    if text.contains("://") || text.contains(".com") || text.contains(".ai") {
        let service = "web-service".to_string();
        let entry = entities.entry(service).or_insert((0, 0, now, now));
        entry.0 += 1;
    }
}
