"use client";

import { useDashboard } from "@/context/DashboardContext";

export default function EvolutionPage() {
  const ctx = useDashboard();
  const { evolutionScore, proposedMutations, mutationHistory, traitSnapshots, sendControlFrame } = ctx;

  const score = evolutionScore as any;
  const stage = score?.stage || "Seedling";
  const evolutionScoreVal = score?.evolution_score || 0;

  return (
    <div style={{ padding: "24px", height: "100%", overflow: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "24px" }}>
        <div>
          <h2 style={{ color: "var(--accent)", letterSpacing: "2px", margin: 0 }}>PERSONALITY EVOLUTION</h2>
          <div style={{ opacity: 0.5, fontSize: "10px", marginTop: "4px" }}>SOVEREIGN GROWTH</div>
        </div>
        <div style={{ textAlign: "right" }}>
          <div style={{ fontSize: "24px", fontWeight: 800, color: "var(--accent)" }}>
            {evolutionScoreVal.toFixed(1)}
          </div>
          <div style={{ fontSize: "11px", opacity: 0.6 }}>{stage}</div>
        </div>
      </div>

      {/* OCEAN Trait Gauges */}
      {traitSnapshots.length > 0 && (
        <div style={{ marginBottom: "24px" }}>
          <h3 style={{ fontSize: "11px", fontWeight: 800, textTransform: "uppercase", opacity: 0.7, marginBottom: "12px" }}>OCEAN Traits</h3>
          {["openness", "conscientiousness", "extraversion", "agreeableness", "neuroticism"].map((trait) => {
            const val = (traitSnapshots[0] as any)?.[trait] || 0.5;
            return (
              <div key={trait} style={{ marginBottom: "8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "10px", marginBottom: "2px" }}>
                  <span style={{ textTransform: "capitalize" }}>{trait}</span>
                  <span style={{ opacity: 0.5 }}>{(val * 100).toFixed(0)}%</span>
                </div>
                <div style={{ height: "4px", background: "rgba(255,255,255,0.05)", borderRadius: "2px" }}>
                  <div style={{ width: `${val * 100}%`, height: "100%", background: "var(--accent)", borderRadius: "2px", transition: "width 0.5s" }} />
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Pending Mutations */}
      <div style={{ marginBottom: "24px" }}>
        <h3 style={{ fontSize: "11px", fontWeight: 800, textTransform: "uppercase", opacity: 0.7, marginBottom: "12px" }}>
          Pending Mutations ({proposedMutations.length})
        </h3>
        {proposedMutations.map((m: any, i: number) => (
          <div key={i} style={{ background: "rgba(255,255,255,0.03)", border: "1px solid var(--border)", borderRadius: "4px", padding: "12px", marginBottom: "8px" }}>
            <div style={{ fontSize: "10px", fontWeight: 800, opacity: 0.7, marginBottom: "4px" }}>
              {m.mutation_type?.toUpperCase()} → {m.target_section}
            </div>
            <div style={{ fontSize: "11px", opacity: 0.6, marginBottom: "8px" }}>{m.reasoning?.slice(0, 200)}</div>
            <div style={{ display: "flex", gap: "6px" }}>
              <button onClick={() => sendControlFrame("SoulMutationApprove", { agent_id: m.agent_id, mutation_id: m.mutation_id })} style={{ background: "#fff", color: "#000", border: "none", borderRadius: "4px", padding: "4px 12px", fontSize: "10px", cursor: "pointer" }}>Approve</button>
              <button onClick={() => sendControlFrame("SoulMutationReject", { agent_id: m.agent_id, mutation_id: m.mutation_id, reason: "User rejected" })} style={{ background: "transparent", border: "1px solid var(--border)", color: "#fff", borderRadius: "4px", padding: "4px 12px", fontSize: "10px", cursor: "pointer" }}>Reject</button>
            </div>
          </div>
        ))}
      </div>

      {/* Mutation History */}
      <div>
        <h3 style={{ fontSize: "11px", fontWeight: 800, textTransform: "uppercase", opacity: 0.7, marginBottom: "12px" }}>
          Mutation History ({mutationHistory.length})
        </h3>
        {mutationHistory.map((m: any, i: number) => (
          <div key={i} style={{ padding: "6px 0", borderBottom: "1px solid rgba(255,255,255,0.03)", fontSize: "10px", opacity: 0.5 }}>
            <span style={{ color: m.status === "approved" ? "var(--accent)" : "rgba(255,100,100,0.7)" }}>
              {m.status?.toUpperCase()}
            </span>
            {" — "}{m.target_section || m.mutation_type}
          </div>
        ))}
      </div>
    </div>
  );
}
