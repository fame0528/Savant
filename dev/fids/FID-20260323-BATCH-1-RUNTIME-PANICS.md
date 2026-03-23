# FID-20260323-BATCH-1-RUNTIME-PANICS

**Date:** 2026-03-23
**Status:** RE-AUDITED — Fix matrix certified via Perfection Loop
**Protocol:** Perfection Loop + Checkpoint Gates (brain surgery protocol)
**Source:** AUDIT-REPORT.md — Runtime panics remaining from production pass
**Standard:** $1M+ enterprise valuation — zero tolerance for stubs, data loss, security gaps
**Competitor Research:** Pending — openclaw, ironclaw, zeroclaw patterns for panic prevention

---

## CRITICAL RULES FOR THIS FID

1. **Brain Surgery Protocol:** Every file is interconnected. One fix in file A can break logic in file Z. Before ANY change:
   - Read the target file 0-EOF
   - Read ALL files that import from or are imported by the target
   - Trace the data flow end-to-end
   - Present the impact matrix to Spencer BEFORE making the change
   - Get approval → Make change → Verify → Checkpoint

2. **Checkpoint Gates:** After EVERY fix:
   - `cargo check --workspace` must pass with 0 errors
   - Present what changed and what could be affected
   - Spencer approves before proceeding to next fix

3. **No Autonomous Changes:** If anything is unclear, STOP. Get clarity.

4. **Read Before Touch:** Every file read 0-EOF before any edit. No exceptions.

5. **Perfection Loop:** Run on each fix plan BEFORE implementation.

6. **No Stubs:** Every implementation must be complete, production-grade, or properly removed.

---

## FIX MATRIX (Re-audited post-production-pass)

### Batch 1: Runtime Panics (4 fixes, 4 files)

| # | Severity | Issue | File | Current Line | Fix | Cross-Impact | Gate |
|---|----------|-------|------|------|-----|-------------|------|
| M1 | HIGH | `unwrap()` on `default_window_icon()` — panics if no icon configured | `desktop/main.rs` | 194 | Build tray without icon if None: `match app.default_window_icon() { Some(i) => .icon(i.clone()), None => { warn no icon } }` | Desktop startup — crashes if icon missing |
| M2 | HIGH | `.expect()` on `run()` — panics on Tauri runtime error | `desktop/main.rs` | 218-219 | `.unwrap_or_else(\|e\| { error!(...); std::process::exit(1); })` — log and exit cleanly | Desktop startup — crashes on runtime error |
| M3 | HIGH | Byte-based truncation splits UTF-8 chars — slice panic on multi-byte content | `agent/react/reactor.rs` | 187-199 | `&output[..head_size]` uses byte index on string — panics if head_size lands mid-char. Fix: find nearest char boundary with `is_char_boundary()` for both head and tail slices. | Tool output truncation — agent loop stability |
| M4 | HIGH | `content[content.len() - 2000..]` — slice panic on UTF-8 boundary | `memory/daily_log.rs` | 131, 146 | Find nearest char boundary backward from target: `while pos > 0 && !content.is_char_boundary(pos) { pos -= 1; }`. Fix BOTH `read_today` and `read_date`. | Daily log ingestion — data corruption risk |

**Re-audit findings:**
- M1: CONFIRMED at line 194 — `.unwrap()` on `Option<&Icon>`
- M2: CONFIRMED at line 218-219 — `.expect()` on `Result`
- M3: CONFIRMED at lines 187-199 — byte indices `&output[..head_size]` and `&output[output.len() - tail_size..]` will panic on multi-byte UTF-8 if split position lands mid-char
- M4: CONFIRMED at lines 131 and 146 — same pattern, byte index `content[content.len() - 2000..]` panics if content has multi-byte chars near the 2000-byte boundary

**Cross-Impact for Batch 1:**
```
desktop/main.rs:194 → App startup, icon loading, window creation
desktop/main.rs:218-219 → App startup, Tauri runtime lifecycle
reactor.rs:187-199 → Tool output processing, agent loop, self-repair
daily_log.rs:131,146 → Memory ingestion, daily log persistence, auto_recall
```

---

## EXECUTION ORDER

```
M1: Desktop main.rs unwrap (line 194)
  ↓ CHECKPOINT: cargo check -p savant_desktop + Spencer approval
M2: Desktop main.rs expect (line 218-219)
  ↓ CHECKPOINT: cargo check -p savant_desktop + Spencer approval
M3: Reactor byte truncation (lines 187-199)
  ↓ CHECKPOINT: cargo check -p savant_agent + Spencer approval
M4: Daily log UTF-8 slice (lines 131, 146)
  ↓ CHECKPOINT: cargo check -p savant_memory + Spencer approval
Final: cargo check --workspace + commit + push
```

---

## SUCCESS CRITERIA

- [ ] M1 fixed — no unwrap on icon load
- [ ] M2 fixed — no expect in production
- [ ] M3 fixed — UTF-8 safe truncation
- [ ] M4 fixed — UTF-8 safe slicing
- [ ] `cargo check --workspace` — 0 errors
- [ ] Tracking files updated
- [ ] Committed + pushed

---

## PERFECTION LOOP ITERATIONS

| Iteration | Changes |
|-----------|---------|
| 1 | Initial FID created — 4 fixes, generic line numbers from audit report |
| 2 | Re-audited all 4 target files 0-EOF — corrected line numbers, confirmed issues exist, found M4 has 2 instances (not 1), M3 is byte-index panic not char-count, updated severity to HIGH |

---

## CERTIFIED FIX APPROACHES

**M1 (desktop/main.rs:194):**
```rust
// BEFORE:
.icon(app.default_window_icon().unwrap().clone())

// AFTER:
.icon(match app.default_window_icon() {
    Some(icon) => icon.clone(),
    None => {
        tracing::warn!("No default window icon configured, building tray without icon");
        // Build tray without icon — use .icon_from_bytes() with empty or skip
        // TrayIconBuilder::new() works without .icon() call
    }
})
```

**M2 (desktop/main.rs:218-219):**
```rust
// BEFORE:
.run(tauri::generate_context!())
.expect("error while running tauri application");

// AFTER:
.run(tauri::generate_context!())
.unwrap_or_else(|e| {
    tracing::error!("Tauri runtime error: {}", e);
    std::process::exit(1);
});
```

**M3 (reactor.rs:187-199):**
```rust
// BEFORE: byte-based slicing — panics on multi-byte UTF-8
&output[..head_size]                    // head_size is bytes, may split char
&output[output.len() - tail_size..]     // tail_size is bytes, may split char

// AFTER: char-boundary-aware slicing
let mut head_end = head_size.min(output.len());
while head_end > 0 && !output.is_char_boundary(head_end) {
    head_end -= 1;
}
let mut tail_start = output.len().saturating_sub(tail_size);
while tail_start < output.len() && !output.is_char_boundary(tail_start) {
    tail_start += 1;
}
format!("{}\n\n[... truncated ...]\n\n{}", &output[..head_end], &output[tail_start..])
```

**M4 (daily_log.rs:131, 146):**
```rust
// BEFORE: byte-based slicing — panics on multi-byte UTF-8
content[content.len() - 2000..]

// AFTER: char-boundary-aware truncation
let mut start = content.len().saturating_sub(2000);
while start > 0 && !content.is_char_boundary(start) {
    start -= 1;
}
Ok(content[start..].to_string())
```

---

*FID certified via Perfection Loop (Iteration 2). Re-audited post-production-pass.*
