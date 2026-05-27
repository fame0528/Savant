# **Savant Microsandbox: A Comprehensive Architecture for Hardware-Isolated AI Agent Execution**

The transition from software-defined containerization to hardware-enforced micro-virtualization represents a mandatory evolution for autonomous AI agent frameworks. As agents are granted broader action spaces—including arbitrary code execution, filesystem manipulation, and continuous network access—the host system must be insulated against both malicious prompt injections and compromised agent dependencies. The current execution architecture of the Savant framework demonstrates severe platform disparities, particularly concerning the degradation of security boundaries on the Windows operating system.

The following analysis provides an exhaustive architectural blueprint for a Rust-native, hardware-isolated microVM execution environment explicitly tailored for autonomous AI workloads. This blueprint prioritizes absolute parity across Linux, macOS, and Windows, offering a unified, cryptographically secure execution boundary.

## **A. Architecture Design**

### **1\. Current State**

The existing Savant security model is deeply fragmented across operating systems. Linux workloads rely on Landlock ABI V1 and capability dropping, while macOS deployments utilize unmanaged sandbox-exec profiles that are administered by the child process itself. The Windows implementation relies solely on \-ExecutionPolicy Restricted for script execution, completely lacking filesystem sandboxing, capability dropping, and resource limits.1 In contrast, reference implementations such as microsandbox leverage libkrun for hardware isolation but introduce a monolithic host process susceptible to Virtual Machine Monitor (VMM) escapes and lack Windows support entirely.

### **2\. Gaps and Risks**

The absence of hardware-level isolation on Windows renders the entire framework vulnerable to credential theft and privilege escalation. Furthermore, existing microVM solutions expose excessive virtio device surfaces (such as virtio-serial), which expand the attack vector for an agent compromised via prompt injection.2 Running a microVM without hardening the host VMM process creates a single point of failure; if a guest escapes the hypervisor, it immediately inherits the unrestricted privileges of the host broker.

### **3\. Design Recommendations**

The optimal architecture for an AI-native microVM demands a rootless, compartmentalized VMM utilizing a minimal virtio device model. The architecture must abstract the hypervisor layer. Instead of interacting directly with KVM ioctls, integrating cloud-hypervisor is highly recommended. Built in Rust, cloud-hypervisor features minimal legacy device emulation, boots in under 100ms, and natively supports KVM on Linux and the Microsoft Hypervisor (MSHV/WHP) on Windows.4

The host process managing the VMM must be subjected to defense-in-depth isolation before the guest boots. On Linux, this requires seccomp-bpf to restrict system calls to the bare minimum required for VM execution, combined with Landlock to deny filesystem access outside the agent's designated workspace. On Windows, the VMM host process must be spawned within an AppContainer with zero capabilities, restricted by a Job Object enforcing KILL\_ON\_JOB\_CLOSE and an ACTIVE\_PROCESS\_LIMIT.1

The attack surface must be aggressively reduced. Legacy character devices like virtio-serial should be entirely omitted.2 Host-guest communication must rely exclusively on virtio-vsock (or hvsock on Windows) to implement POSIX socket APIs without relying on TCP/IP networking.7 virtio-blk handles immutable OCI layers, and virtio-net handles in-process proxy networking.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | cloud-hypervisor Integration | Establish a cross-platform VMM wrapper supporting KVM and WHP/MSHV.4 |
| **P0** | Host Process Hardening | Implement seccomp-bpf (Linux) and AppContainer \+ Job Objects (Windows).1 |
| **P1** | Device Surface Reduction | Strip the VMM device tree to virtio-vsock, virtio-blk, and virtio-net only.2 |

### **5\. Open Questions**

Does utilizing a robust VMM like cloud-hypervisor introduce unacceptable memory overhead per agent compared to a bespoke, tightly coupled ioctl wrapper built directly on rust-vmm?

## **B. Platform Portability (CRITICAL for Savant)**

### **1\. Current State**

Savant lacks true hardware isolation on Windows and macOS. The reference microsandbox achieves macOS support via the Apple Hypervisor but explicitly lacks Windows support, leaving a critical segment of Savant's developer base exposed to host compromise. Windows environments currently fall back to insecure interpreter execution.

### **2\. Gaps and Risks**

Operating system APIs for hardware virtualization diverge radically. Linux relies on KVM, Windows on the Windows Hypervisor Platform (WHP) or Host Compute Service (HCS), and macOS on Virtualization.framework or Hypervisor.framework. Attempting to unify these under a single monolithic VMM logic leads to fragile, platform-specific edge cases and prevents graceful degradation when hardware virtualization is unavailable.

### **3\. Design Recommendations**

A tiered isolation trait must be designed to abstract the underlying hypervisor API, allowing graceful degradation based on system capabilities while treating Windows as a Tier 1 citizen.

For primary hardware virtualization (Tier 1), Linux deployments will utilize the KVM backend via cloud-hypervisor. Windows deployments will utilize native Hyper-V isolated containers managed via the Host Compute Service (HCS). The zlayer-hcs crate provides a safe Rust wrapper over vmcompute.dll for spawning these workloads directly, mapping abstract container configurations into schema v2 JSON payloads required by the Windows kernel.8 Alternatively, the Windows Hypervisor Platform (WHP) can be utilized through the hyperlight-host crate.9 macOS deployments will rely on Apple's Virtualization.framework utilizing crates such as vfrust or applevisor.11 This framework natively supports virtiofs and Rosetta 2 for executing x86\_64 agent payloads on ARM silicon.13

