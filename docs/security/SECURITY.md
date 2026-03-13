# Savant Security Model: CCT & Sandboxing

Savant introduces a bulletproof security model designed to prevent data exfiltration and malicious tool execution in high-scale swarms.

## 1. Crypto-Cap Tokens (CCT)

Every action performed by an agent is authorized via an **Ed25519-signed Capability Token**. Unlike legacy frameworks that rely on environment variables, Savant tokens are:

- **Assignee-Locked**: Only the specific agent hash it was issued to can use it.
- **Resource-Scoped**: Restricted to specific directory paths or network CIDRs.
- **Time-Bound**: Expiration is calculated in milliseconds for high-frequency rotation.
- **Atomic**: Multiple capabilities can be bundled into a single zero-copy payload.

### Token Flow

1. **Gateway** mints a token for an agent during task delegation.
2. **Agent** passes the token to the **Wassette Sandbox** via IPC.
3. **Sandbox** verifies the signature and scope *mathematically* before granting tool access.

## 2. Wassette Sandbox (OCI WASM)

Agents do not run as native processes. They execute as **OCI-compliant WebAssembly Components**.

- **Memory Isolation**: 4GB linear memory per agent, zero shared heap.
- **Punctured Capability Model**: No FS/Net access unless explicitly punctured via CCT.
- **Deterministic Execution**: Prevents side-channel attacks by normalizing instruction timing.

## 3. Threat Mitigation Matrix

| Threat | Mitigation |
| :--- | :--- |
| **Token Theft** | Identity hash verification (assignee_hash mismatch) |
| **Infinite Loops** | TTL/Depth-limit enforcement in IPC header |
| **Credential Leak** | In-memory key vault (never written to disk) |
| **Resource Exhaustion** | Strict WebAssembly memory/gas limits |

## 4. Security Audit
All cryptographic operations use `ed25519-dalek` with formal verification. Zero-copy IPC paths are audited for pointer-overflow and UAF (Use-After-Free) via `kani`.
