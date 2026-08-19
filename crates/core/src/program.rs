use crate::common::Cycle;
use crate::course::Season;

// the title of the « Stages » rule the scraper promotes from the génie
// bacs' graduation-stage prose — the intake finds the rule back by it (ADR
// `2026-08-stage-obligatoire-en-prose-promu-en-regle`)
pub const STAGES_RULE_TITLE: &str = "Stages";

// A block of the « Structure du programme » section, in its three roles. The
// prose a block carries — thematic subgroup labels, stage requirements — is
// understood by no grammar, so it rides along in `notes`: displayed to the
// student, never interpreted (ADR `2026-07-notes-en-prose-conservees`). The
// one exception is the language requirement, a course-or-test graduation gate
// lifted out into `language_requirement` (ADR
// `2026-07-exigence-linguistique-champ-dedie`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Program {
    // the official répertoire code, read from the page itself, its degree
    // prefix abridged to one letter — `B-GEX`, `M-GEX` even for the maîtrise
    // avec mémoire (ADR `2026-08-code-officiel-de-programme-et-slug`)
    pub code: String,
    // the page URL's last segment; the no-URL refresh rebuilds each program
    // URL from this field, so it must survive serialization (same ADR)
    pub slug: String,
    // the vintage the snapshot describes — the session that follows the
    // scrape, since programs change between sessions at no announced date
    // (ADR `2026-08-millesime-de-programme-en-semestre`); students keep the
    // version of the session they enrolled under
    pub semester: Semester,
    // the sessions that can start the program, in the order the page's
    // « Sessions d'admission » block lists them
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        with = "season_letters"
    )]
    // `season_letters` writes letters, not `Season`'s english words
    #[cfg_attr(feature = "tsify", tsify(type = "(\"A\" | \"H\" | \"E\")[]"))]
    pub possible_semester_start: Vec<Season>,
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

// A session vintage — « A26 » is automne 2026: the season letter the session
// naming already uses (uppercased) and a two-digit year. One string in JSON
// and in snapshot file names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Semester {
    pub season: Season,
    pub year: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown semester {raw:?}: expected A<yy>, H<yy> or E<yy> (e.g. A26)")]
pub struct SemesterError {
    pub raw: String,
}

impl std::fmt::Display for Semester {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{:02}", season_letter(self.season), self.year % 100)
    }
}

impl std::str::FromStr for Semester {
    type Err = SemesterError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let error = || SemesterError {
            raw: raw.to_string(),
        };
        let mut letters = raw.chars();
        let season = letters
            .next()
            .and_then(season_from_letter)
            .ok_or_else(error)?;
        // byte arithmetic, not `parse`: u16's parser accepts "+6", and a
        // two-digit year leaves no failing path for a coverage hole
        let year = match letters.as_str().as_bytes() {
            [tens @ b'0'..=b'9', ones @ b'0'..=b'9'] => {
                u16::from((tens - b'0') * 10 + (ones - b'0'))
            }
            _ => return Err(error()),
        };
        Ok(Semester {
            season,
            year: 2000 + year,
        })
    }
}

