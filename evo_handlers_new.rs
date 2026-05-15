                // ─── Evolution System Handlers ──
                savant_core::types::ControlFrame::SoulMutationPropose {
                    agent_id,
                    mutation_type,
                    target_section,
                    proposed_content,
                    reasoning,
                    conversations_triggered,
                    confidence,
                } => {
                    let mutation_id = uuid::Uuid::new_v4().to_string();
                    let proposed_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;

                    let mutation = serde_json::json!({
                        "status": "pending",
                        "mutation_id": mutation_id,
                        "agent_id": agent_id,
                        "mutation_type": mutation_type,
                        "target_section": target_section,
                        "proposed_content": proposed_content,
                        "before_content": "",
                        "reasoning": reasoning,
                        "conversations_triggered": conversations_triggered,
                        "confidence": confidence,
                        "proposed_at": proposed_at,
                        "decided_at": serde_json::Value::Null,
                        "source_evidence": [],
                        "before_hash": "",
                    });

                    let evo_path = std::path::Path::new(&state.config.system.agents_path).join(&agent_id).join("EVOLUTION.jsonl");
                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&evo_path) {
                        let line = serde_json::to_string(&mutation).unwrap_or_default();
                        let _ = writeln!(file, "{}", line);
                    }

                    let _ = state.nexus.publish("system.evolution.mutation_proposed", &serde_json::to_string(&mutation).unwrap_or_default()).await;
                    let _ = send_control_response("MUTATION_PROPOSED", mutation, &session.session_id, &state.nexus).await;
                }
                savant_core::types::ControlFrame::SoulMutationApprove { agent_id, mutation_id } => {
                    let decided_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
                    let workspace_path = std::path::Path::new(&state.config.system.agents_path).join(&agent_id);

                    let evo_path = workspace_path.join("EVOLUTION.jsonl");
                    let mut mutations: Vec<serde_json::Value> = Vec::new();
                    if evo_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&evo_path) {
                            for line in content.lines() {
                                if let Ok(mut m) = serde_json::from_str::<serde_json::Value>(line) {
                                    if m.get("mutation_id").and_then(|v| v.as_str()) == Some(&mutation_id) {
                                        m["status"] = serde_json::json!("approved");
                                        m["decided_at"] = serde_json::json!(decided_at);
                                    }
                                    mutations.push(m);
                                }
                            }
                        }
                    }

                    let _ = std::fs::write(&evo_path, mutations.iter().map(|m| serde_json::to_string(m).unwrap_or_default()).collect::<Vec<_>>().join("\n") + "\n");

                    let config_path = workspace_path.join("agent.json");
                    if config_path.exists() {
                        if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                            if let Ok(mut config_val) = serde_json::from_str::<serde_json::Value>(&config_content) {
                                let state_obj = config_val.as_object_mut().unwrap();
                                let evo_state = state_obj.entry("evolution_state").or_insert_with(|| serde_json::json!({}));
                                let approved_count = mutations.iter().filter(|m| m.get("status").and_then(|v| v.as_str()) == Some("approved")).count();
                                evo_state["mutation_count"] = serde_json::json!(approved_count);
                                evo_state["last_mutation_at"] = serde_json::json!(decided_at);
                                evo_state["evolution_score"] = serde_json::json!((approved_count as f32 / 10.0).min(1.0));
                                evo_state["stage"] = serde_json::json!(if approved_count >= 10 { "Sovereign" } else if approved_count >= 5 { "Mature" } else if approved_count >= 2 { "Growing" } else { "Seedling" });
                                let _ = std::fs::write(&config_path, serde_json::to_string_pretty(&config_val).unwrap_or_default());
                            }
                        }
                    }

                    let result = serde_json::json!({ "status": "approved", "mutation_id": mutation_id, "agent_id": agent_id, "decided_at": decided_at });
                    let _ = state.nexus.publish("system.evolution.mutation_applied", &serde_json::to_string(&result).unwrap_or_default()).await;
                    let _ = send_control_response("MUTATION_APPROVED", result, &session.session_id, &state.nexus).await;
                }
                savant_core::types::ControlFrame::SoulMutationReject { agent_id, mutation_id, reason } => {
                    let decided_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
                    let evo_path = std::path::Path::new(&state.config.system.agents_path).join(&agent_id).join("EVOLUTION.jsonl");

                    let mut mutations: Vec<serde_json::Value> = Vec::new();
                    if evo_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&evo_path) {
                            for line in content.lines() {
                                if let Ok(mut m) = serde_json::from_str::<serde_json::Value>(line) {
                                    if m.get("mutation_id").and_then(|v| v.as_str()) == Some(&mutation_id) {
                                        m["status"] = serde_json::json!("rejected");
                                        m["decided_at"] = serde_json::json!(decided_at);
                                        m["reason"] = serde_json::json!(reason);
                                    }
                                    mutations.push(m);
                                }
                            }
                        }
                    }
                    let _ = std::fs::write(&evo_path, mutations.iter().map(|m| serde_json::to_string(m).unwrap_or_default()).collect::<Vec<_>>().join("\n") + "\n");

                    let result = serde_json::json!({ "status": "rejected", "mutation_id": mutation_id, "agent_id": agent_id, "reason": reason, "decided_at": decided_at });
                    let _ = state.nexus.publish("system.evolution.mutation_applied", &serde_json::to_string(&result).unwrap_or_default()).await;
                    let _ = send_control_response("MUTATION_REJECTED", result, &session.session_id, &state.nexus).await;
                }
                savant_core::types::ControlFrame::SoulMutationRevert { .. } => {
                    tracing::info!("[evolution] Revert requested (not yet implemented)");
                }
                savant_core::types::ControlFrame::EvolutionHistoryRequest { agent_id, limit } => {
                    let evo_path = std::path::Path::new(&state.config.system.agents_path).join(&agent_id).join("EVOLUTION.jsonl");
                    let mutations: Vec<serde_json::Value> = if evo_path.exists() {
                        std::fs::read_to_string(&evo_path).unwrap_or_default().lines().filter_map(|line| serde_json::from_str(line).ok()).collect()
                    } else { Vec::new() };
                    let total = mutations.len();
                    let limited: Vec<_> = if limit > 0 { mutations.into_iter().rev().take(limit as usize).collect() } else { mutations };
                    let result = serde_json::json!({ "agent_id": agent_id, "mutations": limited, "total": total });
                    let _ = send_control_response("EVOLUTION_HISTORY", result, &session.session_id, &state.nexus).await;
                }
                savant_core::types::ControlFrame::EvolutionScoreRequest { agent_id } => {
                    let config_path = std::path::Path::new(&state.config.system.agents_path).join(&agent_id).join("agent.json");
                    let (score, stage, mutation_count) = if config_path.exists() {
                        std::fs::read_to_string(&config_path).ok()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                            .and_then(|v| v.get("evolution_state").cloned())
                            .map(|es| {
                                let count = es.get("mutation_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                let score = es.get("evolution_score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                                let stage = es.get("stage").and_then(|v| v.as_str()).unwrap_or("Seedling").to_string();
                                (score, stage, count)
                            })
                            .unwrap_or((0.0, "Seedling".to_string(), 0))
                    } else { (0.0, "Seedling".to_string(), 0) };
                    let result = serde_json::json!({ "agent_id": agent_id, "evolution_score": score, "stage": stage, "mutation_count": mutation_count });
                    let _ = send_control_response("EVOLUTION_SCORE", result, &session.session_id, &state.nexus).await;
                }
                savant_core::types::ControlFrame::EvolutionIdeaSubmit { agent_id, content, significance } => {
                    let mutation_id = uuid::Uuid::new_v4().to_string();
                    let proposed_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as i64;
                    let mutation = serde_json::json!({
                        "status": "pending", "mutation_id": mutation_id, "agent_id": agent_id,
                        "mutation_type": "additive", "target_section": "IDEAS",
                        "proposed_content": content, "before_content": "",
                        "reasoning": format!("User-submitted idea (significance: {})", significance),
                        "conversations_triggered": [], "confidence": significance,
                        "proposed_at": proposed_at, "decided_at": serde_json::Value::Null,
                        "source_evidence": [], "before_hash": "",
                    });
                    let evo_path = std::path::Path::new(&state.config.system.agents_path).join(&agent_id).join("EVOLUTION.jsonl");
                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&evo_path) {
                        let line = serde_json::to_string(&mutation).unwrap_or_default();
                        let _ = writeln!(file, "{}", line);
                    }
                    let _ = send_control_response("IDEA_SUBMITTED", mutation, &session.session_id, &state.nexus).await;
                }
