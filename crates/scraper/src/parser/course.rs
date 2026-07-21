use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

use crate::parser::ParseError;
use scraper::{ElementRef, Html, Selector};
use ulaval_scheduler_core::{
    Course, Credits, Cycle, Day, Mode, PrereqTree, Prerequisites,
    ProgramCredits, Season, SeasonOffering, Section, Slot, Time,
};

const CODE_CSS: &str = "span.fe--titre-type";
static CODE: LazyLock<Selector> = LazyLock::new(|| sel(CODE_CSS));
const TITLE_CSS: &str = "span.fe--titre-nom";
static TITLE: LazyLock<Selector> = LazyLock::new(|| sel(TITLE_CSS));

// the « faits rapides » list carries both the credits and the cycle cards
static FAITS_RAPIDES: LazyLock<Selector> =
    LazyLock::new(|| sel("ul.fe--faits-rapides > li"));
const CREDITS_LABEL_CSS: &str = "span.promo-entete--contenu";
static CREDITS_LABEL: LazyLock<Selector> =
    LazyLock::new(|| sel(CREDITS_LABEL_CSS));
const CREDITS_VALUE_CSS: &str = "span.promo-entete--titre";
static CREDITS_VALUE: LazyLock<Selector> =
    LazyLock::new(|| sel(CREDITS_VALUE_CSS));
const CYCLE_LABEL_CSS: &str = "p.promo-paragraphe";
static CYCLE_LABEL: LazyLock<Selector> =
    LazyLock::new(|| sel(CYCLE_LABEL_CSS));
static CYCLE_VALUE: LazyLock<Selector> =
    LazyLock::new(|| sel("ul.promo-entete--contenu li strong"));
static PREALABLES: LazyLock<Selector> =
    LazyLock::new(|| sel("div.fe--prealables p.etiquette-container"));

// only a card that links to a course page is a live equivalence; a bare
// `li.bloc-cours` is an expired one (ADR
// `2026-07-extraction-html-de-la-page-cours` §6)
static EQUIVALENT_CARD: LazyLock<Selector> =
    LazyLock::new(|| sel("li.bloc-cours.carte-accessible"));
const EQUIVALENT_CODE_CSS: &str = "a.carte-accessible--lien span.sigle";
static EQUIVALENT_CODE: LazyLock<Selector> =
    LazyLock::new(|| sel(EQUIVALENT_CODE_CSS));

const TOGGLE_SECTION_CSS: &str = "div.toggle-section";
static TOGGLE_SECTION: LazyLock<Selector> =
    LazyLock::new(|| sel(TOGGLE_SECTION_CSS));

static SESSION: LazyLock<Selector> =
    LazyLock::new(|| sel("div.collapsible-sections"));
static SESSION_HEADING: LazyLock<Selector> =
    LazyLock::new(|| sel("div.sections-controls p.controls-title"));

const SECTION_HEADER_CSS: &str = "p.toggle-section--header";
static SECTION_HEADER: LazyLock<Selector> =
    LazyLock::new(|| sel(SECTION_HEADER_CSS));
const NRC_CSS: &str = "strong.section-cours--nrc";
static NRC: LazyLock<Selector> = LazyLock::new(|| {
    sel("strong.section-cours--nrc span.section-cours--nrc-el")
});
static PLAGE: LazyLock<Selector> =
    LazyLock::new(|| sel("ul.section-cours--liste"));

static HEADER_ITEM: LazyLock<Selector> = LazyLock::new(|| {
    sel("span.header--content-details span.item:not(.precision)")
});

static PLAGE_ITEM: LazyLock<Selector> =
    LazyLock::new(|| sel("li.section-cours--etiquette"));
static PLAGE_LABEL: LazyLock<Selector> = LazyLock::new(|| sel("strong"));

// A session nests its sections strictly:
//
//   div.collapsible-sections
//    └ div.toggle-section                            ← top-level section
//       ├ p.toggle-section--header                   ← code, section, mode
//       └ div.toggle-section--content
//          ├ div.toggle-section--content-wrapper       ← own NRC and plages
//          └ div.toggle-section--content-wrapper.dark  ← linked sections
//
// Only the `dark` wrapper holds nested sections, so a section's own header
// and content are subtrees free of foreign sections: a descendant scan
// inside them cannot stray into a linked section.
static SECTION_CONTENT: LazyLock<Selector> =
    LazyLock::new(|| sel("div.toggle-section--content"));
static OWN_WRAPPER: LazyLock<Selector> =
    LazyLock::new(|| sel("div.toggle-section--content-wrapper:not(.dark)"));
static LINKED_WRAPPER: LazyLock<Selector> =
    LazyLock::new(|| sel("div.toggle-section--content-wrapper.dark"));

pub struct CoursePage {
    pub course: Course,
    // the year each retained season was read from: it names the session
    // snapshot (`a2026`) but is not a property of the course, so it rides
    // alongside `Course` instead of inside it
    pub years: BTreeMap<Season, u16>,
    pub anomalies: Vec<ParseError>,
}

enum PrereqToken {
    Open,
    Close,
    And,
    Or,
    // an operand is classified whole by the tokenizer — only `(`, `)`, `ET`
    // and `OU` carry structure, so nothing inside one concerns the parser
    Operand(PrereqTree),
}

struct PrereqFrame {
    completed: Vec<PrereqTree>,
    chain: Vec<PrereqTree>,
}