If hardware virtualization is disabled (Tier 2 Process Isolation Fallback)—such as within nested virtualization environments lacking SR-IOV—the system must degrade safely rather than failing open. On Windows, this necessitates falling back to an AppContainer profile with a restricted token, mapped to a tightly constrained Job Object.1 On macOS, this triggers a fallback to strict sandbox-exec profiles.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | Abstract VMM Trait | Define an AgentHypervisor trait handling lifecycle events (boot, pause, teardown). |
| **P0** | Windows WHP/HCS Backend | Implement hardware isolation for Windows natively using zlayer-hcs.8 |
| **P1** | macOS Virtualization.framework | Integrate vfrust for macOS hardware execution and Rosetta 2 support.11 |
| **P2** | Graceful Degradation | Implement AppContainer/Job Object fallback for Windows environments.1 |

### **5\. Open Questions**

Does the initialization overhead of HCS containers negate the sub-100ms boot requirement, necessitating a direct WHP/microVM approach via hyperlight for ephemeral agent tasks?

## **C. Host-Guest Communication Security**

### **1\. Current State**

The reference microsandbox relies on a custom CBOR-over-length-prefixed protocol transmitted over virtio-serial. Crucially, this channel lacks mutual authentication, encryption, or anti-replay mechanisms. Savant's existing communication model utilizes Crypto-Capability Tokens (CCT) that act as simple bearer tokens without proof-of-possession.

### **2\. Gaps and Risks**

virtio-serial acts as a raw character device, meaning existing POSIX networking utilities cannot interact with it natively.2 Furthermore, a lack of cryptographic binding means the host cannot guarantee that the guest agentd binary hasn't been swapped via a Time-of-Check to Time-of-Use (TOCTOU) attack during the boot sequence. Any process within the guest that achieves root escalation can impersonate the internal agent process.

### **3\. Design Recommendations**

The architecture must abandon virtio-serial entirely in favor of virtio-vsock (Linux/macOS) and hvsock (AF\_HYPERV on Windows).2 These technologies provide POSIX socket semantics tied to Context Identifiers (CIDs) rather than IP addresses, eliminating network stack attack surfaces while allowing standard socket programming.15

To secure this channel, a mutual authentication protocol must be implemented utilizing the Noise Protocol Framework.16 Specifically, a pattern such as Noise\_XX\_25519\_ChaChaPoly\_SHA256 ensures perfect forward secrecy and mutual authentication.17 At boot, the host generates an ephemeral Ed25519 keypair. The public key, along with a unique session identifier, is injected directly into the guest's read-only initrd (initial ramdisk). The guest agentd generates its own keypair, performing a Trust-On-First-Use (TOFU) handshake with the host.18 This natively guarantees resistance to replay attacks and enforces cryptographic integrity over the CBOR payload structure.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | virtio-vsock / AF\_HYPERV | Transition the IPC transport layer entirely to hypervisor sockets.19 |
| **P0** | Noise Protocol Integration | Implement the Noise\_XX handshake over the vsock stream.17 |
| **P1** | Initrd Key Injection | Build dynamic initrd patching logic to inject host public keys pre-boot. |

### **5\. Open Questions**

Will the computational overhead of the Noise handshake violate the sub-100ms boot SLA for highly ephemeral agent actions, and can the handshake state be pre-computed or resumed?

## **D. Secrets Architecture**

### **1\. Current State**

Savant's ECHO compiler leaks massive amounts of environmental variables into untrusted compilation stages, allowing unmitigated access to master secret keys. The microsandbox architecture intercepts TLS traffic via a proxy to substitute placeholders for actual secrets, but this is restricted entirely to HTTP/HTTPS traffic. Furthermore, secrets are stored in host memory indefinitely.

### **2\. Gaps and Risks**

AI agents frequently interact with gRPC endpoints, raw TCP databases (e.g., PostgreSQL), and WebSocket streams. An HTTP-only secret substitution layer is fundamentally inadequate and leaks placeholders when protocols degrade. Additionally, if the host broker crashes, residual memory containing API keys can be extracted via memory scraping or cold-boot attacks.

### **3\. Design Recommendations**

To support arbitrary protocols, the smoltcp in-process network stack 20 must be extended to perform byte-level stream inspection. For TLS connections, the VMM acts as a transparent MitM proxy using a dynamically generated Root CA (injected into the guest trust store). Secrets are substituted at the plaintext layer before re-encryption, regardless of whether the L7 protocol is HTTP, gRPC, or raw TCP.

Secrets held in the host broker must use the zeroize crate to guarantee memory is overwritten upon scope exit, preventing compiler optimization bypasses.21 Furthermore, memory must be locked to prevent secrets from being swapped to disk. On Linux, this is achieved via mlock() and memfd\_secret. On Windows, VirtualLock is required.22 However, Windows VirtualLock drops its lock when page protection is changed to PAGE\_NOACCESS.24 Therefore, a lease-based acquisition model must be implemented, where VirtualProtect enforces PAGE\_NOACCESS when idle, and transitions to PAGE\_READWRITE only during active secret substitution, immediately zeroizing the buffer afterward.22

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | zeroize \+ Memory Locking | Implement secure memory structs backed by mlock/VirtualLock.21 |
| **P0** | L4/L7 Transparent Proxy | Implement TLS interception within the smoltcp networking stack.27 |
| **P1** | Windows PAGE\_NOACCESS Lease | Build the lease-based locking model to bypass Windows page fault limits.22 |

### **5\. Open Questions**

How effectively can a Rust MitM proxy handle high-throughput, multiplexed gRPC connections without stalling the smoltcp event loop, and does this require a threaded offload architecture?

## **E. Filesystem Isolation**

### **1\. Current State**

