use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── Pet Stages (mapped to evolution) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PetStage {
    Egg,
    Seedling,
    Sprout,
    Bloom,
    Sovereign,
}

impl PetStage {
    pub fn decay_rate_multiplier(&self) -> f64 {
        match self {
            PetStage::Egg => 0.0,
            PetStage::Seedling => 1.5,
            PetStage::Sprout => 1.2,
            PetStage::Bloom => 1.0,
            PetStage::Sovereign => 0.7,
        }
    }

    pub fn from_evolution_stage(s: &str) -> Self {
        match s {
            "Growing" => PetStage::Sprout,
            "Mature" => PetStage::Bloom,
            "Sovereign" => PetStage::Sovereign,
            _ => PetStage::Seedling,
        }
    }

    pub fn display_icon(&self) -> &str {
        match self {
            PetStage::Egg => "\u{1f95a}",
            PetStage::Seedling => "\u{1f331}",
            PetStage::Sprout => "\u{1f33f}",
            PetStage::Bloom => "\u{1f33a}",
            PetStage::Sovereign => "\u{1f451}",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            PetStage::Egg => "Egg",
            PetStage::Seedling => "Seedling",
            PetStage::Sprout => "Sprout",
            PetStage::Bloom => "Bloom",
            PetStage::Sovereign => "Sovereign",
        }
    }
}

// ── Mood ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PetMood {
    Happy,
    Content,
    Hungry,
    Sick,
    Sad,
    Sleeping,
    Dead,
}

impl PetMood {
    pub fn display_icon(&self) -> &str {
        match self {
            PetMood::Happy => "\u{1f60a}",
            PetMood::Content => "\u{1f610}",
            PetMood::Hungry => "\u{1f62e}",
            PetMood::Sick => "\u{1f912}",
            PetMood::Sad => "\u{1f622}",
            PetMood::Sleeping => "\u{1f634}",
            PetMood::Dead => "\u{1f480}",
        }
    }
}

// ── Care Actions ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedType {
    Meal,
    Snack,
}

// ── Achievements ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub unlocked_at: Option<i64>,
}

// ── Core Pet State ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetData {
    // Tamagotchi core stats (0.0 - 1.0)
    pub hunger: f64,
    pub happiness: f64,
    pub discipline: f64,
    pub health: f64,

    // Physical state
    pub weight: f64,
    pub cleanliness: f64,
    pub age_hours: f64,

    // Flags
    pub is_sick: bool,
    pub is_sleeping: bool,
    pub is_dead: bool,
    pub poop_count: u32,

    // Tracked metrics
    pub care_mistakes: u32,
    pub times_fed: u64,
    pub times_played: u64,
    pub times_cleaned: u64,

    // Last update timestamp for decay calculation
    pub last_update: i64,
}

impl PetData {
    fn new() -> Self {
        Self {
            hunger: 1.0,
            happiness: 0.7,
            discipline: 0.5,
            health: 1.0,
            weight: 1.0,
            cleanliness: 1.0,
            age_hours: 0.0,
            is_sick: false,
            is_sleeping: false,
            is_dead: false,
            poop_count: 0,
            care_mistakes: 0,
            times_fed: 0,
            times_played: 0,
            times_cleaned: 0,
            last_update: Utc::now().timestamp(),
        }
    }