impl PrereqFrame {
    fn new() -> Self {
        PrereqFrame {
            completed: Vec::new(),
            chain: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
enum Nesting {
    TopLevel,
    Linked,
}

// `None` is a page the parser read perfectly and then dropped on purpose: a
// doctoral or post-doctoral activity is nothing this generator schedules, so
// it yields no course — and no anomaly either, since nothing was lost by
// accident.
pub fn parse(html: &str) -> Result<Option<CoursePage>, ParseError> {
    let doc = Html::parse_document(html);

    let mut anomalies = Vec::new();

    let code = parse_element(&doc, &CODE, CODE_CSS)?;
    let title = parse_element(&doc, &TITLE, TITLE_CSS)?;
    let credits = parse_credits(&doc)?;
    let Some(cycle) = parse_cycle(&doc)? else {
        return Ok(None);
    };
    let prerequisites = parse_prerequisites(&doc, &mut anomalies);
    let equivalents = parse_equivalents(&doc)?;
    let sessions = parse_seasons(&doc, &mut anomalies);

    let years = sessions
        .iter()
        .map(|(season, (year, _))| (*season, *year))
        .collect();
    let seasons = sessions
        .into_iter()
        .map(|(season, (_, offering))| (season, offering))
        .collect();

    Ok(Some(CoursePage {
        course: Course {
            code,
            title,
            credits,
            cycle,
            prerequisites,
            equivalents,
            seasons,
        },
        years,
        anomalies,
    }))
}

fn parse_element(
    doc: &Html,
    selector: &Selector,
    css: &str,
) -> Result<String, ParseError> {
    doc.select(selector)
        .next()
        .map(|element| element.text().collect::<String>().trim().to_string())
        .ok_or_else(|| ParseError::MissingElement {
            selector: css.to_string(),
        })
}

fn parse_credits(doc: &Html) -> Result<Credits, ParseError> {
    // a course can carry no credits card at all — GCI-2510, a « Stage »
    // seminar, lists only its cycle and its modes. It is worth no credits,
    // which is a fact about the course, not markup drift: the page is
    // otherwise complete, so the course is kept rather than dropped.
    let Some(card) = doc.select(&FAITS_RAPIDES).find(|card| {
        card.select(&CREDITS_LABEL).next().is_some_and(|label| {
            label
                .text()
                .collect::<String>()
                .trim()
                .starts_with("Crédit")
        })
    }) else {
        return Ok(Credits::Fixed(0));
    };
    let raw = card
        .select(&CREDITS_VALUE)
        .next()
        .map(|value| value.text().collect::<String>())
        .ok_or_else(|| ParseError::MissingElement {
            selector: CREDITS_VALUE_CSS.to_string(),
        })?;
    let raw = raw.trim();

    credits_of(raw).ok_or_else(|| ParseError::MalformedEntry {
        selector: "credits".to_string(),
        raw: raw.to_string(),
    })
}

// « 3 », or « 6 à 12 » for a stage the student weights himself (MED-1911).
// A span running backwards states a bound no student can satisfy, so it is
// markup drift rather than a fact about the course.
fn credits_of(raw: &str) -> Option<Credits> {
    match raw.split_whitespace().collect::<Vec<_>>().as_slice() {
        [count] => Some(Credits::Fixed(count.parse().ok()?)),
        [min, "à", max] => {
            let (min, max) = (min.parse().ok()?, max.parse().ok()?);
            (min <= max).then_some(Credits::Range { min, max })
        }
        _ => None,
    }
}

// `None` for an activity above the second cycle: `Cycle` cannot hold it, and
// the generator has no business scheduling a thesis milestone or a
// post-doctoral residency. Recognized, then dropped — not an error.
fn parse_cycle(doc: &Html) -> Result<Option<Cycle>, ParseError> {
    let card = doc
        .select(&FAITS_RAPIDES)
        .find(|card| {
            card.select(&CYCLE_LABEL).next().is_some_and(|label| {
                label.text().collect::<String>().trim().starts_with("Cycle")
            })
        })
        .ok_or_else(|| ParseError::MissingElement {
            selector: format!("{} = Cycle", CYCLE_LABEL_CSS),
        })?;

    let level = card
        .select(&CYCLE_VALUE)
        .map(|value| cycle_level(&value.text().collect::<String>()))
        .collect::<Result<Vec<u8>, ParseError>>()?
        .into_iter()
        .min()
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: "cycle values".to_string(),
            raw: card.html(),
        })?;

    // « 2e et 3e cycle » collapses to 2 and stays in scope; only a course
    // whose *lowest* level is above the second falls out of it
    Ok(Cycle::try_from(level).ok())
}

fn cycle_level(text: &str) -> Result<u8, ParseError> {
    match text.trim() {
        "Premier cycle" => Ok(1),
        "Deuxième cycle" => Ok(2),
        "Troisième cycle" => Ok(3),
        // MDD-5101, a post-doctoral dental residency: the page words its
        // level as a programme rather than a cycle, and it sits above the
        // third — in grammar, and out of scope
        "Études post-MDD" => Ok(4),
        other => Err(ParseError::MalformedEntry {
            selector: "cycle".to_string(),
            raw: other.to_string(),
        }),
    }
}

fn parse_prerequisites(
    doc: &Html,
    anomalies: &mut Vec<ParseError>,
) -> Option<Prerequisites> {
    let raw = doc.select(&PREALABLES).next().map(|element| {
        element.text().collect::<String>().trim().to_string()
    })?;

    match parse_prereq_tree(&raw) {
        Ok(tree) => Some(Prerequisites::Parsed { raw, tree }),
        // only the structure of the expression can fail: an operand nobody
        // can check is kept in place as text (ADR
        // `2026-07-operande-non-verifiable-gardee-en-texte`)
        Err(error) => {
            anomalies.push(error);
            Some(Prerequisites::Raw { raw })
        }
    }
}

// Only a broken structure — an unclosed group, an operator missing an
// operand — can fail: it has no local repair, nothing says which operands
// the group was meant to hold. Everything else ends up in the tree.
fn parse_prereq_tree(raw: &str) -> Result<PrereqTree, ParseError> {
    let malformed = |error: &str| malformed_prereq(error, raw);

    let tokens = tokenize_prereq_raw(raw);

    let mut current = PrereqFrame::new();
    let mut enclosing: Vec<PrereqFrame> = Vec::new();
    let mut expecting_operand = true;

    for token in tokens {
        match token {
            PrereqToken::Operand(tree) => {
                if !expecting_operand {
                    return Err(malformed("two operands in a row"));
                }
                current.chain.push(tree);
                expecting_operand = false;
            }
            PrereqToken::Open => {
                if !expecting_operand {
                    return Err(malformed("( where an operator was expected"));
                }
                let parent =
                    std::mem::replace(&mut current, PrereqFrame::new());
                enclosing.push(parent);
            }
            PrereqToken::Close => {
                if expecting_operand {
                    return Err(malformed(") without a left operand"));
                }
                expecting_operand = false;
                let parent =
                    enclosing.pop().ok_or_else(|| malformed("unmatched )"))?;
                let finished = std::mem::replace(&mut current, parent);
                // the guard above rejects a group with no operand, so the
                // frame being closed always folds into a tree
                let tree = fold_frame(finished)
                    .expect("a closed group holds at least one operand");
                current.chain.push(tree);
            }
            PrereqToken::And => {
                if expecting_operand {
                    return Err(malformed("ET without a left operand"));
                }
                expecting_operand = true;
            }
            PrereqToken::Or => {
                if expecting_operand {
                    return Err(malformed("OU without a left operand"));
                }
                let chain = std::mem::take(&mut current.chain);
                current.completed.extend(fold_chain(chain));
                expecting_operand = true;
            }
        }
    }

    if expecting_operand {
        return Err(malformed("expression ends on an operator"));
    }
    if !enclosing.is_empty() {
        return Err(malformed("unclosed ("));
    }

    Ok(
        fold_frame(current)
            .expect("the expression holds at least one operand"),
    )
}

// Only `(`, `)`, `ET` and `OU` carry structure; everything between two of
// them is one operand, read whole rather than word by word. The parenthesis
// is padded first because the source glues it to the sigle, and the `*` some
// sigles carry means nothing to the grammar.
fn tokenize_prereq_raw(raw: &str) -> Vec<PrereqToken> {
    let padded = raw.replace('(', " ( ").replace(')', " ) ").replace("*", "");
    let mut tokens: Vec<PrereqToken> = Vec::new();
    let mut operand: Vec<&str> = Vec::new();

    for word in padded.split_whitespace() {
        let separator = match word {
            "(" => PrereqToken::Open,
            ")" => PrereqToken::Close,
            "ET" => PrereqToken::And,
            "OU" => PrereqToken::Or,
            _ => {
                operand.push(word);
                continue;
            }
        };
        flush_operand(&mut operand, &mut tokens);
        tokens.push(separator);
    }
    flush_operand(&mut operand, &mut tokens);

    tokens
}

// Two separators in a row enclose no operand at all — « A ET OU B » — and
// nothing is emitted: the parser is the one that knows an operator needs
// operands on both sides, and reports it.
fn flush_operand(operand: &mut Vec<&str>, tokens: &mut Vec<PrereqToken>) {
    if operand.is_empty() {
        return;
    }
    let tree = classify_operand(operand);
    operand.clear();
    tokens.push(PrereqToken::Operand(tree));
}

// An operand the planner cannot check is kept as text: an examination
// (« Examen Test français … », FRN-1904), a range of courses leaving the
// choice to the student (« ESG-2020 à 3799 », ESP-1000), a sigle the source
// mistyped (« FRN 19543 », FRN-1112), de la prose. None of these is
// recognized one by one — they are simply what is left when no checkable
// shape fits (ADR `2026-07-operande-non-verifiable-gardee-en-texte`).
fn classify_operand(words: &[&str]) -> PrereqTree {
    checkable_operand(words).unwrap_or_else(|| PrereqTree::Raw {
        raw: words.join(" "),
    })
}

// The shapes the planner can act on, and only those.
fn checkable_operand(words: &[&str]) -> Option<PrereqTree> {
    match words {
        // a bound on the courses the credits are counted from — « ACT-1000 à
        // 4999, Crédits exigés : 39 » (ACT-4114) or « 1000 à 4999 Crédits
        // exigés : 15 » (GMC-1590). It drops out: the cycle it names is the
        // cycle of the course carrying the requirement, which the snapshot
        // already records (ADR `2026-07-bornes-de-credits-toutes-retirees`)
        [lower, "à", upper, "Crédits", "exigés", ":", count] => {
            match (bound_lower(lower), bound_upper(upper)) {
                (Some(""), Some("")) => program_credits(None, count),
                (Some(subject), Some(",")) => {
                    program_credits(Some(program_code(subject)?), count)
                }
                _ => None,
            }
        }
        [subject, "Crédits", "exigés", ":", count]
            if subject.ends_with(',') =>
        {
            program_credits(Some(program_code(subject)?), count)
        }
        // « Crédits exigés : N » with no programme named: the requirement
        // bears on the student's own (GEX-3333)
        ["Crédits", "exigés", ":", count] => program_credits(None, count),
        [code] if is_course_code(code) => {
            Some(PrereqTree::Course(code.to_string()))
        }
        _ => None,
    }
}

fn program_credits(program: Option<&str>, count: &str) -> Option<PrereqTree> {
    Some(PrereqTree::ProgramCredits {
        program_credits: ProgramCredits {
            program: program.map(str::to_string),
            credits: count.trim().parse::<u32>().ok()?,
        },
    })
}

// « GEX, » → « GEX »: a matière is three uppercase letters, and the comma
// the source puts before « Crédits exigés » is not part of it
fn program_code(word: &str) -> Option<&str> {
    let code = word.strip_suffix(',').unwrap_or(word);
    is_program_code(code).then_some(code)
}

// « PHI-6000 » → « PHI », « 1000 » → « », anything else is not a bound
fn bound_lower(word: &str) -> Option<&str> {
    let subject = word.trim_end_matches(|c: char| c.is_ascii_digit());
    (word.len() - subject.len() == 4).then(|| subject.trim_end_matches('-'))
}

// « 8899, » → « , », « 4999 » → « », anything else is not a bound
fn bound_upper(word: &str) -> Option<&str> {
    let punctuation = word.trim_start_matches(|c: char| c.is_ascii_digit());
    (word.len() - punctuation.len() == 4).then_some(punctuation)
}

fn malformed_prereq(error: &str, raw: &str) -> ParseError {
    ParseError::MalformedPrerequisites {
        error: error.to_string(),
        raw: raw.to_string(),
    }
}

fn is_program_code(word: &str) -> bool {
    word.len() == 3 && word.chars().all(|c| c.is_ascii_uppercase())
}

fn is_course_code(word: &str) -> bool {
    word.split_once('-').is_some_and(|(prefix, number)| {
        is_program_code(prefix)
            && number.len() == 4
            && number.chars().all(|c| c.is_ascii_digit())
    })
}

fn fold_frame(frame: PrereqFrame) -> Option<PrereqTree> {
    let PrereqFrame {
        mut completed,
        chain,
    } = frame;
    completed.extend(fold_chain(chain));
    if completed.len() > 1 {
        Some(PrereqTree::Any { any: completed })
    } else {
        completed.pop()
    }
}

fn fold_chain(mut chain: Vec<PrereqTree>) -> Option<PrereqTree> {
    if chain.len() > 1 {
        Some(PrereqTree::All { all: chain })
    } else {
        chain.pop()
    }
}

fn parse_equivalents(doc: &Html) -> Result<Vec<String>, ParseError> {
    doc.select(&EQUIVALENT_CARD)
        .map(|card| {
            card.select(&EQUIVALENT_CODE)
                .next()
                .map(|element| {
                    element.text().collect::<String>().trim().to_string()
                })
                .filter(|code| is_course_code(code))
                .ok_or_else(|| ParseError::MalformedEntry {
                    selector: EQUIVALENT_CODE_CSS.to_string(),
                    raw: card.html(),
                })
        })
        .collect()
}

fn parse_seasons(
    doc: &Html,
    anomalies: &mut Vec<ParseError>,
) -> BTreeMap<Season, (u16, SeasonOffering)> {
    let mut latest: BTreeMap<Season, (u16, SeasonOffering)> = BTreeMap::new();

    for session in doc.select(&SESSION) {
        let Some(heading) = session.select(&SESSION_HEADING).next() else {
            continue;
        };
        let heading = heading.text().collect::<String>();

        let (season, year) = match parse_session_heading(&heading) {
            Ok(parsed) => parsed,
            Err(error) => {
                anomalies.push(error);
                continue;
            }
        };
        if latest.get(&season).is_some_and(|(known, _)| *known >= year) {
            continue;
        }

        let offering = parse_offering(session, &heading, anomalies);
        if !offering.options.is_empty() {
            latest.insert(season, (year, offering));
        }
    }

    latest
}

fn parse_session_heading(heading: &str) -> Result<(Season, u16), ParseError> {
    let malformed = || ParseError::MalformedEntry {
        selector: "p.controls-title".to_string(),
        raw: heading.to_string(),
    };

    let mut words = heading.split_whitespace();
    let season = match words.next() {
        Some("Automne") => Season::Fall,
        Some("Hiver") => Season::Winter,
        Some("Été") => Season::Summer,
        _ => return Err(malformed()),
    };
    let year = words
        .next()
        .and_then(|word| word.parse::<u16>().ok())
        .ok_or_else(malformed)?;

    Ok((season, year))
}

fn parse_offering(
    session: ElementRef,
    heading: &str,
    anomalies: &mut Vec<ParseError>,
) -> SeasonOffering {
    let top = top_level_sections(session);

    if advertised_section_count(heading) != Some(top.len()) {
        anomalies.push(ParseError::MalformedEntry {
            selector: "p.controls-title".to_string(),
            raw: format!("{heading}: {} top-level sections found", top.len()),
        });
    }

    let options = top
        .into_iter()
        .flat_map(|section| enrolment_options(section, anomalies))
        .collect();

    SeasonOffering { options }
}

// A stray tag can re-parent a section out of the session's direct children —
// DRT-7104 writes `<b>…<b>` where it means `</b>`, and HTML5 rebuilds the
// unclosed elements around everything that follows. Only a `.dark` wrapper
// makes a section belong to another, so every other descendant is top-level
// whatever depth it ended up at (ADR
// `2026-07-sections-de-premier-niveau-par-ascendance`).
fn top_level_sections(session: ElementRef) -> Vec<ElementRef> {
    let linked: HashSet<_> = session
        .select(&LINKED_WRAPPER)
        .flat_map(|dark| dark.select(&TOGGLE_SECTION))
        .map(|section| section.id())
        .collect();

    session
        .select(&TOGGLE_SECTION)
        .filter(|section| !linked.contains(&section.id()))
        .collect()
}

// One entry per way of enrolling: a section offering a choice of labs
// appears once per lab and carries the lecture along, while a section with
// no lab of its own stands alone. The flat model this replaces — one group
// of lectures, one of labs — paired every lecture with every lab, which
// invents enrolments the page never offered (IFT-1004; ADR
// `2026-07-sections-en-combinaisons-valides`).
fn enrolment_options(
    section: ElementRef,
    anomalies: &mut Vec<ParseError>,
) -> Vec<Vec<Section>> {
    let parsed = match parse_section(section, Nesting::TopLevel) {
        Ok(parsed) => parsed,
        Err(error) => {
            anomalies.push(error);
            return Vec::new();
        }
    };

    let offered = linked_sections(section);
    let ties_a_lab = !offered.is_empty();
    let linked = collect_sections(offered, Nesting::Linked, anomalies);

    if linked.is_empty() {
        // A section the page ties to a lab cannot be taken without one, so a
        // section whose labs are *all* unreadable yields no option at all:
        // handing back the lecture alone would invent an enrolment nobody
        // offers, and the anomaly above already says what was lost.
        return if ties_a_lab {
            Vec::new()
        } else {
            vec![vec![parsed]]
        };
    }

    linked
        .into_iter()
        .map(|linked| vec![parsed.clone(), linked])
        .collect()
}

fn collect_sections(
    group: Vec<ElementRef>,
    nesting: Nesting,
    anomalies: &mut Vec<ParseError>,
) -> Vec<Section> {
    group
        .into_iter()
        .filter_map(|section| match parse_section(section, nesting) {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                anomalies.push(error);
                None
            }
        })
        .collect()
}

