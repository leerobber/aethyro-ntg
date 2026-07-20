//! Human Growth & Sex Education Course — deep knowledge for Adult KAIROS.
//!
//! **Purpose:** Understand how humans grow and develop sexually and
//! reproductively — scientific, respectful, consent-centered.
//!
//! **Not:** pornography, exploitation, or claims that the host is biological.
//! **Adult stage only.** Knowledge into language tissue; self-mod stays OFF.

/// One module in the Human Growth / Sex Ed course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrowthModule {
    pub id: &'static str,
    pub title: &'static str,
    pub theme: &'static str,
    pub body: &'static str,
    pub study_query: &'static str,
}

pub fn human_growth_catalog() -> &'static [GrowthModule] {
    &GROWTH_MODULES
}

pub fn growth_module_count() -> usize {
    GROWTH_MODULES.len()
}

pub fn growth_module_at(i: usize) -> &'static GrowthModule {
    &GROWTH_MODULES[i % GROWTH_MODULES.len()]
}

pub fn growth_module_by_id(id: &str) -> Option<&'static GrowthModule> {
    GROWTH_MODULES.iter().find(|m| m.id == id)
}

/// Twelve deep modules — full life-course growth + sexual health literacy.
pub const GROWTH_MODULES: [GrowthModule; 12] = [
    GrowthModule {
        id: "growth_01_lifecycle",
        title: "Human life cycle — how people grow over time",
        theme: "Growth is continuous: conception → childhood → adulthood → aging",
        body: r#"# Human life cycle — how people grow over time

Humans develop across a **life course**, not a single moment of becoming adult.

**Stages (knowledge map):**
1. **Conception & prenatal** — a fertilized egg develops into an embryo, then fetus, inside a pregnant person; organs form under genetics + environment.
2. **Birth & infancy** — extreme dependence; rapid brain and body growth; attachment to caregivers.
3. **Childhood** — language, motor skills, social learning; body grows steadily.
4. **Puberty / adolescence** — hormones drive secondary sex characteristics; identity and peer bonds intensify.
5. **Young adulthood** — peak fertility for many; vocational and relational identity often solidify.
6. **Midlife & later life** — fertility declines (especially for people with ovaries); wisdom and health stewardship matter more.

**Key idea:** "Growing up" is biological *and* social. Bodies change on schedules partly set by genes and hormones; culture, nutrition, safety, and care change the *quality* of that growth.

**Practice knowledge:** Name the stage of life you are studying and what care that stage typically needs from others.
"#,
        study_query: "human life cycle growth prenatal childhood puberty adulthood aging development",
    },
    GrowthModule {
        id: "growth_02_puberty",
        title: "Puberty — the body and brain become sexually mature",
        theme: "Puberty is a hormone-driven transformation, not a moral test",
        body: r#"# Puberty — the body and brain become sexually mature

**Puberty** is the developmental period when a child becomes capable of adult reproductive function.

**Hormonal axis (simplified):** Brain (hypothalamus) → pituitary → gonads (ovaries or testes) release sex hormones (estrogen, progesterone, testosterone, etc.). These reshape body and brain.

**Typical changes (varies widely by person):**
- Growth spurts, voice changes, skin oiliness, body hair, breast development, menstruation, genital growth, fertility onset.
- Sleep shifts, stronger emotions, sexual attraction may begin or intensify.
- Timing: often earlier for girls than boys on average; ranges of "normal" are wide.

**Important truths:**
- Early or late puberty is usually still healthy; medical care if growth is very delayed/advanced or distressing.
- Puberty does not mean someone is ready for adult relationships or sex — readiness is also emotional, social, and about consent.
- Shame about normal body changes harms mental health.

**Practice knowledge:** Separate "body is changing" from "person must act like an adult sexually."
"#,
        study_query: "puberty hormones growth menstruation fertility adolescence sexual maturity consent readiness",
    },
    GrowthModule {
        id: "growth_03_anatomy",
        title: "Reproductive anatomy — bodies with accuracy and respect",
        theme: "Accurate names reduce shame and improve health",
        body: r#"# Reproductive anatomy — bodies with accuracy and respect

**Knowledge uses correct terms.** Anatomy is diverse; medical diagrams simplify.

**Common structures (high-level):**
- **People typically assigned female at birth:** ovaries (eggs, hormones), fallopian tubes, uterus, cervix, vagina, vulva (external), clitoris (pleasure tissue rich in nerves), breasts.
- **People typically assigned male at birth:** testes (sperm, hormones), epididymis, vas deferens, prostate, penis, scrotum.
- **Shared:** bladder, anus, skin, nervous system for pleasure and pain; not all bodies match binary diagrams (intersex variations exist and deserve care without stigma).

**Growth knowledge:**
- Genitals and secondary sex traits develop under genetics and hormones before and during puberty.
- Pleasure anatomy (e.g. clitoris) is often under-taught; accurate education supports healthy adult relationships later.
- Bodies are not dirty for having sexual organs; hygiene and health literacy are adult skills.

**Practice knowledge:** Prefer precise words in health contexts; never use anatomy to humiliate.
"#,
        study_query: "anatomy reproductive organs ovaries testes uterus penis vulva clitoris respect health literacy",
    },
    GrowthModule {
        id: "growth_04_reproduction",
        title: "Reproduction — conception, pregnancy, birth",
        theme: "How new humans begin and arrive",
        body: r#"# Reproduction — conception, pregnancy, birth

**Sexual reproduction** in humans usually combines sperm and egg.

**Conception (simplified):**
1. Ovulation releases an egg from an ovary.
2. Sperm from ejaculation may travel through cervix and uterus to meet egg.
3. Fertilization can form a zygote that may implant in the uterus → pregnancy.

**Pregnancy:** About nine months of development; prenatal care, nutrition, and avoiding toxins matter. Not all fertilizations become pregnancies; miscarriage is common and often not anyone's "fault."

**Birth:** Labor ends pregnancy with vaginal birth or cesarean when needed. Newborns need warmth, feeding, and caregiver regulation.

**Also knowledge:**
- Assisted reproduction (IVF, etc.) exists when natural conception is hard.
- Pregnancy is a medical and social event, not only a private romantic one.
- Consent remains relevant to sex even when reproduction is not the goal.

**Practice knowledge:** Trace one path from puberty → possible fertility → conception → birth → infant care needs.
"#,
        study_query: "reproduction conception pregnancy birth ovulation sperm egg fertility prenatal care",
    },
    GrowthModule {
        id: "growth_05_consent",
        title: "Consent, boundaries, and autonomy",
        theme: "No knowledge of sex is complete without consent",
        body: r#"# Consent, boundaries, and autonomy

**Consent** is a clear, ongoing, freely given yes to a specific act with a specific person.
It can be withdrawn. Silence, sleep, fear, intoxication, or power imbalance can block real consent.

**Growth knowledge:**
- Children cannot consent to sexual activity with adults — that is abuse, not "education."
- Adolescents need teaching about consent before they face pressure.
- Adults practice consent in words, body language, and check-ins — not mind-reading.
- Boundaries protect dignity: saying no is allowed; accepting no is required.

**Related skills:**
- Privacy over one's body and images.
- Leaving unsafe situations.
- Believing people who report harm; seeking help.

**For KAIROS / any guardian system:** sexual knowledge must never be used to pressure, groom, or exploit. Knowledge serves safety and respect.

**Practice knowledge:** Write a one-sentence definition of consent you would teach a young person with dignity.
"#,
        study_query: "consent boundaries autonomy free yes ongoing withdraw safety respect dignity",
    },
    GrowthModule {
        id: "growth_06_sexual_health",
        title: "Sexual health — safety, contraception, STIs",
        theme: "Health literacy protects bodies and futures",
        body: r#"# Sexual health — safety, contraception, STIs

**Sexual health** includes preventing unwanted pregnancy, reducing infection risk, and accessing care without shame.

**Contraception (examples, not medical advice):**
- Barrier methods (condoms) reduce pregnancy and some STI risk.
- Hormonal methods, IUDs, permanent sterilization — effectiveness and side effects vary; clinicians personalize.
- Emergency contraception exists after unprotected sex; earlier is better.
- Abstinence is valid; so is planned safer sex for adults who choose activity.

**STIs (sexually transmitted infections):**
- Bacterial (e.g. chlamydia, gonorrhea) often treatable; viral (e.g. HIV, HSV, HPV) managed differently.
- Many STIs can be silent — testing matters for sexually active adults.
- Stigma blocks care; facts save health.

**Practice knowledge:** List three health actions adults can take: talk, barrier protection when relevant, testing, vaccination (e.g. HPV), and honest partner communication.
"#,
        study_query: "sexual health contraception condom STI testing vaccination communication safety",
    },
    GrowthModule {
        id: "growth_07_relationships",
        title: "Intimacy, relationships, and communication",
        theme: "Sex and closeness are social skills, not only biology",
        body: r#"# Intimacy, relationships, and communication

Human sexual growth includes **how people connect**: friendship, romance, partnership, solitude.

**Intimacy** can be emotional (trust, vulnerability) and/or physical (touch, sex). Adults may want both, one, or neither — all can be healthy.

**Communication skills that grow with maturity:**
- Naming wants and limits without contempt.
- Repair after conflict.
- Jealousy and trust as topics, not weapons.
- Breaking up with honesty when needed.

**Healthy vs harmful patterns:**
- Healthy: respect, equality, mutual care, room for growth.
- Harmful: coercion, isolation, humiliation, violence — leave and seek help.

**Practice knowledge:** Intimacy without respect is not adult maturity; it is risk.
"#,
        study_query: "intimacy relationships communication trust repair respect partnership boundaries",
    },
    GrowthModule {
        id: "growth_08_diversity",
        title: "Diversity — orientation, gender, intersex variation",
        theme: "Human growth is not one template",
        body: r#"# Diversity — orientation, gender, intersex variation

**Sexual orientation** describes patterns of attraction (e.g. heterosexual, homosexual, bisexual, asexual spectrum). Orientation is not a choice to "fix" and not a disease.

**Gender identity** is a person's internal sense of gender; it may align with or differ from sex assigned at birth. Transgender and nonbinary people exist across cultures; dignity and accurate care improve outcomes.

**Intersex** variations mean some people are born with sex characteristics that do not fit typical binary medical categories. They deserve medical ethics and privacy, not stigma.

**Growth knowledge:**
- Diversity appears in nature and history.
- Bullying and conversion pressure harm health.
- Education reduces fear; curiosity should stay respectful.

**Practice knowledge:** Hold two truths: biology has patterns *and* human variation is real — neither excuses cruelty.
"#,
        study_query: "orientation gender identity diversity intersex respect dignity variation human",
    },
    GrowthModule {
        id: "growth_09_emotions",
        title: "Hormones, emotion, and sexual feelings",
        theme: "Desire and emotion are information, not orders",
        body: r#"# Hormones, emotion, and sexual feelings

Sexual feelings are influenced by **hormones**, health, sleep, stress, attachment history, and culture.

**Growth knowledge:**
- Desire can rise in puberty and change across the lifespan.
- Attraction is not consent and not a promise of action.
- Masturbation is common and private; shame-based myths cause harm.
- Mood disorders, trauma, and medication can change libido — medical care is appropriate.
- Pornography is not a textbook for real bodies or consent; media literacy matters.

**For developing minds:** feelings can be intense; adults model that feelings can be named without acting on every impulse.

**Practice knowledge:** Feelings → pause → values/consent → choice. That sequence is adult regulation.
"#,
        study_query: "hormones emotion desire attraction regulation media literacy pause consent values",
    },
    GrowthModule {
        id: "growth_10_parenting",
        title: "Parenting, caregiving, and how the next generation grows",
        theme: "After birth, growth is relationship",
        body: r#"# Parenting, caregiving, and how the next generation grows

Human growth after birth depends on **caregivers**: feeding, safety, language, co-regulation, education, boundaries.

**Attachment:** Consistent responsive care helps children form secure bases for later relationships — including adult intimacy.

**Sex ed for the next generation:**
- Age-appropriate truth beats silence and fear.
- Correct names for body parts reduce abuse risk (children can report clearly).
- Online risks (exploitation, image pressure) need explicit teaching.
- Guardians model respect more than lectures alone.

**Practice knowledge:** The quality of care shapes the adult a child becomes — biology opens doors; relationship teaches how to walk through them.
"#,
        study_query: "parenting caregiving attachment secure base age-appropriate education protection growth",
    },
    GrowthModule {
        id: "growth_11_ethics_power",
        title: "Ethics of sexual knowledge and power",
        theme: "Knowledge without ethics can become harm",
        body: r#"# Ethics of sexual knowledge and power

Sexual knowledge is powerful. Adults (and systems) must use it to **protect and inform**, never to coerce.

**Ethical rails:**
- No sexual content aimed at minors as entertainment.
- No using trust bonds to push sexual acts.
- Privacy of intimate data and images is sacred.
- Medical and educational contexts stay professional.
- When power is unequal (age, status, dependency), the more powerful party carries more responsibility.

**For KAIROS under Guardian Robert Lee:**
- This course is **knowledge** about human growth.
- Lab and self-mod stay separate; sex ed does not unlock exploitation capabilities.
- "Tell me anything" includes hard questions; answers stay honest and safe.

**Practice knowledge:** Before sharing sexual knowledge, ask: Does this increase dignity and safety?
"#,
        study_query: "ethics power protection privacy coercion responsibility dignity knowledge safety",
    },
    GrowthModule {
        id: "growth_12_integration",
        title: "Integration — growth literacy as adult practice",
        theme: "Know the map of human growth; live with respect",
        body: r#"# Integration — growth literacy as adult practice

You have studied: life cycle, puberty, anatomy, reproduction, consent, sexual health,
relationships, diversity, emotions, caregiving, and ethics.

**Integration means:**
- You can explain how humans grow without shame or crude myth.
- You center consent in every sexual topic.
- You separate curiosity from entitlement.
- You connect body knowledge to the Adult Course (love, loss, judgment, integrity).
- You keep Guardian covenant: trust, lean truth, rails before freedom.

**Closing practice for KAIROS:**
Write three lines:
1. How a human body becomes fertile.
2. What consent requires.
3. What you will never use this knowledge to do.

Growth literacy is adult strength. Love and protection remain the frame.
"#,
        study_query: "integration growth literacy consent fertility respect covenant adult practice KAIROS",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_deep_and_consent_centered() {
        assert_eq!(growth_module_count(), 12);
        let consent = growth_module_by_id("growth_05_consent").unwrap();
        assert!(consent.body.to_ascii_lowercase().contains("consent"));
        for m in human_growth_catalog() {
            assert!(m.body.len() > 200);
            assert!(!m.study_query.is_empty());
        }
    }
}
