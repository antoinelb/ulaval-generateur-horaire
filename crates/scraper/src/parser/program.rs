use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};
use ulaval_scheduler_core::{
    Concentration, Constraint, Cycle, Keyword, Profile, Program, Rule,
    RuleCourses, RuleReference,
};

use crate::parser::ParseError;

const TITLE_CSS: &str = "h1";
static TITLE: LazyLock<Selector> = LazyLock::new(|| sel(TITLE_CSS));

// the code is the URL slug: the page never writes it in its own body
const CANONICAL_CSS: &str = r#"link[rel="canonical"]"#;
static CANONICAL: LazyLock<Selector> = LazyLock::new(|| sel(CANONICAL_CSS));

// the « En bref » credits card — the same `promo-entete` pair the course
// page carries, in another container
const PROMO_CSS: &str = "div.bloc-promo";
static PROMO: LazyLock<Selector> = LazyLock::new(|| sel(PROMO_CSS));
const PROMO_VALUE_CSS: &str = "span.promo-entete--titre";
static PROMO_VALUE: LazyLock<Selector> =
    LazyLock::new(|| sel(PROMO_VALUE_CSS));
static PROMO_LABEL: LazyLock<Selector> =
    LazyLock::new(|| sel("span.promo-entete--contenu"));

const STRUCTURE_CSS: &str = "section#section-structure";
static STRUCTURE: LazyLock<Selector> = LazyLock::new(|| sel(STRUCTURE_CSS));

// « Structure du programme » nests strictly:
//
//   section#section-structure
//    └ div.fe-bloc-section                    ← group, optional <h3> label
//       ├ h3                                  ← « Concentrations » | « Profils »
//       └ div.collapsible-sections            ← one per block
//          ├ h4.fe-bloc-titre--texte          ← block title
//          ├ span.fe-bloc-titre--credits      ← « N crédits exigés »
//          ├ div.toggle-section               ← « Cours obligatoires » | « Règle N – … »
//          └ div.fe-bloc-section--paragraphe  ← prose note of the block
//
// Accordions never nest on a program page — one `span.item` per
// `div.toggle-section` on the six frozen fixtures — so a descendant scan
// inside a block cannot stray into another block's rules.
static GROUP: LazyLock<Selector> =
    LazyLock::new(|| sel("div.fe-bloc-section"));
const GROUP_CSS: &str = "div.fe-bloc-section";
static GROUP_HEADING: LazyLock<Selector> = LazyLock::new(|| sel("h3"));

static BLOCK: LazyLock<Selector> =
    LazyLock::new(|| sel("div.collapsible-sections"));
const BLOCK_TITLE_CSS: &str = "h4.fe-bloc-titre--texte";
static BLOCK_TITLE: LazyLock<Selector> =
    LazyLock::new(|| sel(BLOCK_TITLE_CSS));
const BLOCK_CREDITS_CSS: &str = "span.fe-bloc-titre--credits";
static BLOCK_CREDITS: LazyLock<Selector> =
    LazyLock::new(|| sel(BLOCK_CREDITS_CSS));
static BLOCK_NOTE: LazyLock<Selector> =
    LazyLock::new(|| sel("div.fe-bloc-section--paragraphe"));

static ACCORDION: LazyLock<Selector> =
    LazyLock::new(|| sel("div.toggle-section"));
const ACCORDION_HEADING_CSS: &str = "p.toggle-section--header span.item";
static ACCORDION_HEADING: LazyLock<Selector> =
    LazyLock::new(|| sel(ACCORDION_HEADING_CSS));
static ACCORDION_BODY: LazyLock<Selector> =
    LazyLock::new(|| sel("div.toggle-section--content"));

const COURSE_CODE_CSS: &str = "span.cours-carte--sigle";
static COURSE_CODE: LazyLock<Selector> =
    LazyLock::new(|| sel(COURSE_CODE_CSS));
const RULE_LINE_CSS: &str = "p.fe-bloc-regle--ligne";
static RULE_LINE: LazyLock<Selector> = LazyLock::new(|| sel(RULE_LINE_CSS));

// the one accordion that states no constraint: its courses are required
const MANDATORY_HEADING: &str = "Cours obligatoires";
const CONCENTRATIONS_HEADING: &str = "Concentrations";
const PROFILES_HEADING: &str = "Profils";

const ANY_PROSE: &str = "tous les cours de premier cycle";
const REFERENCE_PROSE: &str = "tous les cours de la ";

#[derive(Debug)]
pub struct ProgramPage {
    pub program: Program,
    pub anomalies: Vec<ParseError>,
}

// The page read verbatim, before any block is given a role: a rule may name
// a concentration by title, so every title has to be known before the first
// rule is built.
struct Group {
    kind: GroupKind,
    blocks: Vec<Block>,
}

#[derive(Clone, Copy, PartialEq)]
enum GroupKind {
    Program,
    Concentrations,
    Profiles,
}

struct Block {
    title: String,
    credits: Option<i64>,
    accordions: Vec<Accordion>,
    notes: Vec<String>,
}

struct Accordion {
    heading: String,
    courses: Vec<String>,
    lines: Vec<String>,
}