    /// Apply time-based decay since the last update.
    /// Returns notifications for any significant events (got sick, etc.).
    fn tick(&mut self, now: &DateTime<Utc>, stage: &PetStage) -> Vec<String> {
        if self.is_dead {
            return Vec::new();
        }

        let elapsed_secs = (now.timestamp() - self.last_update).max(0) as f64;
        self.last_update = now.timestamp();

        // Age
        self.age_hours += elapsed_secs / 3600.0;

        let multiplier = stage.decay_rate_multiplier();
        let hours = elapsed_secs / 3600.0;

        // Hunger decays fastest
        self.hunger = (self.hunger - 0.04 * multiplier * hours).clamp(0.0, 1.0);

        // Happiness decays moderately
        self.happiness = (self.happiness - 0.025 * multiplier * hours).clamp(0.0, 1.0);

        // Discipline slowly drifts down
        self.discipline = (self.discipline - 0.01 * multiplier * hours).clamp(0.0, 1.0);

        // Cleanliness decays
        self.cleanliness = (self.cleanliness - 0.03 * multiplier * hours).clamp(0.0, 1.0);

        // Poop accumulates over time (random-ish using deterministic formula)
        let new_poop = (hours * 0.3 * multiplier) as u32;
        if new_poop > 0 {
            self.poop_count = self.poop_count.saturating_add(new_poop);
        }

        // Health drops if hunger or happiness stay low
        if self.hunger < 0.2 || self.happiness < 0.2 {
            self.health = (self.health - 0.05 * multiplier * hours).clamp(0.0, 1.0);
        }

        // Sickness check
        let mut events = Vec::new();
        if !self.is_sick && (self.hunger < 0.15 || self.cleanliness < 0.2 || self.poop_count >= 5) {
            self.is_sick = true;
            self.health = (self.health - 0.1).max(0.0);
            events.push("Your pet got sick! Give it medicine.".to_string());
        }

        // Care mistake tracking
        if self.hunger < 0.1 {
            self.care_mistakes = self.care_mistakes.saturating_add(1);
        }

        // Death check
        if self.health <= 0.0 || self.hunger <= 0.0 {
            self.is_dead = true;
            self.is_sleeping = false;
            events.push(
                "Your pet has passed away from neglect. Start a new egg to try again.".to_string(),
            );
        }

        events
    }

    fn derive_mood(&self) -> PetMood {
        if self.is_dead {
            return PetMood::Dead;
        }
        if self.is_sleeping {
            return PetMood::Sleeping;
        }
        if self.is_sick {
            return PetMood::Sick;
        }
        if self.hunger < 0.3 {
            return PetMood::Hungry;
        }
        if self.happiness < 0.3 {
            return PetMood::Sad;
        }
        if self.happiness > 0.7 && self.hunger > 0.7 && !self.is_sick {
            return PetMood::Happy;
        }
        PetMood::Content
    }
}

// ─── Gamification State ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamificationState {
    pub pet: PetData,
    pub pet_stage: PetStage,
    pub xp: u64,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub last_checkin_date: Option<String>,
    pub checkin_count: u32,
    pub total_sessions: u32,
    pub total_turns: u64,
    pub total_tool_calls: u64,
    pub achievements: Vec<Achievement>,
    pub panel_usage: HashMap<String, bool>,
}

impl GamificationState {
    fn default_achievements() -> Vec<Achievement> {
        vec![
            Achievement {
                id: "genesis".into(),
                name: "Genesis".into(),
                description: "Complete your first session".into(),
                icon: "\u{1f31f}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "symbiosis".into(),
                name: "Symbiosis".into(),
                description: "Connect to the Savant Gateway".into(),
                icon: "\u{1f517}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "architect".into(),
                name: "Architect".into(),
                description: "Use all core panels".into(),
                icon: "\u{1f3db}\u{fe0f}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "sovereign".into(),
                name: "Sovereign".into(),
                description: "Reach max pet evolution stage".into(),
                icon: "\u{1f451}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "streak_7".into(),
                name: "Weekly Warrior".into(),
                description: "Maintain a 7-day streak".into(),
                icon: "\u{1f525}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "streak_30".into(),
                name: "Monthly Champion".into(),
                description: "Maintain a 30-day streak".into(),
                icon: "\u{1f4aa}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "checkin_100".into(),
                name: "Century".into(),
                description: "Check in 100 times".into(),
                icon: "\u{1f3af}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "turns_1000".into(),
                name: "Conversationalist".into(),
                description: "Complete 1000 turns".into(),
                icon: "\u{1f4ac}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "perfect_care".into(),
                name: "Perfect Care".into(),
                description: "Reach Sovereign with 0 care mistakes".into(),
                icon: "\u{1f4af}".into(),
                unlocked_at: None,
            },
            Achievement {
                id: "feeder".into(),
                name: "Devoted Feeder".into(),
                description: "Feed your pet 100 times".into(),
                icon: "\u{1f356}".into(),
                unlocked_at: None,
            },
        ]
    }

