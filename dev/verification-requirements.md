# External Requirements for Savant

**Purpose:** Resources/items needed from external sources to complete remaining verification tasks  
**Last updated:** 2026-03-18

---

## Currently Needed

### 1. Docker Desktop
- Start Docker Desktop (Windows)
- Pull alpine image: `docker pull alpine:latest`
- This enables 2 Docker tests to pass and allows full sandbox verification

### 2. GitHub Admin Access (for CI)
- Create `.github/workflows/security.yml` for `cargo audit`
- Optional: Set up Dependabot automatic dependency updates

---

## Everything Else Is Done

All code fixes, tests, documentation, and benchmarks are complete. The remaining items are infrastructure-dependent verification tasks that require:
- Running Docker Desktop (for Docker sandbox tests)
- GitHub admin access (for CI-007 `cargo audit` check)
- Manual browser testing (for dashboard UI verification)