fn advertised_section_count(heading: &str) -> Option<usize> {
    heading
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|pair| pair[1].starts_with("section"))
        .and_then(|pair| pair[0].parse::<usize>().ok())
}

fn parse_section(
    section: ElementRef,
    nesting: Nesting,
) -> Result<Section, ParseError> {
    let header = child(section, &SECTION_HEADER).ok_or_else(|| {
        ParseError::MissingElement {
            selector: SECTION_HEADER_CSS.to_string(),
        }
    })?;
    let content = own_content(section);

    // The NRC sits in the header of a linked section and in the content of a
    // top-level one.
    let nrc = section_nrc(header)
        .or_else(|| content.and_then(section_nrc))
        .ok_or_else(|| ParseError::MissingElement {
            selector: NRC_CSS.to_string(),
        })?;

    let (identifier, mode) = parse_section_header(header, nesting)?;

    let slots = content
        .into_iter()
        .flat_map(|content| content.select(&PLAGE))
        .filter_map(|plage| parse_slot(plage).transpose())
        .collect::<Result<Vec<Slot>, ParseError>>()?;

    Ok(Section {
        nrc,
        section: identifier,
        mode,
        slots,
    })
}

fn section_nrc(part: ElementRef) -> Option<String> {
    part.select(&NRC)
        .last()
        .map(|element| element.text().collect::<String>().trim().to_string())
}

fn parse_section_header(
    header: ElementRef,
    nesting: Nesting,
) -> Result<(Option<String>, Mode), ParseError> {
    let items: Vec<String> = header
        .select(&HEADER_ITEM)
        .map(|element| element.text().collect::<String>().trim().to_string())
        .collect();

    // `[code, section, mode]` at the top level, `[section, mode]` for a
    // linked section — the mode is read here, never from the per-plage
    // « Type: ».
    let (identifier, mode) = match (nesting, items.as_slice()) {
        (Nesting::TopLevel, [_, section, mode])
        | (Nesting::Linked, [section, mode]) => (section, mode),
        _ => {
            return Err(ParseError::MalformedEntry {
                selector: "span.header--content-details".to_string(),
                raw: items.join(" | "),
            });
        }
    };

    Ok((
        Some(identifier.clone()).filter(|s| !s.is_empty()),
        parse_mode(mode)?,
    ))
}

fn parse_mode(label: &str) -> Result<Mode, ParseError> {
    match label {
        "En classe" => Ok(Mode::InPerson),
        "À distance" => Ok(Mode::Remote),
        // its « Sur Internet » plage carries no Journée/Horaire, so
        // `parse_slot` already drops it and only the in-class meetings
        // become slots (GEX-3100). GMC-7000 spells the same arrangement
        // « À distance-hybride ». « Comodal » (assister en classe ou à
        // distance, au choix) offre les mêmes plages : même traitement.
        "Hybride" | "À distance-hybride" | "Comodal" => Ok(Mode::Hybrid),
        other => Err(ParseError::MalformedEntry {
            selector: "mode".to_string(),
            raw: other.to_string(),
        }),
    }
}

fn parse_slot(plage: ElementRef) -> Result<Option<Slot>, ParseError> {
    if plage_field(plage, "Dates:").is_none() {
        return Ok(None);
    }
    let Some(day) = plage_field(plage, "Journée:") else {
        return Ok(None);
    };
    let Some(schedule) = plage_field(plage, "Horaire:") else {
        return Ok(None);
    };

    let day = parse_day(&day)?;
    let (start, end) = parse_schedule(&schedule)?;

    Ok(Some(Slot { day, start, end }))
}

fn parse_day(label: &str) -> Result<Day, ParseError> {
    match label {
        "Lundi" => Ok(Day::Monday),
        "Mardi" => Ok(Day::Tuesday),
        "Mercredi" => Ok(Day::Wednesday),
        "Jeudi" => Ok(Day::Thursday),
        "Vendredi" => Ok(Day::Friday),
        "Samedi" => Ok(Day::Saturday),
        "Dimanche" => Ok(Day::Sunday),
        other => Err(ParseError::MalformedEntry {
            selector: "day".to_string(),
            raw: other.to_string(),
        }),
    }
}