    pub fn new() -> Self {
        Self {
            pet: PetData::new(),
            pet_stage: PetStage::Egg,
            xp: 0,
            current_streak: 0,
            longest_streak: 0,
            last_checkin_date: None,
            checkin_count: 0,
            total_sessions: 0,
            total_turns: 0,
            total_tool_calls: 0,
            achievements: Self::default_achievements(),
            panel_usage: HashMap::new(),
        }
    }

    /// Called before any action to apply time decay and return events
    pub fn tick(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        self.pet.tick(now, &self.pet_stage)
    }

    pub fn add_xp(&mut self, amount: u64, now: &DateTime<Utc>) -> Vec<String> {
        self.xp = self.xp.saturating_add(amount);
        let mut events = Vec::new();

        // XP-based evolution (only if not dead)
        if !self.pet.is_dead && self.pet_stage != PetStage::Sovereign {
            let threshold = match self.pet_stage {
                PetStage::Egg => 10u64,
                PetStage::Seedling => 100,
                PetStage::Sprout => 500,
                PetStage::Bloom => 2000,
                PetStage::Sovereign => u64::MAX,
            };
            if self.xp >= threshold {
                let next = match self.pet_stage {
                    PetStage::Egg => PetStage::Seedling,
                    PetStage::Seedling => PetStage::Sprout,
                    PetStage::Sprout => PetStage::Bloom,
                    PetStage::Bloom => PetStage::Sovereign,
                    PetStage::Sovereign => PetStage::Sovereign,
                };
                self.pet_stage = next.clone();
                if next == PetStage::Sovereign {
                    self.unlock_achievement("sovereign", now, &mut events);
                    if self.pet.care_mistakes == 0 {
                        self.unlock_achievement("perfect_care", now, &mut events);
                    }
                }
                events.push(format!("Pet evolved to {}!", next.display_name()));
            }
        }
        events
    }

    // ── Core Care Actions ───────────────────────────────────────────────

    pub fn feed(&mut self, feed_type: FeedType, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            events.push("Your pet has passed away. You can start a new egg.".to_string());
            return events;
        }

        self.pet.times_fed = self.pet.times_fed.saturating_add(1);
        match feed_type {
            FeedType::Meal => {
                self.pet.hunger = (self.pet.hunger + 0.4).min(1.0);
                self.pet.weight = (self.pet.weight + 0.05).min(2.0);
            }
            FeedType::Snack => {
                self.pet.hunger = (self.pet.hunger + 0.2).min(1.0);
                self.pet.happiness = (self.pet.happiness + 0.1).min(1.0);
                self.pet.weight = (self.pet.weight + 0.08).min(2.0);
            }
        }

        // Overfeeding penalty
        if self.pet.weight > 1.5 {
            self.pet.health = (self.pet.health - 0.02).max(0.0);
        }

        events.push(format!(
            "Fed your pet a {}. Hunger: {:.0}%",
            feed_type.name(),
            self.pet.hunger * 100.0
        ));

        if self.pet.times_fed >= 100 {
            self.unlock_achievement("feeder", now, &mut events);
        }

