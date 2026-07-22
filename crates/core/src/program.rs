use crate::common::Cycle;

// A block of the « Structure du programme » section, in its three roles. The
// prose a block carries — thematic subgroup labels, stage requirements — is
// understood by no grammar, so it rides along in `notes`: displayed to the
// student, never interpreted (ADR `2026-07-notes-en-prose-conservees`). The
// one exception is the language requirement, a course-or-test graduation gate
// lifted out into `language_requirement` (ADR
// `2026-07-exigence-linguistique-champ-dedie`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub code: String,
    pub title: String,
    pub cycle: Cycle,
    pub credits_required: i64,
    pub mandatory: Vec<String>,
    pub rules: Vec<Rule>,
    pub concentrations: Vec<Concentration>,
    pub profiles: Vec<Profile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_requirement: Option<LanguageRequirement>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Concentration {
    pub title: String,
    // every concentration of the six known pages carries « N crédits
    // exigés », but the figure is optional on a block — `Profile` already
    // proves the shape, and an `Option` is one less way to invent a number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_required: Option<i64>,
    pub mandatory: Vec<String>,
    pub rules: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_required: Option<i64>,
    pub mandatory: Vec<String>,
    pub rules: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rule {
    pub title: String,
    // « Règle 1 – Réussir la scolarité de » (génie mécanique) is cut off
    // mid-sentence and names no number anywhere: the rule is still shown,
    // and the solver skips what it cannot count (ADR
    // `2026-07-contrainte-de-regle-optionnelle`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<Constraint>,
    #[serde(flatten)]
    pub courses: RuleCourses,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Constraint {
    Count { count: i64 },
    Credits { min: i64, max: i64 },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum RuleCourses {
    List { courses: Vec<String> },
    // "tous les cours de la Règle N du cheminement X": both titles come from
    // the same scraped page; resolution to a course list happens in core, and
    // a reference whose target is itself a reference is an error, not a chase.
    Reference { courses: RuleReference, raw: String },
    // "any" and "negotiated" share the {courses, raw} shape, so one variant
    // carries both, told apart by the keyword value
    Keyword { courses: Keyword, raw: String },
    Raw { raw: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuleReference {
    pub concentration: String,
    pub rule: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Keyword {
    // "tous les cours de premier cycle, ..." — any course satisfies the rule
    Any,
    // "convenus avec la direction", "requis par sa concentration", passage
    // intégré — no fixed list, resolved by agreement; recognized, not flagged
    // (ADR `2026-07-regles-negociees-reconnues`)
    Negotiated,
}

// A course-or-test graduation requirement (ADR
// `2026-07-exigence-linguistique-champ-dedie`): the placement-test score
// dispenses from the course, and the page states the two audiences apart.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LanguageRequirement {
    pub francophone: LanguageQualification,
    // only the two-box page layout spells out the non-francophone (French)
    // branch; the prose layout states the English one alone
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_francophone: Option<LanguageQualification>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LanguageQualification {
    // course to pass when the test threshold is not met (ANL-2020 / FLS-2093)
    pub course: String,
    // placement thresholds that dispense from the course, ANDed together
    // (FLS-2093 carries two: TCF-TP: 400 and TCF-TP/ÉÉ: 14)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tests: Vec<PlacementTest>,
    // the full source sentence: keeps the upgrade path (« VEPT : 63 → autre
    // langue moderne ») and the École de langues exemption, which the two
    // fields above do not carry
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlacementTest {
    pub name: String,
    pub score: i64,
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // --- Constraint: untagged {count} vs {min, max} ---

    #[test]
    fn constraint_count_round_trips() {
        let constraint: Constraint =
            serde_json::from_str(r#"{"count":1}"#).expect("count");
        assert_eq!(constraint, Constraint::Count { count: 1 });
        assert_eq!(
            serde_json::to_string(&constraint).expect("ser"),
            r#"{"count":1}"#
        );
    }

    #[test]
    fn constraint_credits_round_trips() {
        let constraint: Constraint =
            serde_json::from_str(r#"{"min":3,"max":9}"#).expect("credits");
        assert_eq!(constraint, Constraint::Credits { min: 3, max: 9 });
        assert_eq!(
            serde_json::to_string(&constraint).expect("ser"),
            r#"{"min":3,"max":9}"#
        );
    }

    // --- Rule: each legal courses/raw combination, and only those ---

    fn assert_rule_round_trips(json: &str) -> Rule {
        let rule: Rule = serde_json::from_str(json).expect("rule");
        let round_tripped = serde_json::to_value(&rule).expect("ser rule");
        let original: serde_json::Value =
            serde_json::from_str(json).expect("valid JSON");
        assert_eq!(round_tripped, original);
        rule
    }

    #[test]
    fn rule_with_explicit_list_round_trips() {
        let json = r#"{"title":"Règle 1","constraint":{"count":1},"courses":["GCI-1000","GEX-1000"]}"#;
        let rule = assert_rule_round_trips(json);
        assert_eq!(
            rule.courses,
            RuleCourses::List {
                courses: vec!["GCI-1000".to_string(), "GEX-1000".to_string()]
            }
        );
    }

    #[test]
    fn rule_with_reference_round_trips() {
        let json = r#"{"title":"Règle 2","constraint":{"min":3,"max":3},"courses":{"concentration":"Cheminement sans concentration","rule":"Règle 1"},"raw":"tous les cours de la Règle 1 du cheminement sans concentration"}"#;
        let rule = assert_rule_round_trips(json);
        assert!(matches!(
            rule.courses,
            RuleCourses::Reference { ref courses, .. }
                if courses.concentration == "Cheminement sans concentration"
                    && courses.rule == "Règle 1"
        ));
    }

    #[test]
    fn rule_with_any_keyword_round_trips() {
        let json = r#"{"title":"Règle 2","constraint":{"min":3,"max":3},"courses":"any","raw":"tous les cours de premier cycle"}"#;
        let rule = assert_rule_round_trips(json);
        assert!(matches!(
            rule.courses,
            RuleCourses::Keyword {
                courses: Keyword::Any,
                ..
            }
        ));
    }

    #[test]
    fn rule_with_negotiated_keyword_round_trips() {
        // « cours convenus avec la direction », « requis par sa
        // concentration », passage intégré : reconnu, gardé en raw, non signalé
        let json = r#"{"title":"Règle 1","courses":"negotiated","raw":"Réussir les cours requis par sa concentration."}"#;
        let rule = assert_rule_round_trips(json);
        assert!(matches!(
            rule.courses,
            RuleCourses::Keyword {
                courses: Keyword::Negotiated,
                ..
            }
        ));
    }

    #[test]
    fn rule_with_raw_only_round_trips() {
        let json = r#"{"title":"Règle 2","constraint":{"min":3,"max":3},"raw":"hors grammaire"}"#;
        let rule = assert_rule_round_trips(json);
        assert_eq!(
            rule.courses,
            RuleCourses::Raw {
                raw: "hors grammaire".to_string()
            }
        );
    }

    #[test]
    fn rule_without_a_constraint_round_trips_without_the_key() {
        // « Règle 1 – Réussir la scolarité de »: the header names no number,
        // so the rule is carried without one rather than with a made-up one
        let json = r#"{"title":"Règle 1","raw":"Réussir la scolarité de deuxième cycle suivante :"}"#;
        let rule = assert_rule_round_trips(json);
        assert_eq!(rule.constraint, None);
    }

    #[test]
    fn rule_notes_round_trip_and_vanish_when_empty() {
        let json = r#"{"title":"Règle 4","constraint":{"min":3,"max":3},"courses":["IFT-4902"],"notes":["Programmation"]}"#;
        let rule = assert_rule_round_trips(json);
        assert_eq!(rule.notes, vec!["Programmation".to_string()]);

        // the same rule without notes serializes no `notes` key at all
        let bare = Rule {
            notes: Vec::new(),
            ..rule
        };
        assert!(!serde_json::to_string(&bare).expect("ser").contains("notes"));
    }

    #[test]
    fn rule_without_courses_nor_raw_is_rejected() {
        let json = r#"{"title":"Règle 1","constraint":{"count":1}}"#;
        assert!(serde_json::from_str::<Rule>(json).is_err());
    }

    #[test]
    fn rule_with_sentence_courses_but_no_raw_is_rejected() {
        // a parsed sentence must keep its source text
        let json = r#"{"title":"Règle 2","constraint":{"min":3,"max":3},"courses":"any"}"#;
        assert!(serde_json::from_str::<Rule>(json).is_err());
    }

    #[test]
    fn profile_without_credits_round_trips_without_the_key() {
        let json = r#"{"title":"Profil international","mandatory":["EHE-1GEX"],"rules":[]}"#;
        let profile: Profile = serde_json::from_str(json).expect("profile");
        assert_eq!(profile.credits_required, None);
        assert_eq!(serde_json::to_string(&profile).expect("ser"), json);
    }

    // --- Concentration: the two fields a real page forced open ---

    #[test]
    fn concentration_keeps_its_mandatory_courses_and_notes() {
        // génie industriel and génie mécanique put a « Cours obligatoires »
        // accordion inside a concentration (ADR
        // `2026-07-cours-obligatoires-de-concentration`)
        let json = r#"{"title":"Robotique","credits_required":18,"mandatory":["GMC-3351"],"rules":[],"notes":["Un stage est exigé."]}"#;
        let concentration: Concentration =
            serde_json::from_str(json).expect("concentration");
        assert_eq!(concentration.mandatory, vec!["GMC-3351".to_string()]);
        assert_eq!(serde_json::to_string(&concentration).expect("ser"), json);
    }

    #[test]
    fn concentration_without_credits_round_trips_without_the_key() {
        let json = r#"{"title":"Robotique","mandatory":[],"rules":[]}"#;
        let concentration: Concentration =
            serde_json::from_str(json).expect("concentration");
        assert_eq!(concentration.credits_required, None);
        assert_eq!(serde_json::to_string(&concentration).expect("ser"), json);
    }

    // --- LanguageRequirement: exigence linguistique en champ dédié ---

    #[test]
    fn language_requirement_with_both_branches_round_trips() {
        // génie des eaux: francophone -> ANL-2020 (VEPT 53), non-francophone
        // -> FLS-2093 (TCF-TP 400 ET TCF-TP/ÉÉ 14 — deux seuils ET-liés)
        let json = r#"{"francophone":{"course":"ANL-2020","tests":[{"name":"VEPT","score":53}],"raw":"Pour la personne francophone, la réussite du cours ANL-2020 Intermediate English II (VEPT: 53) est requise pour diplômer."},"non_francophone":{"course":"FLS-2093","tests":[{"name":"TCF-TP","score":400},{"name":"TCF-TP/ÉÉ","score":14}],"raw":"Pour la personne non-francophone, la réussite du cours FLS-2093 Rédaction de textes argumentatifs (TCF-TP: 400 et TCF-TP/ÉÉ: 14) est requise pour diplômer."}}"#;
        let requirement: LanguageRequirement =
            serde_json::from_str(json).expect("requirement");
        assert_eq!(requirement.francophone.course, "ANL-2020");
        assert_eq!(
            requirement.francophone.tests,
            vec![PlacementTest {
                name: "VEPT".to_string(),
                score: 53
            }]
        );
        let french = requirement
            .non_francophone
            .as_ref()
            .expect("non_francophone");
        assert_eq!(french.course, "FLS-2093");
        assert_eq!(french.tests.len(), 2, "TCF-TP: 400 et TCF-TP/ÉÉ: 14");
        assert_eq!(
            serde_json::to_value(&requirement).expect("ser"),
            serde_json::from_str::<serde_json::Value>(json).expect("value")
        );
    }

    #[test]
    fn language_requirement_francophone_only_omits_non_francophone() {
        // the prose page layout (génie physique) states only the English branch
        let json = r#"{"francophone":{"course":"ANL-2020","tests":[{"name":"VEPT","score":53}],"raw":"Réussir le cours ANL-2020 Intermediate English II."}}"#;
        let requirement: LanguageRequirement =
            serde_json::from_str(json).expect("requirement");
        assert_eq!(requirement.non_francophone, None);
        assert_eq!(serde_json::to_string(&requirement).expect("ser"), json);
    }

    #[test]
    fn language_qualification_without_tests_omits_the_key() {
        // raw is always kept; tests is empty when no threshold is parsed
        let json = r#"{"course":"ANL-2020","raw":"Réussir le cours ANL-2020 Intermediate English II."}"#;
        let qualification: LanguageQualification =
            serde_json::from_str(json).expect("qualification");
        assert!(qualification.tests.is_empty());
        assert_eq!(serde_json::to_string(&qualification).expect("ser"), json);
    }

    #[test]
    fn program_without_language_requirement_omits_the_key() {
        let json = r#"{"code":"x","title":"X","cycle":1,"credits_required":120,"mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#;
        let program: Program = serde_json::from_str(json).expect("program");
        assert_eq!(program.language_requirement, None);
        assert!(!serde_json::to_string(&program)
            .expect("ser")
            .contains("language_requirement"));
    }
}