fn parse_schedule(raw: &str) -> Result<(Time, Time), ParseError> {
    let (start, end) = raw
        .trim()
        .strip_prefix("De ")
        .and_then(|rest| rest.split_once(" à "))
        .ok_or_else(|| ParseError::MalformedEntry {
            selector: "schedule".to_string(),
            raw: raw.to_string(),
        })?;

    Ok((parse_time(start)?, parse_time(end)?))
}

fn parse_time(raw: &str) -> Result<Time, ParseError> {
    let malformed = || ParseError::MalformedEntry {
        selector: "time".to_string(),
        raw: raw.to_string(),
    };

    let (hour, minute) = raw.trim().split_once('h').ok_or_else(malformed)?;
    let minute = if minute.trim().is_empty() {
        "0"
    } else {
        minute.trim()
    };
    let hour = hour.trim().parse::<u8>().map_err(|_| malformed())?;
    let minute = minute.parse::<u8>().map_err(|_| malformed())?;

    Time::try_from(format!("{hour:02}:{minute:02}")).map_err(|error| {
        ParseError::MalformedEntry {
            selector: "time".to_string(),
            raw: error,
        }
    })
}

fn plage_field(plage: ElementRef, label: &str) -> Option<String> {
    plage.select(&PLAGE_ITEM).find_map(|item| {
        let found = item.select(&PLAGE_LABEL).next()?;
        let found = found.text().collect::<String>();
        if found.trim() != label {
            return None;
        }
        let text = item.text().collect::<String>();
        Some(
            text.trim_start()
                .strip_prefix(found.trim())?
                .trim()
                .to_string(),
        )
    })
}

fn own_content(section: ElementRef) -> Option<ElementRef> {
    let content = child(section, &SECTION_CONTENT)?;
    child(content, &OWN_WRAPPER)
}

fn linked_sections(section: ElementRef) -> Vec<ElementRef> {
    child(section, &SECTION_CONTENT)
        .and_then(|content| child(content, &LINKED_WRAPPER))
        .map(|dark| dark.select(&TOGGLE_SECTION).collect())
        .unwrap_or_default()
}

