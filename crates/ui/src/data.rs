use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{Course, Program};
use ulaval_scheduler_ui_calculations::merge::merge_manual;

// Everything the app fetched, still unparsed: the fetch lives in `browser`
// (wasm-only glue), the parse here — testable without a browser.
#[derive(Debug, Clone, PartialEq)]
pub struct RawData {
    pub courses: String,
    // None = meta.json unavailable — an honest unknown, never a blocker
    // (ERR-5: one auxiliary file degrades one line, not the app)
    pub meta: Option<String>,
    pub programs: Vec<(String, String)>, // (file name, contents)
}

// The one immutable catalogue the whole app reads — provided once as a
// context, read by reference, never cloned (ADR
// `2026-07-donnees-servies-en-assets-du-harnais`).
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub courses: Vec<Course>,
    // code → position in `courses`
    pub by_code: BTreeMap<String, usize>,
    pub programs: Vec<Program>,
    // manual codes shadowed by a scraped course — displayed, never only
    // logged (ADR `2026-07-contribution-de-cours-manuels`)
    pub collisions: Vec<String>,
    // codes that came from the student's hand, marked « manuel » on screen
    pub manual_codes: BTreeSet<String>,
    // anything the load had to tolerate — surfaced, never silent
    pub warnings: Vec<String>,
    pub provenance: Provenance,
}

// TRU-2/BLD-4: what the footer and every diagnostic block say about the
// data actually loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    // the scraper's own stamp (ADR
    // `2026-08-meta-json-provenance-du-snapshot`); None = « date de
    // récolte inconnue », never a guessed date
    pub scraped_at: Option<String>,
    pub course_count: usize,
    // fnv1a-64 of the raw bytes, hex — ties any screenshot to its data
    pub data_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DataError {
    #[error("fetching {file} : {detail}")]
    Fetch { file: String, detail: String },
    #[error("parsing {file} : {detail}")]
    Parse { file: String, detail: String },
}

#[derive(serde::Deserialize)]
struct CoursesFile {
    courses: Vec<Course>,
}

#[derive(Default, serde::Deserialize)]
struct Meta {
    scraped_at: Option<String>,
}

pub fn parse_data(
    raw: &RawData,
    manual: Vec<Course>,
) -> Result<Snapshot, DataError> {
    let file: CoursesFile = parse(&raw.courses, "cours.json")?;
    let mut warnings = Vec::new();
    let meta = parse_meta(raw.meta.as_deref(), &mut warnings);
    let named_programs: Vec<(String, Program)> = raw
        .programs
        .iter()
        .map(|(name, contents)| {
            parse(contents, name).map(|program| (name.clone(), program))
        })
        .collect::<Result<_, _>>()?;
    // two files carrying the same (code, vintage) would be two identical
    // picker entries: keep the first, name the ignored file (the A24 file
    // with an A26 inside is the known case)
    let mut kept: Vec<(String, Program)> = Vec::new();
    for (name, program) in named_programs {
        let vintage = format!("{}-{}", program.code, program.semester);
        let twin = kept.iter().position(|(_, held)| {
            held.code == program.code && held.semester == program.semester
        });
        match twin {
            None => kept.push((name, program)),
            Some(index) => {
                // keep the file whose name agrees with its content — the
                // A24-named file carrying an A26 inside is the known case
                let held_name_agrees = kept[index].0.starts_with(&vintage);
                let (kept_name, dropped_name) = if held_name_agrees {
                    (kept[index].0.clone(), name)
                } else {
                    let dropped = kept[index].0.clone();
                    kept[index] = (name, program);
                    (kept[index].0.clone(), dropped)
                };
                warnings.push(format!(
                    "{dropped_name} ignoré : {kept_name} porte déjà cette \
                     version du programme (probable erreur de millésime \
                     dans le fichier ignoré)."
                ));
            }
        }
    }
    // several vintages of one program are all offered — the picker lists
    // them (code, then millésime), deterministically
    let mut programs: Vec<Program> =
        kept.into_iter().map(|(_, program)| program).collect();
    programs.sort_by_key(|program| {
        (program.code.clone(), program.semester.to_string())
    });
    let data_hash = format!("{:016x}", hash_raw(raw));
    let manual_codes: BTreeSet<String> =
        manual.iter().map(|course| course.code.clone()).collect();
    let merged = merge_manual(file.courses, manual);
    Ok(Snapshot {
        manual_codes: manual_codes
            .into_iter()
            .filter(|code| !merged.collisions.contains(code))
            .collect(),
        by_code: merged
            .courses
            .iter()
            .enumerate()
            .map(|(i, course)| (course.code.clone(), i))
            .collect(),
        provenance: Provenance {
            scraped_at: meta.scraped_at,
            course_count: merged.courses.len(),
            data_hash,
        },
        collisions: merged.collisions,
        warnings,
        courses: merged.courses,
        programs,
    })
}