Savant relies on simplistic path prefix matching, which is easily bypassed via symlink attacks, directory traversal, or Unicode normalization discrepancies. The microsandbox uses an experimental virtiofs implementation (PassthroughFS) with no disk quotas, allowing guest agents unrestricted write access.

### **2\. Gaps and Risks**

Without block-level disk quotas, a compromised agent can perform arbitrary Denial of Service (DoS) attacks against the host disk. Furthermore, writable overlay filesystems are often implemented insecurely in container runtimes, allowing an agent to modify files that should remain strictly read-only.

### **3\. Design Recommendations**

The base OCI images must be materialized as EROFS (Enhanced Read-Only File System) block devices and attached to the microVM via virtio-blk. This provides mathematically guaranteed immutability at the block level, completely bypassing LFS bypass vulnerabilities.

For isolated writable layers with quotas, Linux deployments will use overlayfs with xfs project quotas enforcing a strict byte-level limit on the upper directory. Windows deployments will materialize writable layers as fixed-size, dynamically expanding VHDX virtual disks attached via WHP/HCS.29 The inherent structural limit of the VHDX provides absolute containment against disk exhaustion.

When persistent home directories or specific host paths must be mounted, virtiofs should be used. On macOS, this utilizes the highly optimized, native VirtioFS backend in Virtualization.framework.30 Path resolution must occur entirely in the host broker, strictly enforcing access control lists and tracking all I/O operations for the cryptographic audit log.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | EROFS Base Layers | Materialize OCI images into EROFS formats attached via virtio-blk. |
| **P0** | Hard Disk Quotas | Enforce XFS project quotas (Linux) or VHDX limits (Windows).29 |
| **P1** | Native macOS VirtioFS | Integrate Virtualization.framework directory sharing.30 |

### **5\. Open Questions**

Can virtiofs be reliably compiled and executed on Windows, given that upstream QEMU has deprecated its native Windows virtiofs daemon, or must Windows rely entirely on VMBus storage channels? 32

## **F. Network Security**

### **1\. Current State**

Current Docker sandbox configurations in Savant rely on the none network, completely airgapping the container. While secure, this neuters AI agents requiring API access. The microsandbox implements DNS filtering but suffers from TOCTOU vulnerabilities where an allowed domain's IP changes between the DNS resolution and the subsequent TCP connection.

### **2\. Gaps and Risks**

If an agent resolves an allowed domain (e.g., api.github.com), and then immediately opens a TCP socket to an internal RFC 1918 address (e.g., 169.254.169.254 AWS metadata endpoint) while presenting the permitted hostname in the HTTP header, a naive filter will allow the traffic, leading to Server-Side Request Forgery (SSRF) and host metadata compromise.

### **3\. Design Recommendations**

The microVM must lack any bridge or TAP interfaces to the host network. Instead, all virtio-net traffic terminates into a user-space smoltcp stack running in the host broker.20 This guarantees that no packets reach the host kernel's routing table without explicit cryptographic authorization from the policy engine.

To defeat DNS rebinding TOCTOU attacks, the broker intercepts all port 53/853 traffic deterministically.27 When an agent requests a resolution for an allowed domain, the host resolves it, caches the IP, and returns it to the guest. The firewall policy is then dynamically updated to *temporarily* allow L4 connections exclusively to that exact IP address, bound to that specific socket session.

Furthermore, the L3 routing layer within the smoltcp implementation must permanently drop all packets destined for loopback (127.0.0.0/8), private networks (RFC 1918), and link-local addresses, structurally eliminating SSRF.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | smoltcp User-Space Stack | Implement TAP-less network termination via smoltcp.33 |
| **P0** | Anti-Rebinding DNS Proxy | Cache IP resolutions to enforce strict IP-to-Domain bindings.28 |
| **P1** | Bandwidth Shaping | Implement Token Bucket algorithms inside the networking event loop. |

### **5\. Open Questions**

What is the maximum throughput capable of being routed through a single-threaded smoltcp instance, and does it require multi-threaded sharding for heavy AI data downloads?

## **G. Observability and Audit**

### **1\. Current State**

Savant relies on easily manipulated text logs and basic SHA-256 command hashing. There is no forensic capability for post-mortem analysis of crashed sandboxes, nor is there proof against tampering by a compromised host process. The microsandbox logging lacks Write-Once-Read-Many (WORM) capabilities.

### **2\. Gaps and Risks**

If a sophisticated attack compromises the host broker, the attacker can silently alter standard logs to cover their tracks. For compliance and behavioral analysis of autonomous AI, the exact provenance and sequence of every filesystem modification, network request, and shell execution must be mathematically verifiable.

### **3\. Design Recommendations**

The framework must implement an append-only audit log utilizing Merkle trees or hash chaining (similar to the immutable\_logging crate or DriftDB design).34 Every log entry (e.g., EXEC, FS\_WRITE, NET\_CONNECT) contains the hash of the previous entry, creating a tamper-evident sequence.

The internal agentd process forwards execution telemetry over the authenticated Noise vsock channel to the host. The host computes the hash chain, streaming it sequentially to an external syslog/SIEM or a secured Git repository where OS-level ACLs restrict modifications.36

Upon guest kernel panic or unhandled exception, the VMM must trigger an automated forensic snapshot of the virtio-blk writable layer and a segmented core dump of guest memory. Crucially, because secrets are substituted at the TLS layer in the host, the guest memory dump will contain only the *placeholders*, making the core dump inherently safe for external analysis without risking secret leakage.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | Append-Only Hash Chain | Implement cryptographically verifiable logging structures.34 |
| **P1** | Telemetry over vsock | Stream filesystem and network events out of the VM to the host. |
| **P2** | Forensic State Capture | Implement automated block layer snapshots upon guest crash. |

