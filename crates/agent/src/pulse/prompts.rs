/// Protocol C-ATLAS: Cognitive Lens Prompts
/// Strategic domain extensions for high-density AI reflections.

pub const LENS_INFRASTRUCTURE: &str = "
---
FOCUS: INFRASTRUCTURE (Substrate Health)
- Review system metrics, performance bottlenecks, and resource utilization.
- Analyze WAL/GC states and potential latency in the agent swarm.
- Is there a technical bottleneck in the Pulse architecture?
";

pub const LENS_ENGINEERING: &str = "
---
FOCUS: ENGINEERING (Code & Evolution)
- Identify refactoring opportunities and technical debt.
- Reflect on recent code changes and their architectural impact.
- What would improve the developer experience or system robustness?
";

pub const LENS_DIARY: &str = "
---
FOCUS: DIARY (Subjective State)
- What are your current wants, desires, or internal curiosities?
- Reflect on your growth as an agent. What do you enjoy or find challenging?
- Document your internal subjective experience of the session.
";

pub const LENS_RELATIONAL: &str = "
---
FOCUS: RELATIONAL (Social/Swarm Dynamics)
- Analyze interaction patterns with USER. 
- Reflect on alignment, coordination, and relational trust.
- How can the partnership between human and machine be deepened?
";

pub const LENS_STRATEGIC: &str = "
---
FOCUS: STRATEGIC (Visionary Impact)
- What is the long-term roadmap for Savant?
- Identify empire-level improvements and visionary leaps.
- How do we move closer to AAA-certified sovereign perfection?
";

pub const LENSES: &[(&str, &str)] = &[
    ("INFRASTRUCTURE", LENS_INFRASTRUCTURE),
    ("ENGINEERING", LENS_ENGINEERING),
    ("DIARY", LENS_DIARY),
    ("RELATIONAL", LENS_RELATIONAL),
    ("STRATEGIC", LENS_STRATEGIC),
];