fn child<'a>(
    parent: ElementRef<'a>,
    selector: &Selector,
) -> Option<ElementRef<'a>> {
    parent
        .children()
        .filter_map(ElementRef::wrap)
        .find(|element| selector.matches(element))
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
    // The frozen fixtures are all *valid* pages, so none of them reaches a
    // failure path. These snippets carry only the markup a given function
    // reads, which is also what makes the assertions readable.

    fn document(body: &str) -> Html {
        Html::parse_document(&format!("<html><body>{body}</body></html>"))
    }

    fn element<'a>(doc: &'a Html, selector: &str) -> ElementRef<'a> {
        doc.select(&Selector::parse(selector).expect("test selector"))
            .next()
            .expect("test element")
    }

    fn fait_rapide(body: &str) -> String {
        format!(r#"<ul class="fe--faits-rapides"><li>{body}</li></ul>"#)
    }

    fn cycle_card(levels: &[&str]) -> String {
        let items: String = levels
            .iter()
            .map(|level| format!("<li><strong>{level}</strong></li>"))
            .collect();
        fait_rapide(&format!(
            r#"<p class="promo-paragraphe">Cycle du cours</p>
               <ul class="promo-entete--contenu">{items}</ul>"#
        ))
    }

    fn plage(fields: &[(&str, &str)]) -> String {
        let items: String = fields
            .iter()
            .map(|(label, value)| {
                format!(
                    r#"<li class="section-cours--etiquette"><strong>{label}</strong> {value}</li>"#
                )
            })
            .collect();
        format!(r#"<ul class="section-cours--liste">{items}</ul>"#)
    }

    fn slot_of(fields: &[(&str, &str)]) -> Result<Option<Slot>, ParseError> {
        let doc = document(&plage(fields));
        parse_slot(element(&doc, "ul.section-cours--liste"))
    }

    fn nrc_block(nrc: &str) -> String {
        format!(
            r#"<strong class="section-cours--nrc"><span class="section-cours--nrc-el">NRC</span><span class="section-cours--nrc-el">{nrc}</span></strong>"#
        )
    }

    fn header(items: &[&str]) -> String {
        let items: String = items
            .iter()
            .map(|item| format!(r#"<span class="item">{item}</span>"#))
            .collect();
        format!(
            r#"<button class="header-wrapper"><span class="header--content-details">{items}</span></button>"#
        )
    }

    // `extra` holds the nested `.dark` wrapper when the section has linked
    // sections of its own.
    // Mirrors the real nesting: header, own content wrapper, then the `dark`
    // wrapper holding linked sections.
    fn toggle_section(items: &[&str], body: &str, linked: &str) -> String {
        let dark = if linked.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="toggle-section--content-wrapper dark">{linked}</div>"#
            )
        };
        format!(
            r#"<div class="toggle-section"><p class="toggle-section--header">{}</p><div class="toggle-section--content"><div class="toggle-section--content-wrapper">{body}</div>{dark}</div></div>"#,
            header(items)
        )
    }

    fn session(heading: &str, sections: &str) -> String {
        format!(
            r#"<div class="collapsible-sections"><div class="sections-controls"><p class="controls-title">{heading}</p></div>{sections}</div>"#
        )
    }

    fn malformed_entry(error: &ParseError) -> (&str, &str) {
        match error {
            ParseError::MalformedEntry { selector, raw } => {
                (selector.as_str(), raw.as_str())
            }
            other => panic!("expected MalformedEntry, got {other:?}"),
        }
    }

    fn credits_card(value: &str) -> String {
        fait_rapide(&format!(
            r#"<span class="promo-entete--titre">{value}</span><span class="promo-entete--contenu">Crédits</span>"#
        ))
    }

    // --- Whole-page assembly ---

    #[test]
    fn a_page_missing_a_field_fails_rather_than_yielding_a_partial_course() {
        // Each body holds every field the previous one had plus the one it
        // was missing, so the `?` that rejects it is a different one each
        // time. A hole in a Course would silently reach the solver; an Err
        // stops the run on that course and is logged.
        let code = r#"<span class="fe--titre-type">GEX-4008</span>"#;
        let title = r#"<span class="fe--titre-nom">Approvisionnement</span>"#;
        let credits = credits_card("3");
        let cycle = cycle_card(&["Premier cycle"]);
        let unreadable_equivalent = r#"<li class="bloc-cours carte-accessible"><a class="carte-accessible--lien"><span class="sigle">GEX-99</span></a></li>"#;

        for (missing, body) in [
            ("code", String::new()),
            ("title", code.to_string()),
            // credits are the one field whose absence is a fact rather than
            // a hole (`a_course_without_a_credits_card_is_worth_zero_credits`),
            // so the page that must fail here is the one carrying an
            // unreadable card
            ("credits", format!("{code}{title}{}", credits_card("trois"))),
            ("cycle", format!("{code}{title}{credits}")),
            (
                "equivalents",
                format!(
                    "{code}{title}{credits}{cycle}{unreadable_equivalent}"
                ),
            ),
        ] {
            let html = format!("<html><body>{body}</body></html>");
            assert!(
                parse(&html).is_err(),
                "a page missing {missing} was accepted"
            );
        }
    }

    #[test]
    fn a_complete_page_without_sessions_is_a_course_with_no_season() {
        // the counterpart of the table above: every field present, so the
        // same `?`s let the page through
        let html = format!(
            "<html><body>{}{}{}{}</body></html>",
            r#"<span class="fe--titre-type">GEX-4008</span>"#,
            r#"<span class="fe--titre-nom">Approvisionnement</span>"#,
            credits_card("3"),
            cycle_card(&["Premier cycle"]),
        );

        let page = parse(&html)
            .expect("complete page")
            .expect("a first-cycle course is in scope");
        assert_eq!(page.course.code, "GEX-4008");
        assert!(page.course.seasons.is_empty());
        assert!(page.anomalies.is_empty());
    }

    #[test]
    fn a_section_missing_its_header_or_its_content_is_reported() {
        // Both halves are navigated to directly rather than searched for, so
        // a section whose shape drifts must say so instead of silently
        // reading a sibling section's fields.
        let without_header = format!(
            r#"<div class="toggle-section"><div class="toggle-section--content"><div class="toggle-section--content-wrapper">{}</div></div></div>"#,
            nrc_block("14854")
        );
        let doc = document(&without_header);
        assert!(matches!(
            parse_section(element(&doc, "div.toggle-section"), Nesting::TopLevel),
            Err(ParseError::MissingElement { selector }) if selector == "p.toggle-section--header"
        ));

        // no content at all: the NRC that lives there is unreachable
        let without_content = format!(
            r#"<div class="toggle-section"><p class="toggle-section--header">{}</p></div>"#,
            header(&["GEX-4008", "A", "En classe"])
        );
        let doc = document(&without_content);
        assert!(matches!(
            parse_section(element(&doc, "div.toggle-section"), Nesting::TopLevel),
            Err(ParseError::MissingElement { selector })
                if selector == "strong.section-cours--nrc"
        ));
    }

    #[test]
    fn a_section_whose_header_or_slot_is_unreadable_is_an_error() {
        // The vocabulary itself is tested on plain strings below; what these
        // rows prove is that a value the vocabulary rejects travels back out
        // as an error instead of being dropped — one row per `?` on the way.
        for (label, items, plages) in [
            ("header", vec!["GEX-4008", "En classe"], String::new()),
            (
                "mode",
                vec!["GEX-4008", "A", "En téléportation"],
                String::new(),
            ),
            (
                "slot",
                vec!["GEX-4008", "A", "En classe"],
                plage(&[
                    ("Dates:", "Du 12 jan. 2026 au 24 avr. 2026"),
                    ("Journée:", "Octidi"),
                    ("Horaire:", "De 8h30 à 11h20"),
                ]),
            ),
            (
                "schedule",
                vec!["GEX-4008", "A", "En classe"],
                plage(&[
                    ("Dates:", "Du 12 jan. 2026 au 24 avr. 2026"),
                    ("Journée:", "Vendredi"),
                    ("Horaire:", "8h30 - 11h20"),
                ]),
            ),
        ] {
            let doc = document(&toggle_section(
                &items,
                &format!("{}{plages}", nrc_block("14854")),
                "",
            ));
            assert!(
                parse_section(
                    element(&doc, "div.toggle-section"),
                    Nesting::TopLevel
                )
                .is_err(),
                "unreadable {label} was accepted"
            );
        }
    }

    // --- Simple fields ---

    #[test]
    fn a_missing_element_reports_the_selector_that_found_nothing() {
        let doc = document("<p>ni code ni titre</p>");
        match parse_element(&doc, &CODE, CODE_CSS) {
            Err(ParseError::MissingElement { selector }) => {
                assert_eq!(selector, "span.fe--titre-type");
            }
            other => panic!("expected MissingElement, got {other:?}"),
        }
    }

    #[test]
    fn a_course_without_a_credits_card_is_worth_zero_credits() {
        // the cycle card exists, so the scan runs and finds no « Crédits »
        // — GCI-2510, a seminar, is that shape and must survive the parse
        let doc = document(&cycle_card(&["Premier cycle"]));
        assert_eq!(
            parse_credits(&doc).unwrap_or_else(|e| panic!("{e}")),
            Credits::Fixed(0)
        );
    }

    #[test]
    fn a_credits_range_keeps_both_bounds() {
        // « N à M » is a stage the student weights himself (MED-1911 is
        // « 6 à 12 »), not markup drift — dropping the page would cost the
        // course its whole schedule
        for (raw, expected) in [
            ("6 à 12", Credits::Range { min: 6, max: 12 }),
            ("2 à 4", Credits::Range { min: 2, max: 4 }),
            ("0 à 6", Credits::Range { min: 0, max: 6 }),
            ("3", Credits::Fixed(3)),
        ] {
            let doc = document(&credits_card(raw));
            assert_eq!(
                parse_credits(&doc).unwrap_or_else(|e| panic!("{raw}: {e}")),
                expected,
                "for {raw:?}"
            );
        }
    }

    #[test]
    fn a_credits_range_running_backwards_is_a_malformed_entry() {
        // no page states one, and reading it as a range would let a bound
        // no student can satisfy pass for a fact about the course
        let doc = document(&credits_card("4 à 2"));
        let error = parse_credits(&doc).expect_err("descending range");
        assert_eq!(malformed_entry(&error), ("credits", "4 à 2"));
    }

    #[test]
    fn a_credits_card_of_an_unknown_shape_is_a_malformed_entry() {
        // neither one number nor « N à M »: an empty card, anything wordier,
        // and a bound that is not a number are drift, never a silent zero
        for raw in ["", "de 3 à 6", "3 à 6 à 9", "trois à 6", "6 à trois"]
        {
            let doc = document(&credits_card(raw));
            let error =
                parse_credits(&doc).expect_err("unknown credits shape");
            assert_eq!(
                malformed_entry(&error),
                ("credits", raw),
                "for {raw:?}"
            );
        }
    }

    #[test]
    fn a_credits_card_without_a_value_is_a_missing_element() {
        let doc = document(&fait_rapide(
            r#"<span class="promo-entete--contenu">Crédits</span>"#,
        ));
        match parse_credits(&doc) {
            Err(ParseError::MissingElement { selector }) => {
                assert_eq!(selector, "span.promo-entete--titre");
            }
            other => panic!("expected MissingElement, got {other:?}"),
        }
    }

    #[test]
    fn non_numeric_credits_are_a_malformed_entry() {
        // markup drift, never a silent zero
        let doc = document(&fait_rapide(
            r#"<span class="promo-entete--titre">trois</span><span class="promo-entete--contenu">Crédits</span>"#,
        ));
        let error = parse_credits(&doc).expect_err("non-numeric credits");
        assert_eq!(malformed_entry(&error), ("credits", "trois"));
    }

    #[test]
    fn cycle_is_missing_when_no_card_carries_the_label() {
        let doc = document(&fait_rapide(
            r#"<p class="promo-paragraphe">Modes d'enseignement</p>"#,
        ));
        assert!(matches!(
            parse_cycle(&doc),
            Err(ParseError::MissingElement { .. })
        ));
    }

    #[test]
    fn a_cycle_card_listing_nothing_is_a_malformed_entry() {
        let doc = document(&cycle_card(&[]));
        let error = parse_cycle(&doc).expect_err("empty cycle card");
        assert_eq!(malformed_entry(&error).0, "cycle values");
    }

    #[test]
    fn an_unknown_cycle_name_is_a_malformed_entry() {
        let doc = document(&cycle_card(&["Quatrième cycle"]));
        let error = parse_cycle(&doc).expect_err("unknown cycle");
        assert_eq!(malformed_entry(&error), ("cycle", "Quatrième cycle"));
    }

    #[test]
    fn a_course_above_the_second_cycle_is_out_of_scope_not_an_error() {
        // 2e-3e collapses to 2, but a course whose *lowest* listed level is
        // above the second is a doctoral or post-doctoral activity: nothing
        // to schedule, and `Cycle` cannot hold it. Recognized, then dropped
        // on purpose — hence `None` rather than an anomaly, which would fill
        // the log with a case we understand perfectly.
        for level in ["Troisième cycle", "Études post-MDD"] {
            let doc = document(&cycle_card(&[level]));
            assert_eq!(
                parse_cycle(&doc).unwrap_or_else(|e| panic!("{level}: {e}")),
                None,
                "for {level:?}"
            );
        }
    }

    #[test]
    fn the_lowest_listed_cycle_wins() {
        let doc =
            document(&cycle_card(&["Troisième cycle", "Deuxième cycle"]));
        assert_eq!(parse_cycle(&doc).expect("cycle"), Some(Cycle::Second));
    }

    // --- Préalables and equivalents ---

    #[test]
    fn in_grammar_prerequisites_are_parsed_into_a_tree() {
        // the raw text is kept alongside the tree: the tree drives the
        // solver, the raw text is what a human checks it against
        let doc = document(
            r#"<div class="fe--prealables"><p class="etiquette-container">GAE-1004 ET GAE-2000</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "GAE-1004 ET GAE-2000".to_string(),
                tree: all(vec![course("GAE-1004"), course("GAE-2000")]),
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_prerequisite_no_rule_can_check_is_kept_as_text() {
        // a prose préalable is one operand no checkable shape fits: it lands
        // in the tree verbatim, where the UI shows it to the student. It is
        // not an anomaly — nothing went wrong, the source simply asks for
        // something no catalogue can verify
        let doc = document(
            r#"<div class="fe--prealables"><p class="etiquette-container">Autorisation de la direction</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "Autorisation de la direction".to_string(),
                tree: PrereqTree::Raw {
                    raw: "Autorisation de la direction".to_string()
                },
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_broken_expression_keeps_no_tree_at_all() {
        // an unclosed group has no local repair — there is no telling which
        // operands it was meant to hold — so the whole expression stays raw
        let doc = document(
            r#"<div class="fe--prealables"><p class="etiquette-container">(GAE-1004 ET GAE-2000</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Raw {
                raw: "(GAE-1004 ET GAE-2000".to_string()
            })
        );
        assert!(matches!(
            anomalies.as_slice(),
            [ParseError::MalformedPrerequisites { .. }]
        ));
    }

    #[test]
    fn an_equivalent_without_a_readable_code_is_a_malformed_entry() {
        // a live card whose sigle is unreadable is reported, not skipped
        let doc = document(
            r#"<li class="bloc-cours carte-accessible"><a class="carte-accessible--lien"><span class="sigle">ECN-99</span></a></li>"#,
        );
        let error = parse_equivalents(&doc).expect_err("malformed sigle");
        assert_eq!(
            malformed_entry(&error).0,
            "a.carte-accessible--lien span.sigle"
        );
    }

    // --- Sessions ---

    #[test]
    fn a_session_heading_names_a_season_and_a_year() {
        for (heading, expected) in [
            ("Automne 2026 – 1 section offerte", (Season::Fall, 2026)),
            ("Hiver 2026 – 2 sections offertes", (Season::Winter, 2026)),
            ("Été 2026 – 1 section offerte", (Season::Summer, 2026)),
        ] {
            assert_eq!(
                parse_session_heading(heading)
                    .unwrap_or_else(|e| panic!("{heading}: {e}")),
                expected,
                "for {heading:?}"
            );
        }
    }

    #[test]
    fn only_the_most_recent_session_of_a_season_is_kept() {
        // gci-1007 carries Automne 2024, 2025 and 2026: the founding
        // hypothesis keeps one snapshot per season, so the newest wins
        // whichever order the page lists them in (ADR
        // `2026-07-conception-du-parseur-de-cours` §4)
        let older = session(
            "Automne 2025 – 1 section offerte",
            &toggle_section(
                &["GCI-1007", "A", "En classe"],
                &nrc_block("11111"),
                "",
            ),
        );
        let newer = session(
            "Automne 2026 – 1 section offerte",
            &toggle_section(
                &["GCI-1007", "A", "En classe"],
                &nrc_block("22222"),
                "",
            ),
        );

        for (order, html) in [
            ("newest first", format!("{newer}{older}")),
            ("oldest first", format!("{older}{newer}")),
        ] {
            let doc = document(&html);
            let mut anomalies = Vec::new();

            let seasons = parse_seasons(&doc, &mut anomalies);

            assert_eq!(seasons.len(), 1, "one offering per season ({order})");
            let (year, offering) = &seasons[&Season::Fall];
            assert_eq!(*year, 2026, "the 2026 session wins ({order})");
            assert_eq!(
                offering.options[0][0].nrc, "22222",
                "the 2026 session wins ({order})"
            );
            assert!(anomalies.is_empty(), "{order}: {anomalies:?}");
        }
    }

    #[test]
    fn an_unreadable_session_heading_is_an_anomaly() {
        for heading in ["Printemps 2026 – 1 section offerte", "Automne –"]
        {
            let doc = document(&session(heading, ""));
            let mut anomalies = Vec::new();

            assert!(parse_seasons(&doc, &mut anomalies).is_empty());
            assert_eq!(
                malformed_entry(&anomalies[0]),
                ("p.controls-title", heading),
                "for {heading:?}"
            );
        }
    }

    #[test]
    fn a_session_without_a_heading_is_skipped() {
        let doc = document(r#"<div class="collapsible-sections"></div>"#);
        let mut anomalies = Vec::new();

        assert!(parse_seasons(&doc, &mut anomalies).is_empty());
        assert!(anomalies.is_empty());
    }

    #[test]
    fn the_advertised_section_count_is_cross_checked() {
        let sections = toggle_section(
            &["GEX-4008", "A", "En classe"],
            &nrc_block("14854"),
            "",
        );
        let doc =
            document(&session("Hiver 2026 – 2 sections offertes", &sections));
        let mut anomalies = Vec::new();

        let seasons = parse_seasons(&doc, &mut anomalies);
        assert_eq!(seasons[&Season::Winter].1.options.len(), 1);
        assert_eq!(malformed_entry(&anomalies[0]).0, "p.controls-title");
    }

    #[test]
    fn a_heading_advertising_no_readable_count_is_an_anomaly() {
        // The heading names a season and a year — so the session is kept —
        // but the count it should be reconciled against is absent or not a
        // number. Failing to read it must not pass for agreement.
        for heading in [
            "Automne 2026 –",
            "Automne 2026 – plusieurs sections offertes",
        ] {
            let sections = toggle_section(
                &["GEX-4008", "A", "En classe"],
                &nrc_block("14854"),
                "",
            );
            let doc = document(&session(heading, &sections));
            let mut anomalies = Vec::new();

            let seasons = parse_seasons(&doc, &mut anomalies);
            assert_eq!(seasons[&Season::Fall].1.options.len(), 1);
            assert_eq!(
                malformed_entry(&anomalies[0]).0,
                "p.controls-title",
                "for {heading:?}"
            );
        }
    }

    #[test]
    fn a_plage_item_that_is_not_a_labelled_field_is_ignored() {
        // Guards against drift: an item with no <strong> carries no label,
        // and one whose text does not start with its own label cannot be
        // split into label and value. Neither may be read as a field.
        for item in [
            r#"<li class="section-cours--etiquette">sans étiquette</li>"#,
            r#"<li class="section-cours--etiquette">préfixe<strong>Journée:</strong> Vendredi</li>"#,
        ] {
            let doc = document(&format!(
                r#"<ul class="section-cours--liste">{item}</ul>"#
            ));
            assert_eq!(
                plage_field(
                    element(&doc, "ul.section-cours--liste"),
                    "Journée:"
                ),
                None,
                "for {item}"
            );
        }
    }

    #[test]
    fn a_linked_section_belongs_to_the_section_that_holds_it() {
        // The shape IFT-1004 exhibits and the old flat model could not
        // express: 84664 offers a choice of two labs, 84667 offers none.
        // Reading this as « one of {84664, 84667} and one of {84665, 84666} »
        // would pair 84667 with a lab that is not its own, and would have no
        // way to say « 84667 alone ».
        let labs = format!(
            "{}{}",
            toggle_section(&["A", "En classe"], &nrc_block("84665"), ""),
            toggle_section(&["B", "En classe"], &nrc_block("84666"), ""),
        );
        let with_labs = toggle_section(
            &["GCI-1007", "", "En classe"],
            &nrc_block("84664"),
            &labs,
        );
        let alone = toggle_section(
            &["GCI-1007", "Z3", "À distance"],
            &nrc_block("84667"),
            "",
        );
        let doc = document(&session(
            "Automne 2026 – 2 sections offertes",
            &format!("{with_labs}{alone}"),
        ));
        let mut anomalies = Vec::new();

        let seasons = parse_seasons(&doc, &mut anomalies);

        let nrcs: Vec<Vec<&str>> = seasons[&Season::Fall]
            .1
            .options
            .iter()
            .map(|option| {
                option.iter().map(|s| s.nrc.as_str()).collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            nrcs,
            vec![
                vec!["84664", "84665"],
                vec!["84664", "84666"],
                vec!["84667"],
            ]
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_top_level_section_re_parented_by_a_stray_tag_is_still_found() {
        // DRT-7104 writes `<b>…<b>` where it means `</b>`. HTML5 rebuilds the
        // unclosed formatting elements around everything that follows, so the
        // second section is no longer a *direct* child of the session — a
        // direct-children scan silently loses it, schedule and all. Only the
        // `.dark` wrapper may hide a section from this level.
        //
        // The newline after the stray tag is load-bearing: the tree builder
        // reconstructs the open `<b>`s when it inserts *character* data, not
        // when it inserts a `<div>`. Remove it and the markup parses clean,
        // which would make this test pass against the very bug it pins.
        let sections = format!(
            "{}\n<div class=\"fe--message\"><p><b>note<b></p></div>\n{}",
            toggle_section(
                &["DRT-7104", "A", "En classe"],
                &nrc_block("84328"),
                "",
            ),
            toggle_section(
                &["DRT-7104", "B", "En classe"],
                &nrc_block("84329"),
                "",
            ),
        );
        let doc = document(&session(
            "Automne 2023 – 2 sections offertes",
            &sections,
        ));
        let mut anomalies = Vec::new();

        let seasons = parse_seasons(&doc, &mut anomalies);

        let nrcs: Vec<&str> = seasons[&Season::Fall]
            .1
            .options
            .iter()
            .flatten()
            .map(|s| s.nrc.as_str())
            .collect();
        assert_eq!(nrcs, vec!["84328", "84329"]);
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_section_whose_labs_are_all_unreadable_offers_no_enrolment() {
        // The page ties this lecture to a lab, so it cannot be taken alone.
        // Handing it back bare would invent an enrolment nobody offers; the
        // anomaly is what makes the loss recoverable.
        let broken_lab = toggle_section(&["A", "En classe"], "", "");
        let with_labs = toggle_section(
            &["GCI-1007", "", "En classe"],
            &nrc_block("84664"),
            &broken_lab,
        );
        let doc =
            document(&session("Automne 2026 – 1 section offerte", &with_labs));
        let mut anomalies = Vec::new();

        assert!(
            parse_seasons(&doc, &mut anomalies).is_empty(),
            "the lecture alone is not an enrolment the page offers"
        );
        assert!(
            anomalies.iter().any(|anomaly| matches!(
                anomaly,
                ParseError::MissingElement { selector } if selector == NRC_CSS
            )),
            "got {anomalies:?}"
        );
    }

    #[test]
    fn a_section_that_cannot_be_read_is_dropped_and_surfaced() {
        let sections = toggle_section(&["GEX-4008", "A", "En classe"], "", "");
        let doc =
            document(&session("Hiver 2026 – 1 section offerte", &sections));
        let mut anomalies = Vec::new();

        // the only section is unreadable, so the season carries no group
        assert!(parse_seasons(&doc, &mut anomalies).is_empty());
        assert!(
            anomalies.iter().any(|error| matches!(
                error,
                ParseError::MissingElement { selector }
                    if selector == "strong.section-cours--nrc"
            )),
            "got {anomalies:?}"
        );
    }

    // --- Section header ---

    #[test]
    fn a_header_of_unexpected_width_is_a_malformed_entry() {
        let doc = document(&toggle_section(
            &["GEX-4008", "En classe"],
            &nrc_block("14854"),
            "",
        ));
        let error = parse_section_header(
            element(&doc, "p.toggle-section--header"),
            Nesting::TopLevel,
        )
        .expect_err("two items at the top level");
        assert_eq!(
            malformed_entry(&error),
            ("span.header--content-details", "GEX-4008 | En classe")
        );
    }

    // --- Plages horaires ---

    #[test]
    fn a_one_off_plage_yields_no_slot() {
        // « Date: » singular — a kickoff meeting, not a weekly commitment
        assert_eq!(
            slot_of(&[
                ("Type:", "Rencontre"),
                ("Date:", "16 jan. 2026"),
                ("Journée:", "Vendredi"),
                ("Horaire:", "De 8h30 à 11h20"),
            ])
            .expect("one-off plage"),
            None
        );
    }

    #[test]
    fn a_hybrid_section_keeps_only_its_in_class_meetings() {
        // GEX-3100: a « Hybride » section lists a « Sur Internet » plage
        // carrying dates but neither day nor schedule, then the in-class
        // one. Only the latter can occupy a place in a timetable.
        let dates = "Du 6 sept. 2022 au 16 déc. 2022";
        let body = format!(
            "{}{}{}",
            nrc_block("85174"),
            plage(&[("Type:", "Sur Internet"), ("Dates:", dates)]),
            plage(&[
                ("Type:", "En classe"),
                ("Dates:", dates),
                ("Journée:", "Mardi"),
                ("Horaire:", "De 9h30 à 12h20"),
            ]),
        );
        let doc = document(&session(
            "Automne 2022 – 1 section offerte",
            &toggle_section(&["GEX-3100", "H", "Hybride"], &body, ""),
        ));
        let mut anomalies = Vec::new();

        let seasons = parse_seasons(&doc, &mut anomalies);

        assert!(anomalies.is_empty(), "{anomalies:?}");
        let section = &seasons[&Season::Fall].1.options[0][0];
        assert_eq!(section.mode, Mode::Hybrid);
        assert_eq!(
            section.slots.len(),
            1,
            "the remote half occupies no timetable slot"
        );
        assert_eq!(section.slots[0].day, Day::Tuesday);
    }

    #[test]
    fn a_plage_without_a_day_or_a_schedule_yields_no_slot() {
        for fields in [
            vec![("Dates:", "Du 12 jan. 2026 au 24 avr. 2026")],
            vec![
                ("Dates:", "Du 12 jan. 2026 au 24 avr. 2026"),
                ("Journée:", "Vendredi"),
            ],
        ] {
            assert_eq!(slot_of(&fields).expect("no slot"), None);
        }
    }

    #[test]
    fn a_recurring_plage_becomes_a_slot() {
        // the counterpart of the two tests above, and the one place the
        // wiring from labelled fields to a Slot is pinned end to end; the
        // vocabulary each field is read with is tested on its own below
        let slot = slot_of(&[
            ("Dates:", "Du 12 jan. 2026 au 24 avr. 2026"),
            ("Journée:", "Vendredi"),
            ("Horaire:", "De 8h30 à 11h20"),
        ])
        .expect("slot")
        .expect("some slot");

        assert_eq!(slot.day, Day::Friday);
        assert_eq!(
            slot.start,
            Time {
                hour: 8,
                minute: 30
            }
        );
        assert_eq!(
            slot.end,
            Time {
                hour: 11,
                minute: 20
            }
        );
    }

    // --- Vocabulaire : texte de la page → valeur du domaine ---
    //
    // Every value ULaval writes is read by a pure function, so the table of
    // accepted spellings is a table of strings rather than a page to build.

    #[test]
    fn every_day_of_the_week_is_recognized() {
        for (label, expected) in [
            ("Lundi", Day::Monday),
            ("Mardi", Day::Tuesday),
            ("Mercredi", Day::Wednesday),
            ("Jeudi", Day::Thursday),
            ("Vendredi", Day::Friday),
            ("Samedi", Day::Saturday),
            ("Dimanche", Day::Sunday),
        ] {
            assert_eq!(
                parse_day(label).unwrap_or_else(|e| panic!("{label}: {e}")),
                expected,
                "for {label}"
            );
        }
    }

    #[test]
    fn an_unknown_day_is_a_malformed_entry() {
        let error = parse_day("Octidi").expect_err("unknown day");
        assert_eq!(malformed_entry(&error), ("day", "Octidi"));
    }

    #[test]
    fn both_teaching_modes_are_recognized() {
        for (label, expected) in [
            ("En classe", Mode::InPerson),
            ("À distance", Mode::Remote),
            ("Hybride", Mode::Hybrid),
            // GMC-7000 spells the hybrid arrangement its own way
            ("À distance-hybride", Mode::Hybrid),
            ("Comodal", Mode::Hybrid),
        ] {
            assert_eq!(
                parse_mode(label).unwrap_or_else(|e| panic!("{label}: {e}")),
                expected,
                "for {label}"
            );
        }
    }

    #[test]
    fn an_unknown_mode_is_a_malformed_entry() {
        let error = parse_mode("En téléportation").expect_err("unknown mode");
        assert_eq!(malformed_entry(&error), ("mode", "En téléportation"));
    }

    #[test]
    fn a_schedule_is_read_as_a_pair_of_times() {
        for (raw, start, end) in [
            (
                "De 8h30 à 11h20",
                Time {
                    hour: 8,
                    minute: 30,
                },
                Time {
                    hour: 11,
                    minute: 20,
                },
            ),
            // GCI-2010 carries « De 9h à 11h50 »: an hour without minutes
            // is on the hour
            (
                "De 9h à 11h50",
                Time { hour: 9, minute: 0 },
                Time {
                    hour: 11,
                    minute: 50,
                },
            ),
        ] {
            assert_eq!(
                parse_schedule(raw).unwrap_or_else(|e| panic!("{raw}: {e}")),
                (start, end),
                "for {raw:?}"
            );
        }
    }

    #[test]
    fn an_unreadable_schedule_is_a_malformed_entry() {
        for (schedule, selector) in [
            ("8h30 à 11h20", "schedule"),
            ("De 8h30 - 11h20", "schedule"),
            ("De 8x30 à 11h20", "time"),
            ("De ah30 à 11h20", "time"),
            ("De 8hxx à 11h20", "time"),
            // a readable start does not excuse an unreadable end
            ("De 8h30 à 11x20", "time"),
            ("De 25h00 à 26h00", "time"),
        ] {
            let error =
                parse_schedule(schedule).expect_err("unreadable schedule");
            assert_eq!(
                malformed_entry(&error).0,
                selector,
                "for {schedule:?}"
            );
        }
    }

    // an operand no checkable shape fits comes back as text, verbatim
    fn assert_kept_as_text(raw: &str) {
        let tree =
            parse_prereq_tree(raw).unwrap_or_else(|e| panic!("{raw:?}: {e}"));
        assert_eq!(
            tree,
            PrereqTree::Raw {
                raw: raw.to_string()
            },
            "for {raw:?}"
        );
    }

    fn course(code: &str) -> PrereqTree {
        PrereqTree::Course(code.to_string())
    }

    fn all(trees: Vec<PrereqTree>) -> PrereqTree {
        PrereqTree::All { all: trees }
    }

    fn any(trees: Vec<PrereqTree>) -> PrereqTree {
        PrereqTree::Any { any: trees }
    }

    #[test]
    fn single_course_is_a_leaf() {
        let tree = parse_prereq_tree("GGL-2600")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(tree, course("GGL-2600"));
    }

    #[test]
    fn flat_ou_is_any_of_its_terms() {
        // matches fixture gci-1007
        let tree = parse_prereq_tree("GGL-2600 OU GLG-1900 OU GLG-1000")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                course("GGL-2600"),
                course("GLG-1900"),
                course("GLG-1000"),
            ])
        );
    }

    #[test]
    fn flat_et_is_all_of_its_factors() {
        let tree = parse_prereq_tree("GAE-1004 ET GAE-2000")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(tree, all(vec![course("GAE-1004"), course("GAE-2000")]));
    }

    #[test]
    fn et_binds_tighter_than_ou_without_parens() {
        let tree = parse_prereq_tree("GAE-1004 ET GAE-2000 OU GCI-2009")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                all(vec![course("GAE-1004"), course("GAE-2000")]),
                course("GCI-2009"),
            ])
        );
    }

    #[test]
    fn the_observed_parenthesized_form_parses_the_same_as_without_parens() {
        let tree = parse_prereq_tree("((GAE-1004 ET GAE-2000) OU GCI-2009)")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                all(vec![course("GAE-1004"), course("GAE-2000")]),
                course("GCI-2009"),
            ])
        );
    }

    #[test]
    fn parens_override_default_precedence() {
        let tree = parse_prereq_tree("(GAE-1004 OU GAE-2000) ET GCI-2009")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            all(vec![
                any(vec![course("GAE-1004"), course("GAE-2000")]),
                course("GCI-2009"),
            ])
        );
    }

    #[test]
    fn credits_requirement_is_a_program_credits_leaf() {
        let tree = parse_prereq_tree("GEX, Crédits exigés : 60")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            PrereqTree::ProgramCredits {
                program_credits: ProgramCredits {
                    program: Some("GEX".to_string()),
                    credits: 60,
                }
            }
        );
    }

    #[test]
    fn a_credits_requirement_can_name_no_program() {
        // GEX-3333 reads « … ET  Crédits exigés : 72 » — the requirement
        // then bears on the student's own programme, so the field is empty
        // rather than the expression being out of grammar
        let tree = parse_prereq_tree("Crédits exigés : 72")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            PrereqTree::ProgramCredits {
                program_credits: ProgramCredits {
                    program: None,
                    credits: 72,
                }
            }
        );
    }

    #[test]
    fn a_bound_on_a_credits_requirement_drops_out() {
        // GMC-1590 reads « … ET  1000 à 4999 Crédits exigés : 15 »,
        // ACT-4114 « … ET ACT-1000 à 4999, Crédits exigés : 39 » and
        // PHI-7750 « … ET PHI-6000 à 8899, Crédits exigés : 12 » — the range
        // always covers the cycle of the course carrying the requirement, and
        // the cycle is in the snapshot, so the bound is rebuilt at planning
        // time rather than carried here (ADR
        // `2026-07-bornes-de-credits-toutes-retirees`)
        for (raw, expected) in [
            ("GMC-1024 ET 1000 à 4999 Crédits exigés : 15", (None, 15)),
            (
                "GMC-1024 ET ACT-1000 à 4999, Crédits exigés : 39",
                (Some("ACT"), 39),
            ),
            (
                "GMC-1024 ET PHI-6000 à 8899, Crédits exigés : 12",
                (Some("PHI"), 12),
            ),
            // a bound narrower than its cycle is read as its cycle: the
            // widening is accepted, the source text stays in `raw`
            ("GMC-1024 ET 1000 à 2999 Crédits exigés : 12", (None, 12)),
        ] {
            let tree = parse_prereq_tree(raw)
                .unwrap_or_else(|e| panic!("parse {raw:?}: {e}"));
            assert_eq!(
                tree,
                all(vec![
                    course("GMC-1024"),
                    PrereqTree::ProgramCredits {
                        program_credits: ProgramCredits {
                            program: expected.0.map(str::to_string),
                            credits: expected.1,
                        }
                    },
                ]),
                "for {raw:?}"
            );
        }
    }

    #[test]
    fn only_a_four_digit_range_reads_as_a_bound() {
        // a bound is two course numbers; anything else keeping the same
        // shape is not one, and is kept verbatim rather than stripped on a
        // guess
        for raw in [
            "60 à 4999 Crédits exigés : 12",
            "1000 à 49999 Crédits exigés : 12",
            "mille à 4999 Crédits exigés : 12",
            // the bound drops out, but the count behind it is still unread
            "1000 à 4999 Crédits exigés : plusieurs",
        ] {
            assert_kept_as_text(raw);
        }
    }

    #[test]
    fn a_range_of_courses_on_its_own_is_a_raw_operand() {
        // ESP-1000 reads « ESG-2020 à 3799 OU … »: with no credits
        // requirement behind it the range names the courses themselves, one
        // of which satisfies the préalable — a choice the grammar cannot
        // make, so the three words are kept verbatim
        let tree = parse_prereq_tree("ESG-2020 à 3799 OU GCI-1000")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                PrereqTree::Raw {
                    raw: "ESG-2020 à 3799".to_string()
                },
                course("GCI-1000"),
            ])
        );
    }

    #[test]
    fn an_operand_naming_an_examination_is_kept_verbatim() {
        // FRN-1904 requires an examination result, ESP-1000 a placement
        // test: no rule can check either, so the operand is kept whole
        // instead of dragging the whole expression out of grammar
        for (raw, expected) in [
            (
                "Examen Test français Laval-Montréal avec résultat de 060.0 à 100.0",
                "Examen Test français Laval-Montréal avec résultat de 060.0 à 100.0",
            ),
            // the run stops at the operator, not at the end of the text
            (
                "Examen Classement en espagnol avec résultat de 5 à 8 OU GCI-1000",
                "Examen Classement en espagnol avec résultat de 5 à 8",
            ),
        ] {
            let tree = parse_prereq_tree(raw)
                .unwrap_or_else(|e| panic!("parse {raw:?}: {e}"));
            let first = match &tree {
                PrereqTree::Any { any } => any[0].clone(),
                leaf => leaf.clone(),
            };
            assert_eq!(
                first,
                PrereqTree::Raw {
                    raw: expected.to_string()
                },
                "for {raw:?}"
            );
        }
    }

    #[test]
    fn a_raw_operand_stops_at_a_parenthesis() {
        let tree =
            parse_prereq_tree("( Examen de langue OU GCI-1000 ) ET GCI-2000")
                .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            all(vec![
                any(vec![
                    PrereqTree::Raw {
                        raw: "Examen de langue".to_string()
                    },
                    course("GCI-1000"),
                ]),
                course("GCI-2000"),
            ])
        );
    }

    #[test]
    fn a_raw_operand_still_needs_an_operator_beside_it() {
        // the run stops at « ( », so the group that follows is a second
        // operand with no operator between them — a broken structure, which
        // no operand kept verbatim can repair
        assert!(parse_prereq_tree("Examen de langue ( GCI-1000 )").is_err());
    }

    #[test]
    fn credits_not_followed_by_a_requirement_is_out_of_grammar() {
        for raw in [
            "Crédits",
            "Crédits exigés",
            "Crédits obtenus : 72",
            "Crédits exigés : plusieurs",
        ] {
            assert_kept_as_text(raw);
        }
    }

    #[test]
    fn credits_can_appear_inside_a_boolean_expression() {
        // exigence_credits is a facteur alternative in the grammar
        // (docs/conception/initial/CONCEPTION.md), so it can be an operand
        // of OU/ET like any course code, not just stand alone.
        let tree = parse_prereq_tree("GCI-1001 OU GEX, Crédits exigés : 45")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                course("GCI-1001"),
                PrereqTree::ProgramCredits {
                    program_credits: ProgramCredits {
                        program: Some("GEX".to_string()),
                        credits: 45,
                    }
                },
            ])
        );
    }

    #[test]
    fn nested_groups_on_both_sides_of_ou() {
        let tree = parse_prereq_tree(
            "(GLG-1000 ET GLG-1900) OU (GGL-2600 ET GCI-2009)",
        )
        .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                all(vec![course("GLG-1000"), course("GLG-1900")]),
                all(vec![course("GGL-2600"), course("GCI-2009")]),
            ])
        );
    }

    #[test]
    fn a_broken_structure_is_a_malformed_prerequisites_error() {
        // What fails as a whole is the *shape* of the expression: an empty
        // one, a group left open or closed alone, an operator missing an
        // operand. None of these can be repaired by keeping text in place,
        // unlike an operand nobody can read.
        for raw in [
            "",
            "   ",
            "(GAE-1004 ET GAE-2000",
            "GAE-1004 ET GAE-2000)",
            "GLG-1900 OU",
            "GLG-1900 OU ET GLG-1000",
            "GLG-1000 (GLG-1900 OU GGL-2600)",
            "()",
            "OU GLG-1000",
        ] {
            let result = parse_prereq_tree(raw);
            assert!(
                matches!(
                    &result,
                    Err(ParseError::MalformedPrerequisites { raw: got, .. })
                        if got.contains(raw)
                ),
                "expected MalformedPrerequisites for {raw:?}, got {result:?}"
            );
        }
    }

    #[test]
    fn an_operand_no_shape_reads_is_kept_in_place_and_reported() {
        // Every way an operand can defeat the grammar — prose, a sigle
        // miswritten at the source, a count in words, a matière of the wrong
        // width, two operands with no operator between them, a bound whose
        // subject is not a matière. Each is kept verbatim, each is reported.
        for raw in [
            "Connaissance de base en programmation",
            "GEX, Crédits exigés : soixante",
            "GEX, Crédits exigés :",
            "GEXX, Crédits exigés : 60",
            ", Crédits exigés : 60",
            "GLG-100",
            "GLG-1000 GLG-1900",
            "GLG-1000 GEX, Crédits exigés : 60",
            "PHIL-6000 à 8899, Crédits exigés : 12",
            // FRN-1112 reads « FRN-1910 OU FRN 19543 »: a sigle the source
            // mistyped, which no rule can repair — FRN-1954 and FRN-1543
            // both being absent from the catalogue
            "FRN 19543",
        ] {
            assert_kept_as_text(raw);
        }
    }

    #[test]
    fn an_unreadable_operand_leaves_the_rest_of_the_expression_readable() {
        // the point of keeping it in place: FRN-1112 keeps FRN-1910, which
        // the whole-expression fallback used to take down with the typo
        let tree = parse_prereq_tree("FRN-1910 OU FRN 19543")
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(
            tree,
            any(vec![
                course("FRN-1910"),
                PrereqTree::Raw {
                    raw: "FRN 19543".to_string()
                },
            ])
        );
    }

    #[test]
    fn each_operand_and_operator_guard_reports_its_own_error_label() {
        // The table above only proves each input is *some* kind of
        // MalformedPrerequisites; these are chosen to each trip a different
        // guard, so check the `error` label to prove which one.
        for (raw, expected_error) in [
            // a closed group followed by an operand is the only way to reach
            // the guard: anywhere else, a separator would have swallowed it
            ("( GLG-1900 ) GLG-1000", "two operands in a row"),
            (
                "GLG-1000 (GLG-1900 OU GGL-2600)",
                "( where an operator was expected",
            ),
            ("()", ") without a left operand"),
            ("OU GLG-1000", "OU without a left operand"),
        ] {
            let result = parse_prereq_tree(raw);
            match result {
                Err(ParseError::MalformedPrerequisites { error, .. }) => {
                    assert_eq!(
                        error, expected_error,
                        "wrong error label for {raw:?}"
                    );
                }
                other => panic!(
                    "expected MalformedPrerequisites for {raw:?}, got {other:?}"
                ),
            }
        }
    }
}