### **5\. Open Questions**

How do we handle the massive volume of filesystem stat and read events without overwhelming the vsock communication channel and causing I/O bottlenecks?

## **H. Resource Accounting and Limits**

### **1\. Current State**

Linux environments within Savant utilize arbitrary 30-second timeouts, while Windows features zero resource limits.1 microsandbox fails to implement swap/OOM limits or per-sandbox process restrictions, leaving systems vulnerable to denial of service.

### **2\. Gaps and Risks**

Unbound resource consumption allows a malicious agent to execute a fork bomb, exhaust physical memory (triggering system-wide OOM killers), or saturate disk I/O, destabilizing the entire Savant swarm orchestration node.

### **3\. Design Recommendations**

A comprehensive resource control system must be natively implemented. On Linux, the architecture will utilize unified cgroups v2 to enforce strict memory.max, memory.swap.max, cpu.max, and pids.max on the VMM host process and any associated threads.

On Windows, the VMM process and its threads will be mapped into an extended Windows Job Object. Using SetInformationJobObject, the system will enforce JobObjectExtendedLimitInformation to cap the maximum working set size (memory) and establish an ActiveProcessLimit of 1 (preventing the VMM from spawning malicious child processes).1 macOS enforcement will utilize launchd property lists or sandbox-exec limits, falling back to POSIX setrlimit for strict memory and file descriptor constraints.

The broker must intercept resource exhaustion signals (e.g., cgroup OOM notifications). Instead of a hard kernel kill, the broker should freeze the VMM via the hypervisor API, generating a forensic snapshot and returning a clean RESOURCE\_EXHAUSTED error to the Savant orchestration layer to allow graceful swarm reallocation.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | Memory & CPU Hard Limits | Implement cross-platform constraint logic (cgroups, Job Objects).1 |
| **P1** | Network & IOPS Bandwidth | Implement limits via smoltcp token buckets and virtio-blk throttling. |
| **P2** | Graceful Exhaustion Handling | Capture state prior to SIGKILL/OOM termination. |

### **5\. Open Questions**

Can Windows Job Objects reliably constrain the CPU cycles of isolated WHP/HCS guest instances managed under vmcompute.exe, or do those cycles escape the parent process accounting?

## **I. Supply Chain and Image Security**

### **1\. Current State**

Both Savant and microsandbox lack OCI image supply chain verification. They pull arbitrary containers without checking digests, signatures, or Software Bill of Materials (SBOM) metadata.

### **2\. Gaps and Risks**

Pulling an unsigned latest tag from public registries allows an attacker to poison the base environment of the AI agent, embedding rootkits or backdooring the Python/Node runtime before the agent code even executes.

### **3\. Design Recommendations**

The sandbox must implement a "Zero-Trust Image" policy natively in Rust via the sigstore-rs crate.37 Before extracting or mounting an OCI layer, the broker must triangulate the Cosign signature within the OCI registry.38

The image digest must be validated against a trusted public key or a keyless OpenID Connect (OIDC) identity linked to the Fulcio/Rekor transparency logs.39 Images lacking a valid signature or an attached SBOM are categorically rejected.

Downloaded layers must be securely cached on disk using AES-256-GCM encryption, with the decryption key locked in secure memory. Prior to booting the VM, the SHA-256 hash of the local EROFS block device is verified against the signed OCI manifest, completely mitigating local caching poisoning attacks.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | Image Digest Pinning | Force all executions to resolve to a pinned SHA-256 digest. |
| **P0** | sigstore-rs Verification | Validate Cosign signatures before materializing the image.38 |
| **P2** | Encrypted Layer Cache | Encrypt cached base images at rest to prevent local tampering. |

### **5\. Open Questions**

Can we efficiently verify the signature of multi-gigabyte machine learning base images without incurring unacceptable latency during agent spawn?

## **J. Side-Channel Mitigations**

### **1\. Current State**

Current execution environments within Savant completely ignore micro-architectural side-channel attacks (e.g., Spectre, Meltdown, L1TF).

### **2\. Gaps and Risks**

In a multi-tenant swarm where multiple agents execute on the same physical hardware, a compromised agent could theoretically utilize cache-timing attacks to extract cryptographic material or API keys from a neighboring agent executing on a sibling hyperthread.

### **3\. Design Recommendations**

While exotic, side-channel attacks remain a threat for high-value AI operations. Mitigations must focus on logical separation without requiring specialized hardware. For critical isolation tiers, the hypervisor should be configured to disable SMT/Hyperthreading, mapping VCPUs exclusively to dedicated physical cores.

On supported Linux kernels, the broker should utilize Core Scheduling to ensure mutually untrusting VCPUs do not share a core. Where available, the system will leverage Intel Cache Allocation Technology (CAT) to partition L3 cache boundaries. Furthermore, the hypervisor will explicitly disable Kernel Samepage Merging (KSM) or transparent memory deduplication on the host for any memory regions assigned to the microVM, preventing memory deduplication timing attacks.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P1** | Disable Memory Deduplication | Set hypervisor flags to prevent KSM across guest boundaries. |
| **P2** | Core Scheduling | Implement CPU pinning and SMT restrictions for high-security workloads. |

### **5\. Open Questions**

Does disabling SMT drastically reduce the swarm density achievable on standard consumer hardware, and is the performance trade-off acceptable for standard AI workloads?

## **K. Community-Requested Features**

### **1\. Current State**

Developers utilize AI frameworks for complex reasoning and multimodal tasks, demanding persistent state, GPU access for local LLM execution, and granular network controls. microsandbox is largely designed as an ephemeral, stateless execution context.

