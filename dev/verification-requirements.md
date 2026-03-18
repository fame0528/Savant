# What Spencer Needs To Do

**Purpose:** Exact steps Spencer needs to complete before the remaining 69 verification tasks can proceed.  
**Last updated:** 2026-03-18

---

## Step 1: Delete Old Database (10 seconds)

The old SQLite database is causing stale data warnings. Delete it:

```bash
cd C:\Users\spenc\dev\Savant
rmdir /s /q data\savant
mkdir data\savant
```

This clears the old database. The system will recreate it fresh on next launch.

---

## Step 2: Verify Docker Desktop Is Running (2 minutes)

Docker is required for the sandbox execution path. Check:

```bash
docker --version
docker ps
```

If `docker --version` fails:
- Install Docker Desktop from https://www.docker.com/products/docker-desktop/
- Enable WSL2 backend in Docker Desktop settings
- Restart Docker Desktop after install

If `docker ps` shows permission errors:
- Add your user to the `docker-users` group
- Restart your terminal

**What I need:** Confirmation that `docker ps` works without errors.

---

## Step 3: Provide Threat Intel Endpoint (2 minutes)

The security scanner has a `sync_threat_intelligence()` function that tries to fetch a blocklist from a URL. Right now this URL doesn't exist.

**What I need from you — pick ONE:**

| Option | What happens |
|--------|--------------|
| A) Give me a URL | I'll wire it into the scanner |
| B) Tell me to build a mock | I'll create a local mock endpoint for testing |
| C) Tell me to disable it | I'll remove the feature until you have a feed |

---

## Step 4: Provide Example Skill Content (5 minutes)

The documentation needs a working example skill. I need:

1. **A skill name** — e.g., `hello-savant`
2. **What it should do** — e.g., "Takes a name, returns a greeting"
3. **Where to publish it** — ClawHub account, or just local?

**What I need from you:** Just tell me what the example skill should do. I'll build it.

---

## Step 5: Decide Nix Sandbox Support (1 minute)

The Nix sandbox only works on Linux. On Windows it currently fails confusingly.

**What I need from you — pick ONE:**

| Option | What happens |
|--------|--------------|
| A) Linux only | I'll add a clear error on Windows: "Nix sandbox requires Linux" |
| B) WSL2 support | I'll detect WSL2 and route Nix commands through it |
| C) Remove Nix | I'll remove the Nix sandbox entirely |

---

## Step 6: GitHub CI/CD Access (2 minutes)

To set up automated testing on PRs, I need:

**What I need from you:** Confirm you want GitHub Actions CI set up. Then I'll create:
- `.github/workflows/ci.yml` — runs `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`
- `.github/dependabot.yml` — automated dependency updates

Just say "set up CI" and I'll do it.

---

## Step 7: Provide Test Machines (when available)

Cross-platform testing requires:

| Platform | What's needed |
|----------|---------------|
| Linux | Ubuntu 22.04+ machine with Rust installed |
| macOS | Apple Silicon Mac with Rust installed |

**What I need from you:** These are optional but recommended. If you have access to these machines, let me know and I'll provide exact test commands to run.

---

## Step 8: Profiling Tools (optional)

Performance profiling requires:

```bash
# On Linux
cargo install flamegraph
sudo apt install valgrind heaptrack

# On macOS
cargo install flamegraph
```

**What I need from you:** Only if you want performance benchmarks. Say "run benchmarks" and I'll set them up.

---

## Summary: What I Need Right Now

| # | Question | Your answer |
|---|----------|-------------|
| 1 | Can you run `docker ps` without errors? | Yes / No |
| 2 | Threat intel endpoint: URL, mock, or disable? | A / B / C |
| 3 | Example skill: what should it do? | Description |
| 4 | Nix on Windows: clear error, WSL2, or remove? | A / B / C |
| 5 | Set up GitHub CI? | Yes / No |

That's it. Everything else I can figure out myself.
