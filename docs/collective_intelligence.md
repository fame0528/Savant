# Collective Intelligence: Multi-Agent Consensus & Sharing

## Overview

The Collective Intelligence substrate enables zero-copy state sharing and multi-agent consensus protocols. It ensures that the Savant Swarm operates as a unified cognitive entity rather than a collection of isolated agents.

## Core Components

### 1. Collective Blackboard (`savant_ipc::collective`)

- **Technology**: Built on `iceoryx2` for true zero-copy shared memory.
- **State**: Maintains global swarm metrics, heuristic versions, and task-allocation maps.
- **Performance**: Sub-microsecond latency for cross-process state propagation.

### 2. Swarm Consensus Protocol (`savant_ipc::consensus`)

- **Voting**: Agents propose high-risk actions (e.g., file deletions, network config changes).
- **Quorum**: A minimum of 3 approvals (or a weighted majority) is required for execution.
- **Rejection**: Lone-agent "runaway" scenarios are blocked by the proposal phase in the `AgentLoop`.

### 3. Cross-Agent Reflection

- **Mechanism**: Agents publish cognitive insights to the blackboard after every task.
- **Propagation**: Other agents subscribe to these insights to adapt their local predictors (`savant_cognitive`).

## Protocols

### Proposal Phase

1. Agent proposes `ToolAction`.
2. Blackboard broadcasts proposal to peers.
3. Peer agents simulate action and return `Approval` or `Veto`.
4. If quorum reached, action executes; otherwise, it is logged and aborted.

### Heuristic Synchronization

- Every insight increment increases the `heuristic_version`.
- Agents with lower local versions automatically pull the latest cognitive substrate before starting a new `AgentLoop`.
