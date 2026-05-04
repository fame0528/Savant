"use client";

import { useDashboard } from "@/context/DashboardContext";

export default function BehindTheCurtainPage() {
  const ctx = useDashboard();
  const { proposedMutations, mutationHistory, traitSnapshots, evolutionScore } = ctx;

  const score = evolutionScore as any;
  const totalMutations = mutationHistory.length;

  return (
    <div style={{ padding: "24px", height: "100%", overflow: "auto" }}>
      <div style={{ marginBottom: "24px" }}>
        <h2 style={{ color: "var(--accent)", letterSpacing: "2px", margin: 0 }}>BEHIND THE CURTAIN</h2>
        <div style={{ opacity: 0.5, fontSize: "10px", marginTop: "4px" }}>Evolution Engine Internals</div>
      </div>

      {/* Evolution Stats */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "12px", marginBottom: "24px" }}>
        <div style={{ background: "rgba(255,255,255,0.03)", border: "1px solid var(--border)", borderRadius: "4px", padding: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "20px", fontWeight: 800, color: "var(--accent)" }}>{totalMutations}</div>
          <div style={{ fontSize: "9px", opacity: 0.5, textTransform: "uppercase" }}>Total Mutations</div>
        </div>
        <div style={{ background: "rgba(255,255,255,0.03)", border: "1px solid var(--border)", borderRadius: "4px", padding: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "20px", fontWeight: 800, color: "var(--accent)" }}>
            {(score?.evolution_score || 0).toFixed(1)}
          </div>
          <div style={{ fontSize: "9px", opacity: 0.5, textTransform: "uppercase" }}>Evolution Score</div>
        </div>
        <div style={{ background: "rgba(255,255,255,0.03)", border: "1px solid var(--border)", borderRadius: "4px", padding: "12px", textAlign: "center" }}>
          <div style={{ fontSize: "20px", fontWeight: 800, color: "var(--accent)" }}>{proposedMutations.length}</div>
          <div style={{ fontSize: "9px", opacity: 0.5, textTransform: "uppercase" }}>Pending</div>
        </div>
      </div>

      {/* OCEAN Drift Velocity */}
      {traitSnapshots.length >= 2 && (
        <div style={{ marginBottom: "24px" }}>
          <h3 style={{ fontSize: "11px", fontWeight: 800, textTransform: "uppercase", opacity: 0.7, marginBottom: "12px" }}>OCEAN Drift Velocity (Δ / week)</h3>
          {["openness", "conscientiousness", "extraversion", "agreeableness", "neuroticism"].map((trait) => {
            const current = (traitSnapshots[0] as any)?.[trait] || 0.5;
            const prev = (traitSnapshots[1] as any)?.[trait] || 0.5;
            const delta = current - prev;
            const color = delta > 0 ? "var(--accent)" : "rgba(255,100,100,0.7)";
            return (
              <div key={trait} style={{ marginBottom: "8px" }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: "10px" }}>
                  <span style={{ textTransform: "capitalize" }}>{trait}</span>
                  <span style={{ color, fontWeight: 800 }}>
                    {delta > 0 ? "+" : ""}{delta.toFixed(3)}
                  </span>
                </div>
                <div style={{ height: "3px", background: "rgba(255,255,255,0.03)", borderRadius: "2px", marginTop: "2px" }}>
                  <div style={{ width: `${Math.abs(delta) * 500}%`, height: "100%", background: color, borderRadius: "2px", transition: "width 0.5s", maxWidth: "100%" }} />
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Rejection Archive */}
      <div style={{ marginBottom: "24px" }}>
        <h3 style={{ fontSize: "11px", fontWeight: 800, textTransform: "uppercase", opacity: 0.7, marginBottom: "12px" }}>Rejection Archive</h3>
        {mutationHistory.filter((m: any) => m.status === "rejected").length === 0 ? (
          <div style={{ opacity: 0.3, fontSize: "11px", padding: "12px", textAlign: "center" }}>
            No rejected mutations yet.
          </div>
        ) : (
          mutationHistory.filter((m: any) => m.status === "rejected").map((m: any, i: number) => (
            <div key={i} style={{ padding: "6px 0", borderBottom: "1px solid rgba(255,255,255,0.03)", fontSize: "10px", opacity: 0.5 }}>
              <span style={{ color: "rgba(255,100,100,0.7)" }}>REJECTED</span>
              {" — "}{m.target_section || m.mutation_type}
              {m.reason && <span style={{ marginLeft: "8px", opacity: 0.3 }}>{m.reason}</span>}
            </div>
          ))
        )}
      </div>

      {/* Section Heatmap */}
      <div>
        <h3 style={{ fontSize: "11px", fontWeight: 800, textTransform: "uppercase", opacity: 0.7, marginBottom: "12px" }}>Section Heatmap</h3>
        {(() => {
          const sectionCounts: Record<string, number> = {};
          mutationHistory.forEach((m: any) => {
            const section = m.target_section || "unknown";
            sectionCounts[section] = (sectionCounts[section] || 0) + 1;
          });
          const maxCount = Math.max(1, ...Object.values(sectionCounts));
          return Object.entries(sectionCounts).map(([section, count]) => (
            <div key={section} style={{ marginBottom: "6px" }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "10px", marginBottom: "2px" }}>
                <span>{section}</span>
                <span style={{ opacity: 0.5 }}>{count}</span>
              </div>
              <div style={{ height: "4px", background: "rgba(255,255,255,0.03)", borderRadius: "2px" }}>
                <div style={{ width: `${(count / maxCount) * 100}%`, height: "100%", background: "var(--accent)", borderRadius: "2px" }} />
              </div>
            </div>
          ));
        })()}
      </div>
    </div>
  );
}