// what a block yields, whichever of the three roles it plays
struct Contents {
    title: String,
    credits: Option<i64>,
    mandatory: Vec<String>,
    rules: Vec<Rule>,
    notes: Vec<String>,
}

#[derive(Default)]
struct Structure {
    mandatory: Vec<String>,
    rules: Vec<Rule>,
    concentrations: Vec<Concentration>,
    profiles: Vec<Profile>,
    notes: Vec<String>,
}

pub fn parse(html: &str) -> Result<ProgramPage, ParseError> {
    let doc = Html::parse_document(html);

    let mut anomalies = Vec::new();

    let title = parse_title(&doc)?;
    let cycle = parse_cycle(&title)?;
    let code = parse_code(&doc)?;
    let credits_required = parse_credits_required(&doc)?;
    let structure = parse_structure(&doc, &mut anomalies)?;

    Ok(ProgramPage {
        program: Program {
            code,
            title,
            cycle,
            credits_required,
            mandatory: structure.mandatory,
            rules: structure.rules,
            concentrations: structure.concentrations,
            profiles: structure.profiles,
            notes: structure.notes,
        },
        anomalies,
    })
}

// the source breaks the heading across lines (« Baccalauréat en\n\t\tgénie
// des eaux »), so the title is rebuilt word by word rather than trimmed
fn parse_title(doc: &Html) -> Result<String, ParseError> {
    doc.select(&TITLE)
        .next()
        .map(|element| collapse(&element.text().collect::<String>()))
        .filter(|title| !title.is_empty())
        .ok_or_else(|| ParseError::MissingElement {
            selector: TITLE_CSS.to_string(),
        })
}

// No markup names the cycle — only the kind of diploma the title opens
// with. A « Doctorat » is a third cycle, which `Cycle` cannot hold (ADR
// `2026-07-troisieme-cycle-hors-perimetre`), so it is rejected rather than
// filed under the nearest neighbour.
fn parse_cycle(title: &str) -> Result<Cycle, ParseError> {
    match title.split_whitespace().next() {
        Some("Baccalauréat") | Some("Certificat") => Ok(Cycle::First),
        Some("Maîtrise") | Some("DESS") => Ok(Cycle::Second),
        _ => Err(ParseError::MalformedEntry {
            selector: "cycle".to_string(),
            raw: title.to_string(),
        }),
    }
}

fn parse_code(doc: &Html) -> Result<String, ParseError> {
    let href = doc
        .select(&CANONICAL)
        .next()
        .and_then(|link| link.value().attr("href"))
        .ok_or_else(|| ParseError::MissingElement {
            selector: CANONICAL_CSS.to_string(),
        })?;

    href.rsplit('/')
        .find(|segment| !segment.is_empty())
        .map(str::to_string)
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: CANONICAL_CSS.to_string(),
            raw: href.to_string(),
        })
}

// The card of the « En bref » block, not the « N crédits exigés » of a
// block header: the latter counts one block, this one counts the diploma.
fn parse_credits_required(doc: &Html) -> Result<i64, ParseError> {
    let card = doc
        .select(&PROMO)
        .find(|card| {
            card.select(&PROMO_LABEL).next().is_some_and(|label| {
                collapse(&label.text().collect::<String>())
                    .starts_with("Crédit")
            })
        })
        .ok_or_else(|| ParseError::MissingElement {
            selector: format!("{PROMO_CSS} = Crédits"),
        })?;

    let raw = card
        .select(&PROMO_VALUE)
        .next()
        .map(|value| collapse(&value.text().collect::<String>()))
        .ok_or_else(|| ParseError::MissingElement {
            selector: PROMO_VALUE_CSS.to_string(),
        })?;

    raw.parse::<i64>().map_err(|_| ParseError::MalformedEntry {
        selector: "credits_required".to_string(),
        raw,
    })
}

fn parse_structure(
    doc: &Html,
    anomalies: &mut Vec<ParseError>,
) -> Result<Structure, ParseError> {
    let section = doc.select(&STRUCTURE).next().ok_or_else(|| {
        ParseError::MissingElement {
            selector: STRUCTURE_CSS.to_string(),
        }
    })?;

    let groups: Vec<Group> = section
        .select(&GROUP)
        .enumerate()
        .map(|(index, group)| parse_group(group, index, anomalies))
        .collect();

    // « tous les cours de la Règle N du cheminement X » names a
    // concentration by title, so the titles are collected before any rule
    // is built — resolution to a course list happens in `core`
    let concentrations: Vec<String> = groups
        .iter()
        .filter(|group| group.kind == GroupKind::Concentrations)
        .flat_map(|group| group.blocks.iter().map(|block| block.title.clone()))
        .collect();

    Ok(assemble(groups, &concentrations, anomalies))
}

fn parse_group(
    group: ElementRef,
    index: usize,
    anomalies: &mut Vec<ParseError>,
) -> Group {
    let heading = group
        .select(&GROUP_HEADING)
        .next()
        .map(|element| collapse(&element.text().collect::<String>()));
    let blocks: Vec<Block> = group
        .select(&BLOCK)
        .filter_map(|block| parse_block(block, anomalies))
        .collect();

    Group {
        kind: classify_group(
            heading.as_deref(),
            index,
            blocks.len(),
            anomalies,
        ),
        blocks,
    }
}

