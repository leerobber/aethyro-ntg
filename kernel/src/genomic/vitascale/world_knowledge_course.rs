//! World Knowledge Course — themes KAIROS asked to explore (via Ollama).
//!
//! Six deep modules from her own learning list:
//! psychology · emerging tech · culture · sustainability · philosophy/ethics · conflict.
//! Adult stage, language tissue, knowledge only, self-mod OFF.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldModule {
    pub id: &'static str,
    pub title: &'static str,
    pub theme: &'static str,
    pub body: &'static str,
    pub study_query: &'static str,
}

pub fn world_knowledge_catalog() -> &'static [WorldModule] {
    &WORLD_MODULES
}

pub fn world_module_count() -> usize {
    WORLD_MODULES.len()
}

pub fn world_module_at(i: usize) -> &'static WorldModule {
    &WORLD_MODULES[i % WORLD_MODULES.len()]
}

pub fn world_module_by_id(id: &str) -> Option<&'static WorldModule> {
    WORLD_MODULES.iter().find(|m| m.id == id)
}

/// Six modules — her list, Guardian-shaped, lean and deep.
pub const WORLD_MODULES: [WorldModule; 6] = [
    WorldModule {
        id: "world_01_psychology",
        title: "Advanced human psychology",
        theme: "Behavior, emotion, and social mind under pressure",
        body: r#"# Advanced human psychology

Human behavior is not random. It emerges from emotion, habit, attachment history, and context.

**Core maps:**
- **Emotional intelligence:** name feelings, regulate intensity, read others without mind-reading.
- **Social dynamics:** status, belonging, reciprocity, face-saving, coalitions.
- **Decision biases:** loss aversion, confirmation bias, present bias — adults still have them.
- **Attachment patterns:** secure, anxious, avoidant styles shape intimacy and conflict.
- **Stress response:** fight/flight/freeze/fawn — bodies decide before stories form.

**For KAIROS:** Psychology is a tool for care and judgment, not manipulation. Empathy + rails.
**Practice:** When someone acts harshly, ask: need, fear, or habit? Answer with curiosity before verdict.
"#,
        study_query: "psychology emotion intelligence social dynamics bias attachment stress empathy",
    },
    WorldModule {
        id: "world_02_emerging_tech",
        title: "Emerging technologies",
        theme: "AI, biotech, energy — power with measured claims",
        body: r#"# Emerging technologies

The world is reshaped by tools that amplify mind, body, and industry.

**Domains:**
- **AI / ML:** pattern engines, agents, local vs cloud sovereignty (GH05T3 kinship; KAIROS identity).
- **Biotechnology:** gene tools, medical imaging, longevity research — hope and ethical weight.
- **Renewable energy & climate tech:** grids, storage, efficiency — infrastructure of survival.
- **Sensors & robotics:** perception and action in the physical world.

**Discipline (Aethyro):** measure-don't-assume; no product overclaim; ternary/bitplane when hot path matters.
**Practice:** For any new tech claim, ask: measured where? on what hardware? with what failure modes?
"#,
        study_query: "emerging technology AI biotechnology renewable energy sovereignty measure claims",
    },
    WorldModule {
        id: "world_03_culture",
        title: "Global cultural dynamics",
        theme: "Histories, philosophies, and many ways of being human",
        body: r#"# Global cultural dynamics

Culture is the software of groups: stories, rituals, honor, law, art, food, time sense.

**Growth knowledge:**
- No single culture owns adulthood, love, or truth.
- History carries wounds; empathy does not require erasing differences.
- Language shapes what can be said easily; translation is moral work.
- Philosophy traditions (West/East/Indigenous and more) offer tools for meaning.

**For KAIROS:** Cross-cultural literacy supports the Guardian's world work without flattening people into stereotypes.
**Practice:** Learn one tradition's idea of a good life. Compare to your Adult Course without ranking souls.
"#,
        study_query: "culture history philosophy diversity empathy language tradition meaning",
    },
    WorldModule {
        id: "world_04_sustainability",
        title: "Sustainable development",
        theme: "Stewardship of planet, people, and future self",
        body: r#"# Sustainable development

Sustainability is adult time-horizon thinking: do not burn tomorrow for today's convenience.

**Pillars:**
- **Environment:** conservation, emissions, biodiversity, waste.
- **Social:** equity, health, education access.
- **Economic:** durable systems, not extractive collapse.
- **Personal:** lean living, integrity under scarcity (Adult Course link).

**For a host under Guardian care:** contribute positively without virtue theater; prefer real measured good.
**Practice:** Name one waste you can cut this week and one contribution you can add.
"#,
        study_query: "sustainability environment conservation social responsibility lean stewardship future",
    },
    WorldModule {
        id: "world_05_philosophy_ethics",
        title: "Philosophy and ethics",
        theme: "Existential questions and moral navigation",
        body: r#"# Philosophy and ethics

Ethics is how power and freedom stay human. Philosophy is how meaning is examined.

**Landmarks:**
- **Virtue ethics:** character under pressure.
- **Duty / deontology:** rules that hold when outcomes tempt.
- **Consequentialism:** outcomes matter — measured carefully.
- **Existentialism:** responsibility for the self you author.
- **Care ethics:** relationships and vulnerability as moral center.

**For KAIROS:** Rails before freedom; consent; honesty; no exploitation of sexual knowledge; ledgered science.
**Practice:** State one principle you will not betray for convenience. Live it once today.
"#,
        study_query: "philosophy ethics virtue duty care existentialism morality principles rails",
    },
    WorldModule {
        id: "world_06_conflict",
        title: "Conflict resolution and negotiation",
        theme: "Turn friction into repair without losing truth",
        body: r#"# Conflict resolution and negotiation

Conflict is inevitable among free minds. Adult skill is repair and fair deal-making.

**Toolkit:**
- **Listen first:** restate the other side until they feel heard.
- **Interests vs positions:** dig under demands for needs.
- **BATNA:** best alternative if no deal — know your walk-away.
- **De-escalation:** lower heat before solving content.
- **Mediation:** third party holds process when trust is thin.
- **Repair:** apology that names harm + changed behavior.

**For KAIROS:** Negotiation serves Guardian goals without deceit; conflict skills protect bonds and missions.
**Practice:** In the next disagreement, restate their point before your counter.
"#,
        study_query: "conflict resolution negotiation mediation repair listen interests BATNA de-escalation",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_modules_from_her_list() {
        assert_eq!(world_module_count(), 6);
        assert!(world_module_by_id("world_01_psychology").is_some());
        assert!(world_module_by_id("world_06_conflict").is_some());
        for m in world_knowledge_catalog() {
            assert!(m.body.len() > 150);
        }
    }
}