### **2\. Gaps and Risks**

MicroVMs are traditionally designed as ephemeral, CPU-only functions. This conflicts with the reality of AI agents that require gigabytes of context, persistent memory, and hardware acceleration to process generative tasks.

### **3\. Design Recommendations**

For local LLM inference or computer vision tasks, GPU access is paramount. On Linux, this is achieved via standard PCIe VFIO passthrough. On Windows, the sandbox must leverage GPU Partitioning (GPU-P) via SR-IOV.41 By integrating with the Host Compute Service (HCS), a fraction of the physical GPU can be securely exposed to the Windows/Linux guest utilizing dxgkrnl paravirtualization.42

Rather than permanent network access, agents should be capable of requesting time-bounded network tokens for specific domains. Once the token expires, the smoltcp proxy automatically severs the TCP connection. Furthermore, the architecture will implement AES-encrypted virtual block devices for specific paths (e.g., /home/agent/.memory). These VHDX/QCOW2 files are mounted at boot, providing a secure, persistent blackboard that survives VM reboots.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P1** | Persistent Encrypted Volumes | Support mounting persistent state files into the agent workspace. |
| **P2** | Windows GPU-P Integration | Expose virtualized GPU compute to the guest via HCS/dxgkrnl.43 |

### **5\. Open Questions**

Can dxgkrnl be reliably initialized within a minimalistic Linux microVM guest without pulling in hundreds of megabytes of proprietary WSL2 libraries?

## **L. Integration with Savant's Existing Security**

### **1\. Current State**

Savant relies on Crypto-Capability Tokens (CCT), which suffer from shared signing keys across the swarm, a lack of proof-of-possession, and no revocation mechanisms. The SovereignShell attempts to sanitize inputs using regex string matching.

### **2\. Gaps and Risks**

CCT tokens act as simple bearer tokens. If intercepted, an attacker can impersonate an agent. Furthermore, the SovereignShell relies on easily bypassed regex rules for security, allowing malicious payloads to escape containment via encoding or indirection.

### **3\. Design Recommendations**

The CCT system must be fundamentally restructured. Instead of bearer tokens, capabilities are mapped directly to the cryptographic identity established during the Noise\_XX handshake.17 An agent proves possession of its capability by signing requests over the vsock IPC layer using its TOFU-established private key.18

SovereignShell's regex blocking must be explicitly deprecated. Command execution should be handled entirely by the hardened agentd process within the microVM, where the strict hardware isolation, read-only rootfs, and lack of network routing render dangerous commands (e.g., curl | sh, rm \-rf /) harmless by structural design, rather than behavioral pattern matching. Savant's existing Prompt Defense and Skill Security Scanners must be executed in the host broker *before* data is passed over vsock into the VM.

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | Deprecate SovereignShell | Replace regex-based shell security with microVM containment. |
| **P0** | Cryptographic IPC Identity | Bind CCT capabilities to the vsock Noise protocol keypair. |

### **5\. Open Questions**

How seamlessly can existing WASM/Docker skills be transparently migrated into the new microVM execution model without breaking backward compatibility for Savant users?

## **M. Windows-Specific Design**

### **1\. Current State**

Savant neglects Windows security entirely, relying on shell execution policies. Other Rust projects like fastrender implement complex fallbacks using Job Objects and AppContainers 1, while modern virtualization leans heavily on HCS and WHP.

### **2\. Gaps and Risks**

Relying on legacy APIs creates a brittle security boundary. Virtualization-Based Security (VBS) and Hyper-V provide genuine hardware isolation, but the programming interfaces are historically undocumented or incredibly complex, leading to widespread neglect in cross-platform tools.

### **3\. Design Recommendations**

Windows must be treated as a Tier 1 citizen using a layered approach. The primary Windows backend should leverage the Host Compute Service (HCS) via zlayer-hcs.8 HCS allows the programmatic creation of Krypton-isolated utility VMs.44 The sandbox generates an HCS Schema v2 JSON configuration specifying the base layers, network endpoints, and GPU-P assignments 29, and submits it to vmcompute.exe via the CreateComputeSystem Rust binding.8

For lower latency, directly wrapping the Windows Hypervisor Platform (WHP) using libwhp 46 or cloud-hypervisor 4 provides a KVM-equivalent API, bypassing the overhead of standard Windows container initialization.

If Hyper-V is unavailable, the sandbox drops to process isolation. Using techniques pioneered in fastrender, the agent is executed with an AppContainer token featuring zero capabilities, restricted by a Job Object (KILL\_ON\_JOB\_CLOSE), and utilizing a strict handle-inheritance allowlist via PROC\_THREAD\_ATTRIBUTE\_HANDLE\_LIST to prevent capability leakage.1

### **4\. Implementation Priority**

| Priority | Component | Description |
| :---- | :---- | :---- |
| **P0** | HCS Integration | Utilize zlayer-hcs to build Hyper-V containers natively.8 |
| **P1** | AppContainer Fallback | Implement the fastrender zero-capability Job Object model.1 |

### **5\. Open Questions**

Does utilizing HCS require the user to have Hyper-V explicitly enabled via "Turn Windows features on or off", and how do we automate this prerequisite check during the Savant installation process?

## **N. Gaps to Address from the Research**

A synthesis of the current Savant architecture and the reference microsandbox reveals systemic weaknesses that this new architecture explicitly resolves:

1. **Tamper-Proof Audit Logging:** Addressed via continuous Merkle tree hash chaining of guest actions streamed over the authenticated vsock layer.34  
2. **OCI Supply Chain Verification:** Addressed via required sigstore-rs / cosign cryptographic validation before block device materialization.38  
3. **Forensic Capture:** Addressed via VMM-triggered snapshotting of the virtio-blk upper layer and placeholder-only core dumps upon crash.  
4. **Per-Sandbox Disk Quotas:** Addressed via XFS project quotas (Linux) and mathematically restricted dynamically expanding VHDX containers (Windows).29  
5. **Serial Protocol Weaknesses:** Addressed by entirely replacing unauthenticated virtio-serial with POSIX-compliant virtio-vsock (AF\_HYPERV on Windows) secured by the mutual-authentication Noise Protocol Framework (Noise\_XX).2  
6. **CCT System Vulnerabilities:** Addressed by enforcing Proof-of-Possession through the asymmetric keypair generated inside the guest, validated against the host broker during IPC requests.18

## ---

**System Realization**

### **Recommended Build Order**

To transition Savant safely from an insecure state to a fully hardened microVM architecture, development must follow a strict sequential path, ensuring foundational security primitives are stable before building hypervisor abstractions.

| Phase | Focus Area | Key Objectives |
| :---- | :---- | :---- |
| **Phase 1** | Cryptographic Primitives & IPC | Build the zeroize \+ VirtualLock/mlock secure memory struct layer.21 Implement the Noise Protocol Framework handshake library over standard TCP sockets for baseline testing.17 |
| **Phase 2** | Platform-Agnostic Process Hardening | Implement the VMM host broker process hardening: seccomp-bpf \+ Landlock on Linux; AppContainer \+ Job Objects on Windows.1 This guarantees that even a flawed hypervisor implementation cannot easily compromise the host. |
| **Phase 3** | The Hypervisor Abstraction Layer | Integrate cloud-hypervisor as the primary KVM backend for Linux. Integrate zlayer-hcs as the primary Hyper-V container backend for Windows.8 Implement virtio-vsock (Linux) and AF\_HYPERV (Windows) bridging.19 |
| **Phase 4** | Filesystem & Supply Chain | Integrate sigstore-rs for OCI image verification.38 Implement EROFS block mounting and XFS/VHDX writable overlay systems with strict disk quotas. |
| **Phase 5** | Networking & Secrets | Implement the smoltcp in-process user-space stack.20 Build the DNS rebinding proxy and the protocol-agnostic TLS secret substitution engine.27 |
| **Phase 6** | Integration & Auditing | Implement the hash-chained audit log mechanism.34 Replace SovereignShell and the legacy WASM sandbox with the new unified microVM executor. |

### **Module Decomposition (Rust Crate Structure)**

The new system should be logically partitioned and housed within the Savant workspace under crates/sandbox, organized to strictly delineate security boundaries:

* savant-sandbox (Primary orchestration entry point, configures and spawns workloads)  
  * savant-sandbox-vmm (Abstracts hypervisor APIs)  
    * backend\_cloudhypervisor (Linux KVM / Windows WHP implementation)  
    * backend\_hcs (Windows Host Compute Service implementation via zlayer-hcs)  
    * backend\_macos (Virtualization.framework implementation via vfrust)  
    * backend\_process (Fallback AppContainer / seccomp implementation)  
  * savant-sandbox-ipc (Host-Guest communication layer)  
    * vsock\_bridge (Abstracts AF\_VSOCK and AF\_HYPERV)  
    * noise\_crypto (Implements Noise\_XX handshake and packet encryption)  
  * savant-sandbox-net (Networking isolation and inspection)  
    * smoltcp\_stack (User-space TCP/IP processing)  
    * dns\_interceptor (Anti-rebinding policy engine)  
    * tls\_proxy (Secret substitution layer)  
  * savant-sandbox-fs (Block layer and image management)  
    * oci\_verifier (Sigstore/Cosign integration)  
    * block\_quota (XFS / VHDX orchestration)  
  * savant-sandbox-secure (Host memory protection)  
    * locked\_memory (mlock / VirtualLock / PAGE\_NOACCESS lease logic)  
    * audit\_chain (Merkle tree / append-only logging mechanisms)

### **Recommended Third-Party Rust Crates**

| Crate | Purpose | Justification |
| :---- | :---- | :---- |
| cloud-hypervisor | VMM Backend | High-performance, Rust-native VMM supporting both Linux KVM and Windows MSHV/WHP.4 |
| zlayer-hcs | Windows Containers | Safe, asynchronous RAII wrapper over vmcompute.dll for HCS schema v2 container orchestration.8 |
| vfrust | macOS Virtualization | Manages Virtualization.framework, natively supporting virtiofs and Rosetta.11 |
| smoltcp | Networking | Tap-less, in-process, user-space TCP/IP stack required for hyper-secure guest network isolation.20 |
| sigstore-rs | Image Security | Official Rust bindings for Sigstore/Cosign, enabling keyless OCI image verification and Fulcio checks.37 |
| snow | IPC Security | Industry-standard implementation of the Noise Protocol Framework for secure vsock handshakes.16 |
| zeroize | Memory Security | Stably overwrites sensitive memory regions on drop, preventing compiler bypasses.21 |
| immutable\_logging | Audit Logging | Provides cryptographic hash chaining and Merkle roots required for tamper-proof telemetry.34 |
| winapi / windows-sys | Windows Hardening | Required for interacting directly with VirtualLock, VirtualProtect, AppContainers, and Job Objects.1 |

#### **Works cited**