// `<h3>` labels the role of a group — except on the bac en génie mécanique,
// which omits it on its concentrations. What tells that group apart from a
// rules block like « Autres exigences » is that it holds several blocks;
// the assumption is reported, so a page that breaks it says so (ADR
// `2026-07-blocs-de-la-page-programme`).
fn classify_group(
    heading: Option<&str>,
    index: usize,
    blocks: usize,
    anomalies: &mut Vec<ParseError>,
) -> GroupKind {
    match (heading, index, blocks) {
        (Some(CONCENTRATIONS_HEADING), _, _) => GroupKind::Concentrations,
        (Some(PROFILES_HEADING), _, _) => GroupKind::Profiles,
        (None, 0, _) | (None, _, 0 | 1) => GroupKind::Program,
        (None, _, _) => {
            anomalies.push(ParseError::MalformedEntry {
                selector: GROUP_CSS.to_string(),
                raw: format!(
                    "{blocks} blocks under no heading, read as concentrations"
                ),
            });
            GroupKind::Concentrations
        }
        (Some(other), _, _) => {
            anomalies.push(ParseError::MalformedEntry {
                selector: "h3".to_string(),
                raw: other.to_string(),
            });
            GroupKind::Program
        }
    }
}

fn parse_block(
    block: ElementRef,
    anomalies: &mut Vec<ParseError>,
) -> Option<Block> {
    let Some(title) = block
        .select(&BLOCK_TITLE)
        .next()
        .map(|element| collapse(&element.text().collect::<String>()))
        .filter(|title| !title.is_empty())
    else {
        anomalies.push(ParseError::MissingElement {
            selector: BLOCK_TITLE_CSS.to_string(),
        });
        return None;
    };

    Some(Block {
        credits: parse_block_credits(block, &title, anomalies),
        accordions: block.select(&ACCORDION).map(parse_accordion).collect(),
        notes: block
            .select(&BLOCK_NOTE)
            .map(|note| collapse(&note.text().collect::<String>()))
            .filter(|note| !note.is_empty())
            .collect(),
        title,
    })
}

// « 18 crédits exigés ». A block stating no total (« Profil international »)
// carries no such span at all, which is a fact about the block rather than
// markup drift; a span that is there but unreadable is drift.
fn parse_block_credits(
    block: ElementRef,
    title: &str,
    anomalies: &mut Vec<ParseError>,
) -> Option<i64> {
    let raw = collapse(
        &block
            .select(&BLOCK_CREDITS)
            .next()?
            .text()
            .collect::<String>(),
    );

    match raw.split_whitespace().next().and_then(|n| n.parse().ok()) {
        Some(credits) => Some(credits),
        None => {
            anomalies.push(ParseError::MalformedEntry {
                selector: format!("{BLOCK_CREDITS_CSS} ({title})"),
                raw,
            });
            None
        }
    }
}

fn parse_accordion(accordion: ElementRef) -> Accordion {
    let body = accordion.select(&ACCORDION_BODY).next();

    Accordion {
        heading: accordion
            .select(&ACCORDION_HEADING)
            .next()
            .map(|element| collapse(&element.text().collect::<String>()))
            .unwrap_or_default(),
        courses: body
            .iter()
            .flat_map(|body| body.select(&COURSE_CODE))
            .map(|code| collapse(&code.text().collect::<String>()))
            .collect(),
        lines: body
            .iter()
            .flat_map(|body| body.select(&RULE_LINE))
            .map(|line| collapse(&line.text().collect::<String>()))
            .filter(|line| !line.is_empty())
            .collect(),
    }
}

fn assemble(
    groups: Vec<Group>,
    concentrations: &[String],
    anomalies: &mut Vec<ParseError>,
) -> Structure {
    let mut structure = Structure::default();
    let mut program_blocks = 0usize;

    for Group { kind, blocks } in groups {
        for block in blocks {
            // only the first block of the programme names its rules bare: a
            // later one (« Autres exigences ») would collide with the
            // « Règle 1 » of the first
            let prefixed = kind == GroupKind::Program && program_blocks > 0;
            let contents =
                block_contents(block, prefixed, concentrations, anomalies);

            match kind {
                GroupKind::Program => {
                    program_blocks += 1;
                    structure.mandatory.extend(contents.mandatory);
                    structure.rules.extend(contents.rules);
                    structure.notes.extend(contents.notes);
                }
                GroupKind::Concentrations => {
                    structure.concentrations.push(Concentration {
                        title: contents.title,
                        credits_required: contents.credits,
                        mandatory: contents.mandatory,
                        rules: contents.rules,
                        notes: contents.notes,
                    });
                }
                GroupKind::Profiles => structure.profiles.push(Profile {
                    title: contents.title,
                    credits_required: contents.credits,
                    mandatory: contents.mandatory,
                    rules: contents.rules,
                    notes: contents.notes,
                }),
            }
        }
    }

    structure
}