impl serde::Serialize for Semester {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Semester {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Profile {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_required: Option<i64>,
    pub mandatory: Vec<String>,
    pub rules: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Program {
    // A block is identified by its title alone — no id exists in the data —
    // so these two lookups are the one resolution rule every consumer
    // (coverage report, solver intake, credit tally) shares.
    pub fn concentration(&self, title: &str) -> Option<&Concentration> {
        self.concentrations
            .iter()
            .find(|block| block.title == title)
    }

    pub fn profile(&self, title: &str) -> Option<&Profile> {
        self.profiles.iter().find(|block| block.title == title)
    }
}

// no Tsify derive: the `flatten` would come out as `interface Rule extends
// RuleCourses`, invalid over a union — declared by hand in the boundary's
// custom section instead (ADR `2026-08-types-typescript-tsify-declaratif`)
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
    // « Les crédits de ces stages sont en sus des crédits exigés du
    // programme » : the rule must still be satisfied, but its credits do
    // not count toward `credits_required` (ADR
    // `2026-08-stage-obligatoire-en-prose-promu-en-regle`)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub credits_in_addition: bool,
}

// « Règle N – <contrainte> parmi : » — the counted unit and its bounds;
// « Un cours parmi » is min 1, max 1. The tag is load-bearing: untagged, a
// course count `{min, max}` would be byte-identical to a credits span, so
// the unit must be spelled out (ADR `2026-08-contrainte-etiquetee-min-max`).
// Whether to show a single number or a range is the UI's choice.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    Course { min: i64, max: i64 },
    Credits { min: i64, max: i64 },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct RuleReference {
    pub concentration: String,
    pub rule: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct LanguageRequirement {
    pub francophone: LanguageQualification,
    // only the two-box page layout spells out the non-francophone (French)
    // branch; the prose layout states the English one alone
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_francophone: Option<LanguageQualification>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct PlacementTest {
    pub name: String,
    pub score: i64,
}

// `possible_semester_start` as the page-facing letters (`["A", "H"]`),
// matching the vintage format rather than `Season`'s english serialization
mod season_letters {
    use serde::Deserialize;

    use super::{season_from_letter, season_letter, Season};

    pub fn serialize<S: serde::Serializer>(
        seasons: &[Season],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(
            seasons
                .iter()
                .map(|&season| season_letter(season).to_string()),
        )
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<Season>, D::Error> {
        let letters = Vec::<String>::deserialize(deserializer)?;
        letters
            .into_iter()
            .map(|letter| {
                let mut chars = letter.chars();
                match (chars.next().and_then(season_from_letter), chars.next())
                {
                    (Some(season), None) => Ok(season),
                    _ => Err(serde::de::Error::custom(format!(
                        "unknown season letter {letter:?}: expected A, H or E"
                    ))),
                }
            })
            .collect()
    }
}

// A = automne (fall), H = hiver (winter), E = été (summer) — the letters the
// session naming (`a2026`) already uses, uppercased for vintages
fn season_letter(season: Season) -> char {
    match season {
        Season::Fall => 'A',
        Season::Winter => 'H',
        Season::Summer => 'E',
    }
}

fn season_from_letter(letter: char) -> Option<Season> {
    match letter {
        'A' => Some(Season::Fall),
        'H' => Some(Season::Winter),
        'E' => Some(Season::Summer),
        _ => None,
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // --- Constraint: tagged {type, min, max} ---

    #[test]
    fn constraint_course_round_trips() {
        let constraint: Constraint =
            serde_json::from_str(r#"{"type":"course","min":1,"max":1}"#)
                .expect("course");
        assert_eq!(constraint, Constraint::Course { min: 1, max: 1 });
        assert_eq!(
            serde_json::to_string(&constraint).expect("ser"),
            r#"{"type":"course","min":1,"max":1}"#
        );
    }

    #[test]
    fn constraint_credits_round_trips() {
        let constraint: Constraint =
            serde_json::from_str(r#"{"type":"credits","min":3,"max":9}"#)
                .expect("credits");
        assert_eq!(constraint, Constraint::Credits { min: 3, max: 9 });
        assert_eq!(
            serde_json::to_string(&constraint).expect("ser"),
            r#"{"type":"credits","min":3,"max":9}"#
        );
    }

    #[test]
    fn a_constraint_with_an_unknown_type_is_rejected() {
        // the tag names the counted unit; a unit outside the grammar must
        // fail loudly, never fall back to one of the known two
        for json in [
            r#"{"type":"stage","min":1,"max":1}"#,
            r#"{"min":1,"max":1}"#,
            r#"{"count":1}"#,
        ] {
            assert!(
                serde_json::from_str::<Constraint>(json).is_err(),
                "{json}"
            );
        }
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
        let json = r#"{"title":"Règle 1","constraint":{"type":"course","min":1,"max":1},"courses":["GCI-1000","GEX-1000"]}"#;
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
        let json = r#"{"title":"Règle 2","constraint":{"type":"credits","min":3,"max":3},"courses":{"concentration":"Cheminement sans concentration","rule":"Règle 1"},"raw":"tous les cours de la Règle 1 du cheminement sans concentration"}"#;
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
        let json = r#"{"title":"Règle 2","constraint":{"type":"credits","min":3,"max":3},"courses":"any","raw":"tous les cours de premier cycle"}"#;
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
        let json = r#"{"title":"Règle 2","constraint":{"type":"credits","min":3,"max":3},"raw":"hors grammaire"}"#;
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
        let json = r#"{"title":"Règle 4","constraint":{"type":"credits","min":3,"max":3},"courses":["IFT-4902"],"notes":["Programmation"]}"#;
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
    fn rule_credits_in_addition_round_trips_and_vanishes_when_false() {
        // the génie stage rule: satisfied like any rule, credits « en sus »
        let json = r#"{"title":"Stages","constraint":{"type":"course","min":1,"max":8},"courses":["GEX-1580"],"credits_in_addition":true}"#;
        let rule = assert_rule_round_trips(json);
        assert!(rule.credits_in_addition);

        // an ordinary rule serializes no key at all
        let ordinary = Rule {
            credits_in_addition: false,
            ..rule
        };
        assert!(!serde_json::to_string(&ordinary)
            .expect("ser")
            .contains("credits_in_addition"));
    }

    #[test]
    fn rule_without_courses_nor_raw_is_rejected() {
        let json = r#"{"title":"Règle 1","constraint":{"type":"course","min":1,"max":1}}"#;
        assert!(serde_json::from_str::<Rule>(json).is_err());
    }

    #[test]
    fn rule_with_sentence_courses_but_no_raw_is_rejected() {
        // a parsed sentence must keep its source text
        let json = r#"{"title":"Règle 2","constraint":{"type":"credits","min":3,"max":3},"courses":"any"}"#;
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
        let json = r#"{"code":"x","slug":"x","semester":"A26","title":"X","cycle":1,"credits_required":120,"mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#;
        let program: Program = serde_json::from_str(json).expect("program");
        assert_eq!(program.language_requirement, None);
        assert!(!serde_json::to_string(&program)
            .expect("ser")
            .contains("language_requirement"));
    }

    // --- Semester: the « A26 » vintage string ---

    #[test]
    fn semester_round_trips_each_season_letter() {
        for (raw, season, year) in [
            ("A26", Season::Fall, 2026),
            ("H27", Season::Winter, 2027),
            ("E07", Season::Summer, 2007),
        ] {
            let semester: Semester = raw.parse().expect(raw);
            assert_eq!(semester, Semester { season, year });
            assert_eq!(semester.to_string(), raw, "Display mirrors FromStr");
            let json = format!("\"{raw}\"");
            assert_eq!(
                serde_json::to_string(&semester).expect("ser"),
                json,
                "serde uses the same string"
            );
            assert_eq!(
                serde_json::from_str::<Semester>(&json).expect("de"),
                semester
            );
        }
    }

    #[test]
    fn a_malformed_semester_is_rejected_with_its_input_named() {
        // lowercase, unknown letter, long year, short year, sign trick, empty
        for raw in ["a26", "X26", "A2026", "A2", "A+6", "26", ""] {
            let error = raw.parse::<Semester>().expect_err(raw);
            assert!(error.to_string().contains(raw), "got {error}");
        }
        assert!(serde_json::from_str::<Semester>("\"A2\"").is_err());
        // the JSON shape itself can be wrong, not just the string inside
        assert!(
            serde_json::from_str::<Semester>("26").is_err(),
            "a bare number is not a semester"
        );
    }

    // --- possible_semester_start: A/H/E letters on the wire ---

    #[test]
    fn possible_semester_start_round_trips_as_letters() {
        let json = r#"{"code":"x","slug":"x","semester":"A26","possible_semester_start":["A","H","E"],"title":"X","cycle":1,"credits_required":120,"mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#;
        let program: Program = serde_json::from_str(json).expect("program");
        assert_eq!(
            program.possible_semester_start,
            vec![Season::Fall, Season::Winter, Season::Summer]
        );
        assert_eq!(serde_json::to_string(&program).expect("ser"), json);
    }

    #[test]
    fn an_empty_possible_semester_start_omits_the_key() {
        let json = r#"{"code":"x","slug":"x","semester":"A26","title":"X","cycle":1,"credits_required":120,"mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#;
        let program: Program = serde_json::from_str(json).expect("program");
        assert!(program.possible_semester_start.is_empty());
        assert!(!serde_json::to_string(&program)
            .expect("ser")
            .contains("possible_semester_start"));
    }

    #[test]
    fn an_unknown_season_letter_is_rejected() {
        // one wrong letter and one two-letter entry: both name the culprit
        for letter in ["Z", "AH"] {
            let json = format!(
                r#"{{"code":"x","slug":"x","semester":"A26","possible_semester_start":["{letter}"],"title":"X","cycle":1,"credits_required":120,"mandatory":[],"rules":[],"concentrations":[],"profiles":[]}}"#
            );
            let error = serde_json::from_str::<Program>(&json)
                .expect_err("an unknown letter must not pass");
            assert!(error.to_string().contains(letter), "got {error}");
        }
        // the JSON shape itself can be wrong, not just a letter inside
        let json = r#"{"code":"x","slug":"x","semester":"A26","possible_semester_start":"AH","title":"X","cycle":1,"credits_required":120,"mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#;
        assert!(
            serde_json::from_str::<Program>(json).is_err(),
            "a bare string is not a season list"
        );
    }
}
