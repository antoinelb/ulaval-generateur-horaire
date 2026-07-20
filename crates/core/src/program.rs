use crate::common::Cycle;

// A block of the « Structure du programme » section, in its three roles. The
// prose a block carries — thematic subgroup labels, stage requirements, the
// English-level note — is understood by no grammar, so it rides along in
// `notes`: displayed to the student, never interpreted (ADR
// `2026-07-notes-en-prose-conservees`).
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
    Any { courses: Keyword, raw: String },
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
            RuleCourses::Any {
                courses: Keyword::Any,
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
}