        events
    }

    pub fn play(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        self.pet.times_played = self.pet.times_played.saturating_add(1);
        self.pet.happiness = (self.pet.happiness + 0.35).min(1.0);
        self.pet.discipline = (self.pet.discipline + 0.05).min(1.0);
        self.pet.weight = (self.pet.weight - 0.03).max(0.5);
        events.push(format!(
            "Played with your pet! Happiness: {:.0}%",
            self.pet.happiness * 100.0
        ));
        events
    }

    pub fn clean(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        self.pet.times_cleaned = self.pet.times_cleaned.saturating_add(1);
        self.pet.poop_count = 0;
        self.pet.cleanliness = 1.0;
        events.push("Cleaned up after your pet. Spotless!".to_string());
        events
    }

    pub fn give_medicine(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        if self.pet.is_sick {
            self.pet.is_sick = false;
            self.pet.health = (self.pet.health + 0.3).min(1.0);
            events.push("Gave medicine. Your pet is feeling better!".to_string());
        } else {
            // Medicine when not sick has no effect
            events.push("Your pet isn't sick. Medicine won't help.".to_string());
        }
        events
    }

    pub fn discipline(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        self.pet.discipline = (self.pet.discipline + 0.2).min(1.0);
        events.push(format!(
            "Disciplined your pet. Discipline: {:.0}%",
            self.pet.discipline * 100.0
        ));
        events
    }

    pub fn put_to_sleep(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        self.pet.is_sleeping = !self.pet.is_sleeping;
        if self.pet.is_sleeping {
            events.push("Your pet is now sleeping. Zzz...".to_string());
        } else {
            events.push("Your pet woke up!".to_string());
        }
        events
    }

    pub fn pet_interact(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        // Light interaction - small happiness boost
        self.pet.happiness = (self.pet.happiness + 0.05).min(1.0);
        events.push("You gently pet your companion.".to_string());
        events
    }

    // ── Session & Achievement Tracking ──────────────────────────────────

    pub fn record_session_end(
        &mut self,
        turns: u64,
        tool_calls: u64,
        now: &DateTime<Utc>,
    ) -> Vec<String> {
        let mut events = self.tick(now);
        if self.pet.is_dead {
            return events;
        }

        self.total_sessions = self.total_sessions.saturating_add(1);
        self.total_turns = self.total_turns.saturating_add(turns);
        self.total_tool_calls = self.total_tool_calls.saturating_add(tool_calls);

        let turn_xp = turns.saturating_mul(1);
        let tool_xp = tool_calls.saturating_mul(2);
        let session_xp = 5u64;
        let total_xp = turn_xp + tool_xp + session_xp;

        events.extend(self.add_xp(total_xp, now));
        self.check_achievements(now, &mut events);

        // Productive work also fills happiness and hunger slightly
        self.pet.happiness = (self.pet.happiness + 0.05).min(1.0);
        self.pet.hunger = (self.pet.hunger - 0.03).max(0.0); // working makes you hungry

        events
    }

    pub fn daily_checkin(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = self.tick(now);

        let today = now.format("%Y-%m-%d").to_string();

        match &self.last_checkin_date {
            Some(last) if last == &today => return events,
            Some(last) => {
                let last_date = chrono::NaiveDate::parse_from_str(last, "%Y-%m-%d").ok();
                let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").ok();
                match (last_date, today_date) {
                    (Some(l), Some(t)) if (t - l).num_days() == 1 => {
                        self.current_streak = self.current_streak.saturating_add(1);
                    }
                    _ => {
                        self.current_streak = 1;
                    }
                }
            }
            None => {
                self.current_streak = 1;
            }
        }

        self.last_checkin_date = Some(today);
        self.checkin_count = self.checkin_count.saturating_add(1);

        if self.current_streak > self.longest_streak {
            self.longest_streak = self.current_streak;
        }

        let streak_bonus = std::cmp::min(self.current_streak.saturating_mul(5), 50) as u64;
        let checkin_xp = 10u64 + streak_bonus;
        events.extend(self.add_xp(checkin_xp, now));
        self.check_achievements(now, &mut events);

        // Check-in also feeds the pet a little
        if !self.pet.is_dead {
            self.pet.hunger = (self.pet.hunger + 0.1).min(1.0);
            self.pet.happiness = (self.pet.happiness + 0.1).min(1.0);
        }

        events
    }

    pub fn record_panel_usage(&mut self, panel: &str, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = Vec::new();
        self.panel_usage.insert(panel.to_string(), true);
        if self.panel_usage.len() >= 4 {
            self.unlock_achievement("architect", now, &mut events);
        }
        events
    }

    pub fn record_gateway_connected(&mut self, now: &DateTime<Utc>) -> Vec<String> {
        let mut events = Vec::new();
        self.unlock_achievement("symbiosis", now, &mut events);
        events
    }

    pub fn set_evolution_stage(&mut self, stage: &str) {
        self.pet_stage = PetStage::from_evolution_stage(stage);
    }

    pub fn reset_egg(&mut self, _now: &DateTime<Utc>) -> Vec<String> {
        self.pet = PetData::new();
        self.pet_stage = PetStage::Egg;
        self.xp = 0;
        vec!["A new egg has appeared! Take good care of it.".to_string()]
    }

    fn check_achievements(&mut self, now: &DateTime<Utc>, events: &mut Vec<String>) {
        if self.total_sessions >= 1 {
            self.unlock_achievement("genesis", now, events);
        }
        if self.current_streak >= 7 {
            self.unlock_achievement("streak_7", now, events);
        }
        if self.current_streak >= 30 {
            self.unlock_achievement("streak_30", now, events);
        }
        if self.checkin_count >= 100 {
            self.unlock_achievement("checkin_100", now, events);
        }
        if self.total_turns >= 1000 {
            self.unlock_achievement("turns_1000", now, events);
        }
    }

    fn unlock_achievement(&mut self, id: &str, now: &DateTime<Utc>, events: &mut Vec<String>) {
        for a in &mut self.achievements {
            if a.id == id && a.unlocked_at.is_none() {
                a.unlocked_at = Some(now.timestamp());
                events.push(format!("Achievement unlocked: {} {}", a.icon, a.name));
            }
        }
    }
}