fn parse<T: serde::de::DeserializeOwned>(
    contents: &str,
    file: &str,
) -> Result<T, DataError> {
    serde_json::from_str(contents).map_err(|error| DataError::Parse {
        file: file.to_string(),
        detail: error.to_string(),
    })
}

// the meta is auxiliary: absent or unreadable degrades to « date
// inconnue » with a visible warning instead of blocking the app
fn parse_meta(meta: Option<&str>, warnings: &mut Vec<String>) -> Meta {
    match meta {
        None => {
            warnings.push(
                "meta.json indisponible — date de récolte inconnue"
                    .to_string(),
            );
            Meta::default()
        }
        Some(contents) => match serde_json::from_str(contents) {
            Ok(meta) => meta,
            Err(error) => {
                warnings.push(format!(
                    "meta.json illisible ({error}) — date de récolte inconnue"
                ));
                Meta::default()
            }
        },
    }
}

fn hash_raw(raw: &RawData) -> u64 {
    let seed = fnv1a_64(0xcbf2_9ce4_8422_2325, raw.courses.as_bytes());
    raw.programs.iter().fold(seed, |hash, (_, contents)| {
        fnv1a_64(hash, contents.as_bytes())
    })
}

// FNV-1a, 64 bits — 8 lines beat a hashing dependency (BLD-5)
pub fn fnv1a_64(seed: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(seed, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const COURSES: &str = r#"{"courses":[
        {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
         "prerequisites":null,"equivalents":[],
         "seasons":{"fall":{"last_offered":2026,"options":null}}}
    ]}"#;

    const PROGRAM: &str = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
        "title":"P","cycle":1,"credits_required":6,
        "mandatory":["GEX-1000"],
        "rules":[],"concentrations":[],"profiles":[]}"#;

    fn raw() -> RawData {
        RawData {
            courses: COURSES.to_string(),
            meta: Some(
                r#"{"scraped_at":"2026-08-13T18:00:00Z","course_count":1}"#
                    .to_string(),
            ),
            programs: vec![(
                "B-GEX-A26.json".to_string(),
                PROGRAM.to_string(),
            )],
        }
    }

    fn manual(code: &str) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"Manuel","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],"seasons":{{}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    #[test]
    fn a_full_load_indexes_and_stamps_the_catalogue() {
        let snapshot =
            parse_data(&raw(), Vec::new()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(snapshot.courses.len(), 1);
        assert_eq!(snapshot.by_code["GEX-1000"], 0);
        assert_eq!(snapshot.programs[0].code, "B-GEX");
        assert_eq!(
            snapshot.provenance.scraped_at.as_deref(),
            Some("2026-08-13T18:00:00Z")
        );
        assert_eq!(snapshot.provenance.course_count, 1);
        assert_eq!(snapshot.provenance.data_hash.len(), 16);
        assert!(snapshot.warnings.is_empty());
        assert!(snapshot.collisions.is_empty());
    }

    #[test]
    fn manual_courses_join_and_collisions_surface() {
        let snapshot =
            parse_data(&raw(), vec![manual("ANL-2020"), manual("GEX-1000")])
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(snapshot.by_code["ANL-2020"], 0, "sorted before GEX");
        assert_eq!(snapshot.collisions, ["GEX-1000"]);
        assert_eq!(snapshot.provenance.course_count, 2);
    }

    #[test]
    fn an_absent_or_unreadable_meta_degrades_with_a_visible_warning() {
        let mut absent = raw();
        absent.meta = None;
        let snapshot =
            parse_data(&absent, Vec::new()).unwrap_or_else(|e| panic!("{e}"));
        assert!(snapshot.provenance.scraped_at.is_none());
        assert!(snapshot.warnings[0].contains("indisponible"));

        let mut corrupt = raw();
        corrupt.meta = Some("pas du json".to_string());
        let snapshot =
            parse_data(&corrupt, Vec::new()).unwrap_or_else(|e| panic!("{e}"));
        assert!(snapshot.provenance.scraped_at.is_none());
        assert!(snapshot.warnings[0].contains("illisible"));
    }

    #[test]
    fn a_corrupt_courses_or_program_file_is_a_typed_error_naming_it() {
        let mut bad_courses = raw();
        bad_courses.courses = "{".to_string();
        assert!(matches!(
            parse_data(&bad_courses, Vec::new()),
            Err(DataError::Parse { file, .. }) if file == "cours.json"
        ));

        let mut bad_program = raw();
        bad_program.programs =
            vec![("B-GEX-A26.json".to_string(), "{".to_string())];
        assert!(matches!(
            parse_data(&bad_program, Vec::new()),
            Err(DataError::Parse { file, .. }) if file == "B-GEX-A26.json"
        ));
    }

    #[test]
    fn a_duplicated_program_vintage_keeps_the_agreeing_file() {
        // the mislabelled file comes first alphabetically: the one whose
        // name matches its content must still win, the other be named
        let mut duplicated = raw();
        duplicated.programs = vec![
            ("B-GEX-A24.json".to_string(), PROGRAM.to_string()),
            ("B-GEX-A26.json".to_string(), PROGRAM.to_string()),
        ];
        let snapshot = parse_data(&duplicated, Vec::new())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(snapshot.programs.len(), 1, "one picker entry");
        let warning = &snapshot.warnings[0];
        assert!(warning.starts_with("B-GEX-A24.json ignoré"), "{warning}");
        assert!(warning.contains("B-GEX-A26.json"), "{warning}");

        // and the agreeing file first: the duplicate is still dropped
        let mut agreeing_first = raw();
        agreeing_first.programs = vec![
            ("B-GEX-A26.json".to_string(), PROGRAM.to_string()),
            ("B-GEX-A24.json".to_string(), PROGRAM.to_string()),
        ];
        let snapshot = parse_data(&agreeing_first, Vec::new())
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            snapshot.warnings[0].starts_with("B-GEX-A24.json ignoré"),
            "{}",
            snapshot.warnings[0]
        );
    }

    #[test]
    fn two_true_vintages_both_survive_sorted_by_code_then_semester() {
        // A24 and A26 with distinct contents: two picker entries, no
        // warning — several millésimes of one program are all offered
        // (notes 2026-08-13)
        let a24 =
            PROGRAM.replace(r#""semester":"A26""#, r#""semester":"A24""#);
        let mut vintages = raw();
        vintages.programs = vec![
            ("B-GEX-A26.json".to_string(), PROGRAM.to_string()),
            ("B-GEX-A24.json".to_string(), a24),
        ];
        let snapshot = parse_data(&vintages, Vec::new())
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(snapshot.warnings.is_empty(), "{:?}", snapshot.warnings);
        let listed: Vec<(String, String)> = snapshot
            .programs
            .iter()
            .map(|program| {
                (program.code.clone(), program.semester.to_string())
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("B-GEX".to_string(), "A24".to_string()),
                ("B-GEX".to_string(), "A26".to_string())
            ],
            "sorted, both kept"
        );
    }

    #[test]
    fn a_manual_draft_builds_a_real_course_or_says_why_not() {
        let draft = ManualDraft {
            code: "gex-1234".to_string(),
            title: "Cours maison".to_string(),
            credits: "3".to_string(),
            nrc: String::new(),
            slots: vec![ManualSlot {
                day: "monday".to_string(),
                start: "8:30".to_string(),
                end: "11:20".to_string(),
            }],
        };
        let course = build_manual_course(
            &draft,
            ulaval_scheduler_core::Season::Fall,
            2026,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(course.code, "GEX-1234", "uppercased");
        let offering = &course.seasons[&ulaval_scheduler_core::Season::Fall];
        let options = offering.options.as_ref().expect("one option");
        assert_eq!(options[0][0].nrc, "M-GEX-1234", "default NRC");
        assert_eq!(options[0][0].slots.len(), 1);

        let reject = |mutate: fn(&mut ManualDraft)| {
            let mut bad = draft.clone();
            mutate(&mut bad);
            build_manual_course(
                &bad,
                ulaval_scheduler_core::Season::Fall,
                2026,
            )
            .expect_err("must reject")
        };
        assert!(
            reject(|d| d.code = "  ".to_string()).contains("code de cours")
        );
        assert!(reject(|d| d.title = String::new()).contains("titre"));
        assert!(
            reject(|d| d.credits = "trois".to_string()).contains("Crédits")
        );
        assert!(reject(|d| d.slots[0].start = "25h99".to_string())
            .contains("invalide"));
    }

    #[test]
    fn an_empty_slot_row_is_skipped_not_an_error() {
        let draft = ManualDraft {
            code: "GEX-1234".to_string(),
            title: "Sans plage".to_string(),
            credits: "3".to_string(),
            nrc: "12345".to_string(),
            slots: vec![ManualSlot::default()],
        };
        let course = build_manual_course(
            &draft,
            ulaval_scheduler_core::Season::Winter,
            2027,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let offering = &course.seasons[&ulaval_scheduler_core::Season::Winter];
        let options = offering.options.as_ref().expect("one option");
        assert!(options[0][0].slots.is_empty(), "a remote-style course");
    }

    #[test]
    fn a_summer_manual_course_lands_in_its_season() {
        let course = build_manual_course(
            &ManualDraft {
                code: "ETE-9999".to_string(),
                title: "Cours d'été".to_string(),
                credits: "3".to_string(),
                nrc: String::new(),
                slots: Vec::new(),
            },
            ulaval_scheduler_core::Season::Summer,
            2027,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(course
            .seasons
            .contains_key(&ulaval_scheduler_core::Season::Summer));
    }

    #[test]
    fn adding_a_manual_course_keeps_the_snapshot_invariants() {
        let mut snapshot =
            parse_data(&raw(), Vec::new()).unwrap_or_else(|e| panic!("{e}"));
        let course = build_manual_course(
            &ManualDraft {
                code: "AAA-1000".to_string(),
                title: "Premier".to_string(),
                credits: "3".to_string(),
                nrc: String::new(),
                slots: Vec::new(),
            },
            ulaval_scheduler_core::Season::Fall,
            2026,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        add_manual_course(&mut snapshot, course)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(snapshot.by_code["AAA-1000"], 0, "sorted first");
        assert_eq!(snapshot.by_code["GEX-1000"], 1, "reindexed");
        assert!(snapshot.manual_codes.contains("AAA-1000"));
        assert_eq!(snapshot.provenance.course_count, 2);

        let doubled = build_manual_course(
            &ManualDraft {
                code: "GEX-1000".to_string(),
                title: "Doublon".to_string(),
                credits: "3".to_string(),
                nrc: String::new(),
                slots: Vec::new(),
            },
            ulaval_scheduler_core::Season::Fall,
            2026,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let error = add_manual_course(&mut snapshot, doubled)
            .expect_err("the scraped course wins");
        assert!(error.contains("prime"), "{error}");
    }

    #[test]
    fn the_hash_is_the_known_fnv1a_vector_and_tracks_every_file() {
        // the empty input keeps the offset basis — the published vector
        assert_eq!(
            fnv1a_64(0xcbf2_9ce4_8422_2325, b""),
            0xcbf2_9ce4_8422_2325
        );
        assert_eq!(
            fnv1a_64(0xcbf2_9ce4_8422_2325, b"a"),
            0xaf63_dc4c_8601_ec8c
        );

        let mut touched = raw();
        touched.programs =
            vec![("B-GEX-A26.json".to_string(), "{}".to_string())];
        assert_ne!(hash_raw(&raw()), hash_raw(&touched));
    }

    #[test]
    fn both_error_variants_read_in_full() {
        let fetch = DataError::Fetch {
            file: "cours.json".to_string(),
            detail: "HTTP 404".to_string(),
        };
        assert_eq!(fetch.to_string(), "fetching cours.json : HTTP 404");
        let parse = DataError::Parse {
            file: "meta.json".to_string(),
            detail: "expected value".to_string(),
        };
        assert_eq!(parse.to_string(), "parsing meta.json : expected value");
    }
}

// --- manual courses (ADR `2026-07-contribution-de-cours-manuels`) ----------

// A hand-entered course, exactly as the form holds it — validation happens
// by building a real `Course` through serde, so the same types that guard
// the snapshot guard the student's input (units as types, AIR TIM-4).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManualDraft {
    pub code: String,
    pub title: String,
    pub credits: String,
    pub nrc: String,
    pub slots: Vec<ManualSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManualSlot {
    // english serde key (« monday »), the form shows French labels
    pub day: String,
    pub start: String,
    pub end: String,
}

pub fn build_manual_course(
    draft: &ManualDraft,
    season: ulaval_scheduler_core::Season,
    year: u16,
) -> Result<Course, String> {
    let code = draft.code.trim().to_uppercase();
    if code.is_empty() {
        return Err("Entrez un code de cours (ex. GEX-1234).".to_string());
    }
    let title = draft.title.trim();
    if title.is_empty() {
        return Err("Entrez le titre du cours.".to_string());
    }
    let credits: u32 = draft.credits.trim().parse().map_err(|_| {
        "Crédits : entrez un nombre entier (ex. 3).".to_string()
    })?;
    let nrc = match draft.nrc.trim() {
        "" => format!("M-{code}"),
        nrc => nrc.to_string(),
    };
    let slots: Vec<serde_json::Value> = draft
        .slots
        .iter()
        .filter(|slot| {
            !(slot.start.trim().is_empty() && slot.end.trim().is_empty())
        })
        .map(|slot| {
            serde_json::json!({
                "day": slot.day,
                "start": slot.start.trim(),
                "end": slot.end.trim(),
            })
        })
        .collect();
    let season_key = match season {
        ulaval_scheduler_core::Season::Fall => "fall",
        ulaval_scheduler_core::Season::Winter => "winter",
        ulaval_scheduler_core::Season::Summer => "summer",
    };
    let value = serde_json::json!({
        "code": code,
        "title": title,
        "credits": credits,
        "cycle": 1,
        "prerequisites": null,
        "equivalents": [],
        "seasons": {
            season_key: {
                "last_offered": year,
                "options": [[{
                    "nrc": nrc,
                    // spelled out: a bare « M » on the grid block reads
                    // as a section letter, not as « fait main »
                    "section": "manuel",
                    "mode": "in-person",
                    "slots": slots,
                }]],
            },
        },
    });
    // the serde types are the validator: a bad HH:MM or day dies here
    serde_json::from_value(value).map_err(|error| {
        format!("Le cours entré est invalide : {error}. Heures au format HH:MM (ex. 8:30).")
    })
}

// insert a hand-entered course into the loaded snapshot, keeping its
// invariants (sorted by code, indexed); a collision keeps the scraped
// course and says so (the ADR's « le scrapé prime »)
pub fn add_manual_course(
    snapshot: &mut Snapshot,
    course: Course,
) -> Result<(), String> {
    if snapshot.by_code.contains_key(&course.code) {
        return Err(format!(
            "{} existe déjà dans le catalogue — le cours officiel prime.",
            course.code
        ));
    }
    snapshot.manual_codes.insert(course.code.clone());
    snapshot.courses.push(course);
    snapshot.courses.sort_by(|a, b| a.code.cmp(&b.code));
    snapshot.by_code = snapshot
        .courses
        .iter()
        .enumerate()
        .map(|(i, course)| (course.code.clone(), i))
        .collect();
    snapshot.provenance.course_count = snapshot.courses.len();
    Ok(())
}