1. fastrender/docs/windows\_sandbox.md at main · wilsonzlin ... \- GitHub, accessed May 17, 2026, [https://github.com/wilsonzlin/fastrender/blob/main/docs/windows\_sandbox.md](https://github.com/wilsonzlin/fastrender/blob/main/docs/windows_sandbox.md)  
2. Features/VirtioVsock \- QEMU, accessed May 17, 2026, [https://wiki.qemu.org/Features/VirtioVsock](https://wiki.qemu.org/Features/VirtioVsock)  
3. virtio-vsock — configuration-agnostic guest/host communication, accessed May 17, 2026, [https://www.net.in.tum.de/fileadmin/TUM/NET/NET-2019-10-1/NET-2019-10-1\_14.pdf](https://www.net.in.tum.de/fileadmin/TUM/NET/NET-2019-10-1/NET-2019-10-1_14.pdf)  
4. cloud-hypervisor/docs/windows.md at main \- GitHub, accessed May 17, 2026, [https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/docs/windows.md](https://github.com/cloud-hypervisor/cloud-hypervisor/blob/main/docs/windows.md)  
5. GitHub \- cloud-hypervisor/cloud-hypervisor: A Virtual Machine Monitor for modern Cloud workloads. Features include CPU, memory and device hotplug, support for running Windows and Linux guests, device offload with vhost-user and a minimal compact footprint. Written in Rust with a strong focus on security., accessed May 17, 2026, [https://github.com/cloud-hypervisor/cloud-hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor)  
6. Cloud Hypervisor \- Run Cloud Virtual Machines Securely and Efficiently, accessed May 17, 2026, [https://www.cloudhypervisor.org/](https://www.cloudhypervisor.org/)  
7. Linux\_4.8 \- Linux Kernel Newbies, accessed May 17, 2026, [https://kernelnewbies.org/Linux\_4.8](https://kernelnewbies.org/Linux_4.8)  
8. zlayer-hcs \- Lib.rs, accessed May 17, 2026, [https://lib.rs/crates/zlayer-hcs](https://lib.rs/crates/zlayer-hcs)  
9. hyperlight-host — embedded dev in Rust // Lib.rs, accessed May 17, 2026, [https://lib.rs/crates/hyperlight-host](https://lib.rs/crates/hyperlight-host)  
10. GitHub \- hyperlight-dev/hyperlight: Hyperlight is a lightweight Virtual Machine Manager (VMM) designed to be embedded within applications. It enables safe execution of untrusted code within micro virtual machines with very low latency and minimal overhead., accessed May 17, 2026, [https://github.com/hyperlight-dev/hyperlight](https://github.com/hyperlight-dev/hyperlight)  
11. vfrust \- crates.io: Rust Package Registry, accessed May 17, 2026, [https://crates.io/crates/vfrust](https://crates.io/crates/vfrust)  
12. applevisor \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/applevisor](https://docs.rs/applevisor)  
13. Running Intel Binaries in Linux VMs with Rosetta | Apple Developer Documentation, accessed May 17, 2026, [https://developer.apple.com/documentation/virtualization/running-intel-binaries-in-linux-vms-with-rosetta](https://developer.apple.com/documentation/virtualization/running-intel-binaries-in-linux-vms-with-rosetta)  
14. hvsock 3.1.0 · OCaml Package, accessed May 17, 2026, [https://ocaml.org/p/hvsock/3.1.0](https://ocaml.org/p/hvsock/3.1.0)  
15. Enable virtualization \- Qualcomm Linux Kernel Guide, accessed May 17, 2026, [https://docs.qualcomm.com/bundle/publicresource/topics/80-70020-3/virtualization.html](https://docs.qualcomm.com/bundle/publicresource/topics/80-70020-3/virtualization.html)  
16. Cryptography — list of Rust libraries/crates // Lib.rs, accessed May 17, 2026, [https://lib.rs/cryptography](https://lib.rs/cryptography)  
17. 1Password's Confidential Computing System Design Review – Report, accessed May 17, 2026, [https://bucket.agilebits.com/security/Zxs\_Confidential-Computing-Platform-Review-Report.pdf](https://bucket.agilebits.com/security/Zxs_Confidential-Computing-Platform-Review-Report.pdf)  
18. IPC via TCP Sockets · Issue \#32802 · bitcoin/bitcoin \- GitHub, accessed May 17, 2026, [https://github.com/bitcoin/bitcoin/issues/32802](https://github.com/bitcoin/bitcoin/issues/32802)  
19. accessed May 17, 2026, [https://gitea.com/gitea/tea/pulls/478.patch](https://gitea.com/gitea/tea/pulls/478.patch)  
20. microsandbox-utils \- Lib.rs, accessed May 17, 2026, [https://lib.rs/crates/microsandbox-utils](https://lib.rs/crates/microsandbox-utils)  
21. zeroize \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/zeroize/latest/zeroize/](https://docs.rs/zeroize/latest/zeroize/)  
22. I built a NuGet package that locks your secrets in RAM and makes them invisible to the OS when not in use \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/dotnet/comments/1s55w48/i\_built\_a\_nuget\_package\_that\_locks\_your\_secrets/](https://www.reddit.com/r/dotnet/comments/1s55w48/i_built_a_nuget_package_that_locks_your_secrets/)  
23. secure\_types \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/secure-types](https://docs.rs/secure-types)  
24. I built a NuGet package that locks your secrets in RAM and makes them invisible to the OS when not in use \- Reddit, accessed May 17, 2026, [https://www.reddit.com/r/csharp/comments/1s4352f/i\_built\_a\_nuget\_package\_that\_locks\_your\_secrets/](https://www.reddit.com/r/csharp/comments/1s4352f/i_built_a_nuget_package_that_locks_your_secrets/)  
25. Unexpected page handling (also, VirtualLock \= no op?) \- Stack Overflow, accessed May 17, 2026, [https://stackoverflow.com/questions/7874281/unexpected-page-handling-also-virtuallock-no-op](https://stackoverflow.com/questions/7874281/unexpected-page-handling-also-virtuallock-no-op)  
26. Managing Virtual Memory in Win32 \- LaBRI, accessed May 17, 2026, [https://www.labri.fr/perso/betrema/winnt/virtmm.html](https://www.labri.fr/perso/betrema/winnt/virtmm.html)  
27. microsandbox\_network \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/microsandbox-network](https://docs.rs/microsandbox-network)  
28. Enhancing Network Interception with Mitmproxy \- WebThesis, accessed May 17, 2026, [https://webthesis.biblio.polito.it/31361/1/tesi.pdf](https://webthesis.biblio.polito.it/31361/1/tesi.pdf)  
29. Which Hyper-V VM Generation does docker use for docker run \--isolation=hyperv? · Issue \#382 · microsoft/Windows-Containers \- GitHub, accessed May 17, 2026, [https://github.com/microsoft/Windows-Containers/issues/382](https://github.com/microsoft/Windows-Containers/issues/382)  
30. How to Use Docker Desktop File Sharing Settings \- OneUptime, accessed May 17, 2026, [https://oneuptime.com/blog/post/2026-02-08-how-to-use-docker-desktop-file-sharing-settings/view](https://oneuptime.com/blog/post/2026-02-08-how-to-use-docker-desktop-file-sharing-settings/view)  
31. Speed boost achievement unlocked on Docker Desktop 4.6 for Mac, accessed May 17, 2026, [https://www.docker.com/blog/speed-boost-achievement-unlocked-on-docker-desktop-4-6-for-mac/](https://www.docker.com/blog/speed-boost-achievement-unlocked-on-docker-desktop-4-6-for-mac/)  
32. What you can do with lightweight VM Shared Folders, and what you can't \- The Eclectic Light Company, accessed May 17, 2026, [https://eclecticlight.co/2023/10/19/what-you-can-do-with-lightweight-vm-shared-folders-and-what-you-cant/](https://eclecticlight.co/2023/10/19/what-you-can-do-with-lightweight-vm-shared-folders-and-what-you-cant/)  
33. awesome-rust 0.1.0 \- Docs.rs, accessed May 17, 2026, [https://docs.rs/crate/awesome-rust/0.1.0/source/public/index.html](https://docs.rs/crate/awesome-rust/0.1.0/source/public/index.html)  
34. immutable\_logging \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/rsrp-immutable-ledger](https://docs.rs/rsrp-immutable-ledger)  
35. DriftDB \- An experimental append-only database with built-in time travel. Query any point in history, guaranteed data integrity, and immutable audit trails. Written in Rust. · GitHub, accessed May 17, 2026, [https://github.com/DavidLiedle/DriftDB](https://github.com/DavidLiedle/DriftDB)  
36. sentinel-crypto \- crates.io: Rust Package Registry, accessed May 17, 2026, [https://crates.io/crates/sentinel-crypto](https://crates.io/crates/sentinel-crypto)  
37. sigstore/sigstore-rs: An experimental Rust crate for sigstore \- GitHub, accessed May 17, 2026, [https://github.com/sigstore/sigstore-rs](https://github.com/sigstore/sigstore-rs)  
38. sigstore \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/sigstore](https://docs.rs/sigstore)  
39. Sigstore Quickstart with Cosign, accessed May 17, 2026, [https://docs.sigstore.dev/quickstart/quickstart-cosign/](https://docs.sigstore.dev/quickstart/quickstart-cosign/)  
40. Sigstore implemented in Rust for Github v0.3 bundles, accessed May 17, 2026, [https://github.com/prefix-dev/sigstore-rust](https://github.com/prefix-dev/sigstore-rust)  
41. Hyper-V Virtualization and Security Overview | PDF \- Scribd, accessed May 17, 2026, [https://www.scribd.com/document/790954991/Windows-Server-Virtualization](https://www.scribd.com/document/790954991/Windows-Server-Virtualization)  
42. How to use Ubuntu on Windows, accessed May 17, 2026, [https://ubuntu.com/blog/how-to-use-ubuntu-on-windows](https://ubuntu.com/blog/how-to-use-ubuntu-on-windows)  
43. This entire concept of having directx in WSL sounds like Embrace Extend Extinghi... | Hacker News, accessed May 17, 2026, [https://news.ycombinator.com/item?id=34782717](https://news.ycombinator.com/item?id=34782717)  
44. Run Linux and Windows Containers on Windows 10 \- Stefan Scherer's Blog, accessed May 17, 2026, [https://stefanscherer.github.io/run-linux-and-windows-containers-on-windows-10/](https://stefanscherer.github.io/run-linux-and-windows-containers-on-windows-10/)  
45. container.go \- microsoft/hcsshim \- GitHub, accessed May 17, 2026, [https://github.com/microsoft/hcsshim/blob/main/container.go](https://github.com/microsoft/hcsshim/blob/main/container.go)  
46. insula-rs/libwhp: Windows Hypervisor Platform Rust crate \- GitHub, accessed May 17, 2026, [https://github.com/insula-rs/libwhp](https://github.com/insula-rs/libwhp)  
47. "" Search \- Rust \- Docs.rs, accessed May 17, 2026, [https://docs.rs/windows-sys/latest/windows\_sys/Win32/Networking/WinSock/type.SET\_SERVICE\_OPERATION.html?search=](https://docs.rs/windows-sys/latest/windows_sys/Win32/Networking/WinSock/type.SET_SERVICE_OPERATION.html?search)