fn block_contents(
    block: Block,
    prefixed: bool,
    concentrations: &[String],
    anomalies: &mut Vec<ParseError>,
) -> Contents {
    let Block {
        title,
        credits,
        accordions,
        mut notes,
    } = block;
    let mut mandatory = Vec::new();
    let mut rules = Vec::new();

    for accordion in accordions {
        if accordion.heading == MANDATORY_HEADING {
            mandatory.extend(accordion.courses);
            notes.extend(accordion.lines);
        } else {
            rules.push(parse_rule(
                &title,
                prefixed,
                accordion,
                concentrations,
                anomalies,
            ));
        }
    }

    Contents {
        title,
        credits,
        mandatory,
        rules,
        notes,
    }
}

fn parse_rule(
    block: &str,
    prefixed: bool,
    accordion: Accordion,
    concentrations: &[String],
    anomalies: &mut Vec<ParseError>,
) -> Rule {
    let Accordion {
        heading,
        courses,
        lines,
    } = accordion;

    let (name, constraint) = parse_rule_heading(&heading, anomalies);
    let (courses, notes) = parse_rule_courses(
        &heading,
        courses,
        lines,
        concentrations,
        anomalies,
    );

    Rule {
        title: if prefixed {
            format!("{block} – {name}")
        } else {
            name
        },
        constraint,
        courses,
        notes,
    }
}

// « Règle 1 – Un cours parmi : » splits on the en dash the page itself
// writes: what precedes names the rule, what follows constrains it.
fn parse_rule_heading(
    heading: &str,
    anomalies: &mut Vec<ParseError>,
) -> (String, Option<Constraint>) {
    let Some((name, constraint)) = heading.split_once('–') else {
        anomalies.push(ParseError::MalformedEntry {
            selector: ACCORDION_HEADING_CSS.to_string(),
            raw: heading.to_string(),
        });
        return (heading.trim().to_string(), None);
    };

    let parsed = parse_constraint(constraint);
    if parsed.is_none() {
        anomalies.push(ParseError::MalformedEntry {
            selector: "constraint".to_string(),
            raw: heading.to_string(),
        });
    }

    (name.trim().to_string(), parsed)
}

// The three shapes the page writes, each also seen with its « parmi : »
// tail cut off (génie physique, industriel, mécanique).
fn parse_constraint(text: &str) -> Option<Constraint> {
    match text.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["Un", "cours", ..] => Some(Constraint::Count { count: 1 }),
        [min, "à", max, "crédits", ..] => Some(Constraint::Credits {
            min: min.parse().ok()?,
            max: max.parse().ok()?,
        }),
        [count, "crédits" | "crédit", ..] => {
            let count = count.parse().ok()?;
            Some(Constraint::Credits {
                min: count,
                max: count,
            })
        }
        _ => None,
    }
}

// A rule either lists course cards or states its content in prose. The
// prose is kept whole — the grammar recognizes a prefix of it, never a
// rewrite of it (ADR `2026-07-texte-brut-de-regle-paragraphe-complet`) —
// and every further line is a note.
fn parse_rule_courses(
    heading: &str,
    courses: Vec<String>,
    lines: Vec<String>,
    concentrations: &[String],
    anomalies: &mut Vec<ParseError>,
) -> (RuleCourses, Vec<String>) {
    if !courses.is_empty() {
        return (RuleCourses::List { courses }, lines);
    }

    let mut lines = lines.into_iter();
    let Some(raw) = lines.next() else {
        anomalies.push(ParseError::MissingElement {
            selector: format!(
                "{COURSE_CODE_CSS} nor {RULE_LINE_CSS} ({heading})"
            ),
        });
        return (RuleCourses::Raw { raw: String::new() }, Vec::new());
    };

    (
        classify_prose(raw, concentrations, anomalies),
        lines.collect(),
    )
}

fn classify_prose(
    raw: String,
    concentrations: &[String],
    anomalies: &mut Vec<ParseError>,
) -> RuleCourses {
    if raw.starts_with(ANY_PROSE) {
        return RuleCourses::Any {
            courses: Keyword::Any,
            raw,
        };
    }

    match parse_reference(&raw, concentrations) {
        Some(reference) => RuleCourses::Reference {
            courses: reference,
            raw,
        },
        // out of grammar: kept whole and surfaced, never interpreted
        None => {
            anomalies.push(ParseError::MalformedEntry {
                selector: "rule".to_string(),
                raw: raw.clone(),
            });
            RuleCourses::Raw { raw }
        }
    }
}

// « tous les cours de la Règle 1 du cheminement sans concentration » — the
// target is named as the page titles it, so the reference is stored by
// title and resolved in `core` (ADR `2026-07-reference-de-regle-structuree`)
fn parse_reference(
    raw: &str,
    concentrations: &[String],
) -> Option<RuleReference> {
    let (rule, target) =
        raw.strip_prefix(REFERENCE_PROSE)?.split_once(" du ")?;
    let target = target.trim_end_matches('.').trim();

    concentrations
        .iter()
        .find(|title| title.eq_ignore_ascii_case(target))
        .map(|title| RuleReference {
            concentration: title.clone(),
            rule: rule.trim().to_string(),
        })
}

