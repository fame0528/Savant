use std::collections::{HashSet};

/// A node in the speculative execution trajectory.
#[derive(Debug, Clone)]
pub struct SpeculativeNode {
    pub name: String,
    pub args: String,
    pub dependencies: HashSet<usize>, // Indicies of parent nodes in the execution sequence
}

/// A DAG representing a speculative execution plan.
/// 
/// Allows the agent to propose multiple actions that can be executed 
/// together if their dependencies are met.
#[derive(Debug, Clone, Default)]
pub struct SpeculativeDag {
    pub nodes: Vec<SpeculativeNode>,
}

impl SpeculativeDag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new speculative action to the plan.
    pub fn add_step(&mut self, name: String, args: String, deps: Vec<usize>) {
        self.nodes.push(SpeculativeNode {
            name,
            args,
            dependencies: deps.into_iter().collect(),
        });
    }

    /// Returns the number of speculative steps in the plan.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the plan is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Parses a list of actions into a simple sequential DAG.
/// 
/// In future iterations, this will use LLM markers or structural 
/// analysis to detect independent parallel actions.
pub fn parse_sequential_dag(actions: Vec<(String, String)>) -> SpeculativeDag {
    let mut dag = SpeculativeDag::new();
    for (i, (name, args)) in actions.into_iter().enumerate() {
        let deps = if i > 0 { vec![i - 1] } else { vec![] };
        dag.add_step(name, args, deps);
    }
    dag
}