// ─── FeedType helpers ───────────────────────────────────────────────────────

impl FeedType {
    fn name(&self) -> &str {
        match self {
            FeedType::Meal => "meal",
            FeedType::Snack => "snack",
        }
    }
}

// ─── PetState summary for the frontend ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetSummary {
    pub stage: String,
    pub stage_icon: String,
    pub mood: String,
    pub mood_icon: String,
    pub hunger: f64,
    pub happiness: f64,
    pub discipline: f64,
    pub health: f64,
    pub weight: f64,
    pub cleanliness: f64,
    pub poop_count: u32,
    pub is_sick: bool,
    pub is_sleeping: bool,
    pub is_dead: bool,
    pub age_hours: f64,
    pub care_mistakes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamificationSummary {
    pub pet: PetSummary,
    pub xp: u64,
    pub current_streak: u32,
    pub longest_streak: u32,
    pub checkin_count: u32,
    pub total_sessions: u32,
    pub total_turns: u64,
    pub total_tool_calls: u64,
    pub unlocked_achievements: usize,
    pub total_achievements: usize,
    pub achievements: Vec<Achievement>,
}

// ─── Service ────────────────────────────────────────────────────────────────

pub struct GamificationService {
    state: GamificationState,
    state_path: PathBuf,
}

impl GamificationService {
    pub fn new() -> Self {
        let save_dir = dirs::home_dir()
            .map(|p| p.join(".savant"))
            .unwrap_or_else(|| PathBuf::from(".savant"));

        let state_path = save_dir.join("gamification.json");
        let mut state: GamificationState = std::fs::read_to_string(&state_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(GamificationState::new);

        // Apply decay on load (time passed while app was closed)
        let now = Utc::now();
        state.tick(&now);

        let service = Self { state, state_path };
        service.try_save();
        service
    }

    fn try_save(&self) {
        if let Some(parent) = self.state_path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.state) {
            let tmp = self.state_path.with_extension("json.tmp");
            if std::fs::write(&tmp, &json).is_ok() {
                if let Err(e) = std::fs::rename(&tmp, &self.state_path) {
                    tracing::warn!("Failed to rename gamification state file: {}", e);
                }
            }
        }
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create savant dir: {}", e))?;
        }
        let json = serde_json::to_string_pretty(&self.state)
            .map_err(|e| format!("Serialization error: {}", e))?;
        let tmp = self.state_path.with_extension("json.tmp");
        std::fs::write(&tmp, &json).map_err(|e| format!("Write error: {}", e))?;
        std::fs::rename(&tmp, &self.state_path).map_err(|e| format!("Rename error: {}", e))?;
        Ok(())
    }

    fn with_tick<R>(
        &mut self,
        f: impl FnOnce(&mut GamificationState, &DateTime<Utc>) -> R,
    ) -> Result<R, String> {
        let now = Utc::now();
        // Tick is applied inside the state methods, but we also force last_update tracking
        let result = f(&mut self.state, &now);
        self.save()?;
        Ok(result)
    }

    pub fn get_state_summary(&self) -> GamificationSummary {
        let unlocked_count = self
            .state
            .achievements
            .iter()
            .filter(|a| a.unlocked_at.is_some())
            .count();
        let total_count = self.state.achievements.len();
        let p = &self.state.pet;
        let mood = p.derive_mood();

        GamificationSummary {
            pet: PetSummary {
                stage: self.state.pet_stage.display_name().to_string(),
                stage_icon: self.state.pet_stage.display_icon().to_string(),
                mood: format!("{:?}", mood),
                mood_icon: mood.display_icon().to_string(),
                hunger: p.hunger,
                happiness: p.happiness,
                discipline: p.discipline,
                health: p.health,
                weight: p.weight,
                cleanliness: p.cleanliness,
                poop_count: p.poop_count,
                is_sick: p.is_sick,
                is_sleeping: p.is_sleeping,
                is_dead: p.is_dead,
                age_hours: p.age_hours,
                care_mistakes: p.care_mistakes,
            },
            xp: self.state.xp,
            current_streak: self.state.current_streak,
            longest_streak: self.state.longest_streak,
            checkin_count: self.state.checkin_count,
            total_sessions: self.state.total_sessions,
            total_turns: self.state.total_turns,
            total_tool_calls: self.state.total_tool_calls,
            unlocked_achievements: unlocked_count,
            total_achievements: total_count,
            achievements: self.state.achievements.clone(),
        }
    }

    // ── Public action methods ────────────────────────────────────────────────

    pub fn feed(&mut self, feed_type: FeedType) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.feed(feed_type.clone(), now))
    }

    pub fn play(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.play(now))
    }

    pub fn clean(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.clean(now))
    }

    pub fn give_medicine(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.give_medicine(now))
    }

    pub fn discipline(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.discipline(now))
    }

    pub fn put_to_sleep(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.put_to_sleep(now))
    }

    pub fn pet_interact(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.pet_interact(now))
    }

    pub fn daily_checkin(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.daily_checkin(now))
    }

    pub fn record_session_end(
        &mut self,
        turns: u64,
        tool_calls: u64,
    ) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.record_session_end(turns, tool_calls, now))
    }

    pub fn record_panel_usage(&mut self, panel: &str) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.record_panel_usage(panel, now))
    }

    pub fn record_gateway_connected(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.record_gateway_connected(now))
    }

    pub fn set_evolution_stage(&mut self, stage: &str) -> Result<(), String> {
        self.with_tick(|s, _| {
            s.set_evolution_stage(stage);
            Vec::<String>::new()
        })?;
        Ok(())
    }

    pub fn reset_egg(&mut self) -> Result<Vec<String>, String> {
        self.with_tick(|s, now| s.reset_egg(now))
    }
}