// every text the page writes is broken across lines and padded with tabs,
// and « &nbsp; » counts as whitespace like any other
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sel(selector: &str) -> Selector {
    Selector::parse(selector).expect("Static selector is valid")
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    // --- HTML builders ---
    //
    // The six frozen fixtures are all *valid* pages, so none of them reaches
    // a failure path. These snippets carry only the markup a given function
    // reads, which is also what makes the assertions readable.

    fn h1(title: &str) -> String {
        format!("<h1>{title}</h1>")
    }

    fn canonical(slug: &str) -> String {
        format!(
            r#"<link rel="canonical" href="https://www.ulaval.ca/etudes/programmes/{slug}">"#
        )
    }

    fn promo(credits: &str) -> String {
        format!(
            r#"<div class="bloc-promo"><span class="promo-entete--titre">{credits}</span><span class="promo-entete--contenu">Crédits</span></div>"#
        )
    }

    fn structure(groups: &str) -> String {
        format!(r#"<section id="section-structure">{groups}</section>"#)
    }

    fn group(heading: Option<&str>, blocks: &str) -> String {
        let heading =
            heading.map(|h| format!("<h3>{h}</h3>")).unwrap_or_default();
        format!(r#"<div class="fe-bloc-section">{heading}{blocks}</div>"#)
    }

    fn block(title: &str, credits: Option<&str>, body: &str) -> String {
        let credits = credits
            .map(|c| {
                format!(r#"<span class="fe-bloc-titre--credits">{c}</span>"#)
            })
            .unwrap_or_default();
        format!(
            r#"<div class="collapsible-sections"><div class="controls-title fe-bloc-titre"><h4 class="fe-bloc-titre--texte sep">{title}</h4>{credits}</div>{body}</div>"#
        )
    }

    fn accordion(heading: &str, body: &str) -> String {
        format!(
            r#"<div class="toggle-section"><p class="toggle-section--header"><span class="item">{heading}</span></p><div class="toggle-section--content"><div class="fe-bloc-regle--paragraphe">{body}</div></div></div>"#
        )
    }

    fn cards(codes: &[&str]) -> String {
        let items: String = codes
            .iter()
            .map(|code| {
                format!(
                    r#"<li><span class="cours-carte--sigle">{code}</span></li>"#
                )
            })
            .collect();
        format!(r#"<ul class="fe--liste-cours">{items}</ul>"#)
    }

    fn line(text: &str) -> String {
        format!(r#"<p class="fe-bloc-regle--ligne">{text}</p>"#)
    }

    fn note(text: &str) -> String {
        format!(r#"<div class="fe-bloc-section--paragraphe">{text}</div>"#)
    }

    // a complete, minimal page: every `?` in `parse` lets it through
    fn page(groups: &str) -> String {
        format!(
            "<html><body>{}{}{}{}</body></html>",
            h1("Baccalauréat en génie des eaux"),
            canonical("baccalaureat-en-genie-des-eaux"),
            promo("120"),
            structure(groups),
        )
    }

    fn parsed(groups: &str) -> ProgramPage {
        parse(&page(groups)).unwrap_or_else(|e| panic!("valid page: {e}"))
    }

    fn malformed_entry(error: &ParseError) -> (&str, &str) {
        match error {
            ParseError::MalformedEntry { selector, raw } => {
                (selector.as_str(), raw.as_str())
            }
            other => panic!("expected MalformedEntry, got {other:?}"),
        }
    }

    fn no_anomalies() -> Vec<ParseError> {
        Vec::new()
    }

    // --- Whole-page assembly ---

    #[test]
    fn a_page_missing_a_field_fails_rather_than_yielding_a_partial_program() {
        // Each body holds every field the previous one had plus the one it
        // was missing, so the `?` that rejects it is a different one each
        // time. A hole in a Program would silently reach the solver.
        let title = h1("Baccalauréat en génie des eaux");
        let canonical = canonical("baccalaureat-en-genie-des-eaux");
        let credits = promo("120");

        for (missing, body) in [
            ("title", String::new()),
            ("cycle", h1("Doctorat en génie des eaux")),
            ("code", title.clone()),
            ("credits", format!("{title}{canonical}")),
            (
                "credits value",
                format!("{title}{canonical}{}", promo("cent vingt")),
            ),
            ("structure", format!("{title}{canonical}{credits}")),
        ] {
            let html = format!("<html><body>{body}</body></html>");
            assert!(
                parse(&html).is_err(),
                "a page missing {missing} was accepted"
            );
        }
    }

    #[test]
    fn a_complete_page_without_any_group_is_a_program_without_structure() {
        // the counterpart of the table above: every field present, so the
        // same `?`s let the page through
        let page = parsed("");

        assert_eq!(page.program.code, "baccalaureat-en-genie-des-eaux");
        assert_eq!(page.program.title, "Baccalauréat en génie des eaux");
        assert_eq!(page.program.cycle, Cycle::First);
        assert_eq!(page.program.credits_required, 120);
        assert!(page.program.rules.is_empty());
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
    }

    #[test]
    fn the_title_is_rebuilt_word_by_word_across_the_source_line_breaks() {
        let html = format!(
            "<html><body>{}{}{}{}</body></html>",
            "<h1>Baccalauréat en \n\t\t\tgénie des eaux</h1>",
            canonical("baccalaureat-en-genie-des-eaux"),
            promo("120"),
            structure(""),
        );

        let page = parse(&html).expect("valid page");
        assert_eq!(page.program.title, "Baccalauréat en génie des eaux");
    }

    // --- Simple fields ---

    #[test]
    fn every_known_diploma_names_its_cycle() {
        for (title, expected) in [
            ("Baccalauréat en génie civil", Cycle::First),
            ("Certificat en informatique", Cycle::First),
            ("Maîtrise en génie des eaux - avec mémoire", Cycle::Second),
            ("DESS en génie des eaux", Cycle::Second),
        ] {
            assert_eq!(
                parse_cycle(title).unwrap_or_else(|e| panic!("{title}: {e}")),
                expected,
                "for {title:?}"
            );
        }
    }

    #[test]
    fn a_diploma_outside_the_two_cycles_is_a_malformed_entry() {
        // a doctorate is a third cycle: `Cycle` cannot hold it, and filing
        // it under the nearest neighbour would be a lie
        for title in ["Doctorat en génie des eaux", ""] {
            let error = parse_cycle(title).expect_err("outside the cycles");
            assert_eq!(malformed_entry(&error).0, "cycle", "for {title:?}");
        }
    }

    #[test]
    fn a_canonical_link_without_a_usable_segment_is_a_malformed_entry() {
        let html = format!(
            "<html><body>{}{}</body></html>",
            h1("Baccalauréat en génie des eaux"),
            r#"<link rel="canonical" href="///">"#,
        );

        let error = parse(&html).expect_err("no slug");
        assert_eq!(malformed_entry(&error).0, CANONICAL_CSS);
    }

    #[test]
    fn a_credits_card_without_a_value_is_a_missing_element() {
        let html = format!(
            "<html><body>{}{}{}</body></html>",
            h1("Baccalauréat en génie des eaux"),
            canonical("baccalaureat-en-genie-des-eaux"),
            r#"<div class="bloc-promo"><span class="promo-entete--contenu">Crédits</span></div>"#,
        );

        assert!(matches!(
            parse(&html),
            Err(ParseError::MissingElement { selector }) if selector == PROMO_VALUE_CSS
        ));
    }

    // --- Groups: which role a block plays ---

    #[test]
    fn a_labelled_group_names_the_role_of_its_blocks() {
        let rule =
            accordion("Règle 1 – 3 crédits parmi :", &cards(&["GEX-1000"]));
        let page = parsed(&format!(
            "{}{}{}",
            group(
                None,
                &block("Génie des eaux", Some("102 crédits exigés"), &rule)
            ),
            group(
                Some(CONCENTRATIONS_HEADING),
                &block(
                    "Eau et environnement",
                    Some("15 crédits exigés"),
                    &rule
                )
            ),
            group(
                Some(PROFILES_HEADING),
                &block("Profil international", None, &rule)
            ),
        ));

        assert_eq!(page.program.rules.len(), 1);
        assert_eq!(
            page.program.concentrations[0].title,
            "Eau et environnement"
        );
        assert_eq!(page.program.concentrations[0].credits_required, Some(15));
        assert_eq!(page.program.profiles[0].title, "Profil international");
        assert_eq!(page.program.profiles[0].credits_required, None);
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
    }

    #[test]
    fn a_later_unlabelled_group_of_one_block_prefixes_its_rule_titles() {
        // « Autres exigences » (génie civil, physique, industriel): its
        // « Règle 1 » would collide with the first block's
        let rule =
            accordion("Règle 1 – 3 crédits parmi :", &cards(&["ANL-2020"]));
        let page = parsed(&format!(
            "{}{}",
            group(
                None,
                &block("Génie civil", Some("99 crédits exigés"), &rule)
            ),
            group(
                None,
                &block("Autres exigences", Some("6 crédits exigés"), &rule)
            ),
        ));

        assert_eq!(
            page.program
                .rules
                .iter()
                .map(|rule| rule.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Règle 1", "Autres exigences – Règle 1"]
        );
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
    }

    #[test]
    fn a_later_unlabelled_group_of_several_blocks_is_read_as_concentrations() {
        // the bac en génie mécanique omits the « Concentrations » heading;
        // the assumption that several blocks means concentrations is
        // reported so a page breaking it says so
        let rule =
            accordion("Règle 1 – 12 crédits parmi :", &cards(&["GMC-3351"]));
        let page = parsed(&format!(
            "{}{}",
            group(
                None,
                &block("Génie mécanique", Some("102 crédits exigés"), &rule)
            ),
            group(
                None,
                &format!(
                    "{}{}",
                    block("Robotique", Some("18 crédits exigés"), &rule),
                    block(
                        "Génie du bâtiment durable",
                        Some("18 crédits exigés"),
                        &rule
                    ),
                )
            ),
        ));

        assert_eq!(page.program.concentrations.len(), 2);
        assert_eq!(page.program.rules.len(), 1, "only the first block's rule");
        assert_eq!(malformed_entry(&page.anomalies[0]).0, GROUP_CSS);
    }

    #[test]
    fn an_unknown_group_heading_is_an_anomaly_and_reads_as_program_blocks() {
        let rule =
            accordion("Règle 1 – 3 crédits parmi :", &cards(&["GEX-1000"]));
        let page = parsed(&group(
            Some("Passerelles"),
            &block("Passerelle", None, &rule),
        ));

        assert_eq!(page.program.rules.len(), 1);
        assert_eq!(malformed_entry(&page.anomalies[0]), ("h3", "Passerelles"));
    }

    // --- Blocks ---

    #[test]
    fn a_block_without_a_title_is_dropped_and_surfaced() {
        let page = parsed(&group(
            None,
            r#"<div class="collapsible-sections"><div class="controls-title fe-bloc-titre"></div></div>"#,
        ));

        assert!(page.program.rules.is_empty());
        assert!(matches!(
            &page.anomalies[0],
            ParseError::MissingElement { selector } if selector == BLOCK_TITLE_CSS
        ));
    }

    #[test]
    fn a_block_whose_credits_figure_is_unreadable_is_an_anomaly() {
        let page = parsed(&group(
            Some(CONCENTRATIONS_HEADING),
            &block("Robotique", Some("quinze crédits exigés"), ""),
        ));

        assert_eq!(page.program.concentrations[0].credits_required, None);
        assert_eq!(
            malformed_entry(&page.anomalies[0]),
            (
                format!("{BLOCK_CREDITS_CSS} (Robotique)").as_str(),
                "quinze crédits exigés"
            )
        );
    }

    #[test]
    fn cours_obligatoires_belong_to_the_block_that_holds_them() {
        // the maîtrise adds a « Recherche » block whose obligatory courses
        // join the programme's; a concentration keeps its own
        let page = parsed(&format!(
            "{}{}",
            group(
                None,
                &format!(
                    "{}{}",
                    block(
                        "Génie des eaux",
                        Some("15 crédits exigés"),
                        &accordion(MANDATORY_HEADING, &cards(&["GCI-7077"]))
                    ),
                    block(
                        "Recherche",
                        None,
                        &accordion(MANDATORY_HEADING, &cards(&["GEX-6811"]))
                    ),
                )
            ),
            group(
                Some(CONCENTRATIONS_HEADING),
                &block(
                    "Robotique",
                    Some("18 crédits exigés"),
                    &accordion(MANDATORY_HEADING, &cards(&["GMC-3351"]))
                )
            ),
        ));

        assert_eq!(page.program.mandatory, vec!["GCI-7077", "GEX-6811"]);
        assert_eq!(page.program.concentrations[0].mandatory, vec!["GMC-3351"]);
        assert!(page.program.rules.is_empty(), "no rule is invented");
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
    }

    #[test]
    fn the_prose_of_a_block_becomes_its_notes() {
        let page = parsed(&group(
            None,
            &block(
                "Génie mécanique",
                Some("102 crédits exigés"),
                &format!(
                    "{}{}",
                    accordion(MANDATORY_HEADING, &cards(&["GMC-1000"])),
                    note(
                        "Réussir le stage GMC-2580 pour obtenir son diplôme."
                    ),
                ),
            ),
        ));

        assert_eq!(
            page.program.notes,
            vec!["Réussir le stage GMC-2580 pour obtenir son diplôme."]
        );
    }

    // --- Rule headings ---

    #[test]
    fn every_constraint_wording_is_recognized() {
        for (text, expected) in [
            ("Un cours parmi :", Constraint::Count { count: 1 }),
            ("3 crédits parmi :", Constraint::Credits { min: 3, max: 3 }),
            (
                "3 à 9 crédits parmi :",
                Constraint::Credits { min: 3, max: 9 },
            ),
            // génie physique and industriel drop the « parmi : » tail
            ("15 crédits", Constraint::Credits { min: 15, max: 15 }),
            (
                "0 à 12 crédits parmi :",
                Constraint::Credits { min: 0, max: 12 },
            ),
            ("1 crédit", Constraint::Credits { min: 1, max: 1 }),
        ] {
            assert_eq!(parse_constraint(text), Some(expected), "for {text:?}");
        }
    }

    #[test]
    fn a_constraint_naming_no_readable_number_is_not_a_constraint() {
        for text in [
            "Réussir la scolarité de",
            "trois crédits parmi :",
            "trois à neuf crédits parmi :",
            "3 à neuf crédits parmi :",
            "",
        ] {
            assert_eq!(parse_constraint(text), None, "for {text:?}");
        }
    }

    #[test]
    fn a_heading_without_a_dash_keeps_its_whole_text_as_the_rule_name() {
        let mut anomalies = no_anomalies();

        let (name, constraint) = parse_rule_heading("Règle 1", &mut anomalies);

        assert_eq!(name, "Règle 1");
        assert_eq!(constraint, None);
        assert_eq!(malformed_entry(&anomalies[0]).0, ACCORDION_HEADING_CSS);
    }

    #[test]
    fn an_unreadable_constraint_leaves_the_rule_without_one() {
        // « Règle 1 – Réussir la scolarité de » (génie mécanique): the
        // header is cut off mid-sentence and names no number at all
        let page = parsed(&group(
            Some(PROFILES_HEADING),
            &block(
                "Passage intégré au deuxième cycle",
                None,
                &accordion(
                    "Règle 1 – Réussir la scolarité de",
                    &line("deuxième cycle suivante :"),
                ),
            ),
        ));

        let rule = &page.program.profiles[0].rules[0];
        assert_eq!(rule.title, "Règle 1");
        assert_eq!(rule.constraint, None);
        assert_eq!(
            rule.courses,
            RuleCourses::Raw {
                raw: "deuxième cycle suivante :".to_string()
            }
        );
        assert!(
            page.anomalies
                .iter()
                .any(|a| malformed_entry(a).0 == "constraint"),
            "got {:?}",
            page.anomalies
        );
    }

    // --- Rule bodies ---

    #[test]
    fn a_rule_listing_cards_keeps_every_group_and_its_prose_as_notes() {
        // génie des eaux Règle 4: an unlabelled list then six thematic
        // subgroups. The model has no subgroup, so the courses are flattened
        // in document order and the labels ride along as notes.
        let page = parsed(&group(
            None,
            &block(
                "Génie des eaux",
                Some("102 crédits exigés"),
                &accordion(
                    "Règle 4 – 3 crédits parmi :",
                    &format!(
                        "{}{}{}{}{}",
                        cards(&["DDU-2000", "ENT-4020"]),
                        line("Programmation"),
                        cards(&["IFT-4902"]),
                        line("Langue et communication"),
                        cards(&["ANL-2020"]),
                    ),
                ),
            ),
        ));

        let rule = &page.program.rules[0];
        assert_eq!(
            rule.courses,
            RuleCourses::List {
                courses: vec![
                    "DDU-2000".to_string(),
                    "ENT-4020".to_string(),
                    "IFT-4902".to_string(),
                    "ANL-2020".to_string(),
                ]
            }
        );
        assert_eq!(
            rule.notes,
            vec!["Programmation", "Langue et communication"]
        );
        assert!(page.anomalies.is_empty(), "got {:?}", page.anomalies);
    }

    #[test]
    fn a_rule_stated_in_prose_keeps_the_whole_paragraph() {
        // génie civil: the grammar recognizes a prefix of the paragraph,
        // and the sentence that follows it must not disappear
        let raw = "tous les cours de premier cycle, à l'exception des cours correctifs de français. Si le profil développement durable fait partie de votre cheminement, vous devez suivre DDU-1000.";
        let mut anomalies = no_anomalies();

        let courses = classify_prose(raw.to_string(), &[], &mut anomalies);

        assert_eq!(
            courses,
            RuleCourses::Any {
                courses: Keyword::Any,
                raw: raw.to_string()
            }
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_rule_naming_another_rule_resolves_it_by_concentration_title() {
        let concentrations =
            vec!["Cheminement sans concentration".to_string()];
        let raw =
            "tous les cours de la Règle 1 du cheminement sans concentration.";
        let mut anomalies = no_anomalies();

        let courses =
            classify_prose(raw.to_string(), &concentrations, &mut anomalies);

        assert_eq!(
            courses,
            RuleCourses::Reference {
                courses: RuleReference {
                    // the title as the page writes it, not as the sentence does
                    concentration: "Cheminement sans concentration"
                        .to_string(),
                    rule: "Règle 1".to_string(),
                },
                raw: raw.to_string(),
            }
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn prose_the_grammar_does_not_cover_is_kept_raw_and_surfaced() {
        for raw in [
            // names a course, but in prose rather than in a card
            "Réussir le cours ANL-2020 Intermediate English II.",
            // a reference whose target no block on the page carries
            "tous les cours de la Règle 1 du cheminement introuvable.",
            // shaped like a reference, but naming no cheminement at all
            "tous les cours de la Règle 1",
        ] {
            let mut anomalies = no_anomalies();

            let courses = classify_prose(
                raw.to_string(),
                &["Cheminement sans concentration".to_string()],
                &mut anomalies,
            );

            assert_eq!(
                courses,
                RuleCourses::Raw {
                    raw: raw.to_string()
                },
                "for {raw:?}"
            );
            assert_eq!(
                malformed_entry(&anomalies[0]).0,
                "rule",
                "for {raw:?}"
            );
        }
    }

    #[test]
    fn a_rule_with_neither_cards_nor_prose_is_an_anomaly() {
        let page = parsed(&group(
            None,
            &block(
                "Génie des eaux",
                None,
                &accordion("Règle 1 – 3 crédits parmi :", ""),
            ),
        ));

        assert_eq!(
            page.program.rules[0].courses,
            RuleCourses::Raw { raw: String::new() }
        );
        assert!(
            page.anomalies.iter().any(|anomaly| matches!(
                anomaly,
                ParseError::MissingElement { selector }
                    if selector.contains(COURSE_CODE_CSS)
            )),
            "got {:?}",
            page.anomalies
        );
    }

    #[test]
    fn an_accordion_without_a_heading_is_a_rule_with_no_name() {
        // drift guard: the heading is navigated to, never assumed
        let page = parsed(&group(
            None,
            &block(
                "Génie des eaux",
                None,
                r#"<div class="toggle-section"><div class="toggle-section--content"><div class="fe-bloc-regle--paragraphe"><ul class="fe--liste-cours"><li><span class="cours-carte--sigle">GEX-1000</span></li></ul></div></div></div>"#,
            ),
        ));

        assert_eq!(page.program.rules[0].title, "");
        assert_eq!(
            malformed_entry(&page.anomalies[0]).0,
            ACCORDION_HEADING_CSS
        );
    }
}
