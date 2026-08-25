use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

use crate::parser::ParseError;
use crate::{
    is_course_code, parse_prereq_tree, Course, CourseCycle, Credits, Day,
    Mode, Prerequisites, Season, SeasonOffering, Section, Slot, Time,
};
use scraper::{ElementRef, Html, Selector};

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
static FE_MESSAGE: LazyLock<Selector> =
    LazyLock::new(|| sel("div.fe--message"));

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
    pub anomalies: Vec<ParseError>,
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

    // The new-course rule: a page with no session section at all is a course
    // whose schedule is not yet published (GCI-1011) — offered Fall and
    // Winter, vintage and schedule unknown (ADR
    // `2026-07-cours-sans-section-de-session-offert-automne-hiver`). The
    // guard is the *absence of the section*, not an empty parse result: a
    // session block that yields nothing left an anomaly, and nothing is
    // invented next to an anomaly.
    let seasons = if doc.select(&SESSION).next().is_none() {
        [Season::Fall, Season::Winter]
            .into_iter()
            .map(|season| {
                (
                    season,
                    SeasonOffering {
                        last_offered: None,
                        options: None,
                    },
                )
            })
            .collect()
    } else {
        parse_seasons(&doc, &mut anomalies)
    };

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

// `None` for an activity above the second cycle: `CourseCycle` cannot hold it,
// and the generator has no business scheduling a thesis milestone or a
// post-doctoral residency. Recognized, then dropped — not an error.
fn parse_cycle(doc: &Html) -> Result<Option<CourseCycle>, ParseError> {
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
    Ok(CourseCycle::try_from(level).ok())
}

fn cycle_level(text: &str) -> Result<u8, ParseError> {
    match text.trim() {
        // CHM-0150, a « cours d'appoint » below the first cycle — in scope for
        // a course, unlike a programme (ADR
        // `2026-07-cycle-preuniversitaire-cours-seulement`)
        "Préuniversitaire" => Ok(0),
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
    let regular = doc
        .select(&PREALABLES)
        .next()
        .map(|element| element.text().collect::<String>().trim().to_string());

    // a course with no préuniversitaire field keeps its raw byte-for-byte;
    // the field only ever folds in as extra ET-operands (ADR
    // `2026-07-prealables-preuniversitaires-fusionnes`)
    let mut parts: Vec<String> = regular
        .into_iter()
        .chain(parse_preuniversitaire(doc, anomalies))
        .collect();
    let raw = match parts.len() {
        0 => return None,
        1 => parts.swap_remove(0),
        _ => {
            let grouped: Vec<String> =
                parts.iter().map(|part| format!("({part})")).collect();
            grouped.join(" ET ")
        }
    };

    match parse_prereq_tree(&raw) {
        Ok(tree) => Some(Prerequisites::Parsed { raw, tree }),
        // only the structure of the expression can fail: an operand nobody
        // can check is kept in place as text (ADR
        // `2026-07-operande-non-verifiable-gardee-en-texte`)
        Err(error) => {
            anomalies.push(ParseError::MalformedPrerequisites {
                error: error.error,
                raw: error.raw,
            });
            Some(Prerequisites::Raw { raw })
        }
    }
}

// « REMARQUE : Préalables préuniversitaires nécessaires s'il y a lieu :
// BIO-0150, CHM-0150, CHM-0160 ou CHM-0170. » sits in a `.fe--message` box
// (one per session offering) the regular selector never reads. Each marker
// node yields the expression between the colon *after* the marker — a prefix
// like « REMARQUE : » carries its own colon — and the first period, which
// both ends the sentence and glues the prose that follows
// (« MAT-0150.Cette section… »). The expression is handed to the préalables
// grammar with its connectors uppercased; what the grammar cannot check
// (comma lists, cégep sigles, « équivalent ») survives as a Raw operand.
// Distinct messages on one page (BIO-1003) are all kept, ET-joined by the
// caller. ADR `2026-07-prealables-preuniversitaires-en-expression`.
fn parse_preuniversitaire(
    doc: &Html,
    anomalies: &mut Vec<ParseError>,
) -> Vec<String> {
    let markers = doc
        .select(&FE_MESSAGE)
        .flat_map(|message| message.text())
        .filter(|node| {
            node.to_lowercase()
                .contains("préuniversitaires nécessaires")
        });

    let mut kept: Vec<String> = Vec::new();
    let mut malformed: Vec<String> = Vec::new();
    for node in markers {
        let extracted = node
            .split_once("nécessaires")
            .and_then(|(_, tail)| tail.split_once(':'))
            .map(|(_, tail)| {
                normalize_connectors(tail.split('.').next().unwrap_or(""))
            });
        let (bucket, expr) = match extracted {
            Some(expr) if contains_sigle(&expr) => (&mut kept, expr),
            // an expression without a single sigle (« voir la direction »),
            // or a marker the extraction cannot even reach: surfaced, never
            // dropped in silence
            Some(expr) => (&mut malformed, expr),
            None => (&mut malformed, node.trim().to_string()),
        };
        if !bucket.contains(&expr) {
            bucket.push(expr);
        }
    }

    for raw in malformed {
        anomalies.push(ParseError::MalformedEntry {
            selector: "préalables préuniversitaires".to_string(),
            raw,
        });
    }
    kept
}

// The source writes its connectors in lowercase prose (« et », « ou »); the
// grammar only knows the uppercase operators. Everything else — sigles,
// parentheses, commas, prose — passes through untouched.
fn normalize_connectors(expr: &str) -> String {
    let words: Vec<&str> = expr
        .split_whitespace()
        .map(|word| match word {
            "et" => "ET",
            "ou" => "OU",
            other => other,
        })
        .collect();
    words.join(" ")
}

// At least one sigle-shaped token — possibly glued to an opening
// parenthesis — must appear for the expression to be worth folding.
fn contains_sigle(expr: &str) -> bool {
    expr.split_whitespace().any(|token| {
        leading_course_code(token.trim_start_matches('(')).is_some()
    })
}

// The leading course code of a token — two to four uppercase letters, a
// hyphen, four digits — or `None`. « MAT-0150.Cette » yields « MAT-0150 »: a
// sigle can be glued to the prose that follows it by a bare period.
fn leading_course_code(token: &str) -> Option<&str> {
    let subject_len =
        token.bytes().take_while(|b| b.is_ascii_uppercase()).count();
    let end = subject_len + 5;
    let is_code = (2..=4).contains(&subject_len)
        && token.as_bytes().get(subject_len) == Some(&b'-')
        && token
            .get(subject_len + 1..end)
            .is_some_and(|number| number.bytes().all(|b| b.is_ascii_digit()));
    is_code.then(|| &token[..end])
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
) -> BTreeMap<Season, SeasonOffering> {
    let mut latest: BTreeMap<Season, SeasonOffering> = BTreeMap::new();

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
        if latest
            .get(&season)
            .is_some_and(|kept| kept.last_offered >= Some(year))
        {
            continue;
        }

        let options = parse_offering(session, &heading, anomalies);
        if !options.is_empty() {
            latest.insert(
                season,
                SeasonOffering {
                    last_offered: Some(year),
                    options: Some(options),
                },
            );
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
) -> Vec<Vec<Section>> {
    let top = top_level_sections(session);

    if advertised_section_count(heading) != Some(top.len()) {
        anomalies.push(ParseError::MalformedEntry {
            selector: "p.controls-title".to_string(),
            raw: format!("{heading}: {} top-level sections found", top.len()),
        });
    }

    top.into_iter()
        .flat_map(|section| enrolment_options(section, anomalies))
        .collect()
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
    use crate::PrereqTree;

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

    // every parsed offering carries a schedule: `options: None` only comes
    // from the new-course synthesis in `parse`, never from `parse_seasons`
    fn options_of(offering: &SeasonOffering) -> &[Vec<Section>] {
        offering.options.as_deref().expect("a parsed schedule")
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
    fn a_page_above_the_second_cycle_is_recognized_then_dropped_whole() {
        // `parse_cycle` returning `None` is unit-tested on its own
        // (`a_course_above_the_second_cycle_is_out_of_scope_not_an_error`);
        // this one drives the *whole page* through it, the way MDD-5101 and
        // PSY-7851 do, so the early `Ok(None)` in `parse` itself — not just
        // in the helper it calls — is exercised without a fixture.
        let html = format!(
            "<html><body>{}{}{}{}</body></html>",
            r#"<span class="fe--titre-type">MDD-5101</span>"#,
            r#"<span class="fe--titre-nom">Résidence postdoctorale</span>"#,
            credits_card("0"),
            cycle_card(&["Études post-MDD"]),
        );

        assert!(
            parse(&html).expect("a well-formed page above scope").is_none(),
            "a post-MDD course must be out of scope"
        );
    }

    #[test]
    fn a_page_without_sessions_is_offered_fall_and_winter_unknown() {
        // The new-course rule (GCI-1011's shape): no session section at all
        // means the schedule is not yet published, not that the course is
        // never offered — Fall and Winter, vintage and schedule unknown
        // (ADR `2026-07-cours-sans-section-de-session-offert-automne-hiver`)
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
        assert_eq!(
            page.course.seasons.keys().collect::<Vec<_>>(),
            [&Season::Fall, &Season::Winter],
        );
        assert!(page.course.seasons.values().all(|offering| {
            offering.last_offered.is_none() && offering.options.is_none()
        }));
        assert!(page.anomalies.is_empty());
    }

    #[test]
    fn a_session_block_yielding_nothing_synthesizes_no_season() {
        // The guard is the *absence* of the section, never an empty parse
        // result: a session block whose heading the parser cannot read left
        // an anomaly, and nothing is invented next to an anomaly.
        let html = format!(
            "<html><body>{}{}{}{}{}</body></html>",
            r#"<span class="fe--titre-type">GEX-4008</span>"#,
            r#"<span class="fe--titre-nom">Approvisionnement</span>"#,
            credits_card("3"),
            cycle_card(&["Premier cycle"]),
            session("Printemps 2026 – 1 section offerte", ""),
        );

        let page = parse(&html)
            .expect("complete page")
            .expect("a first-cycle course is in scope");
        assert!(page.course.seasons.is_empty());
        assert_eq!(page.anomalies.len(), 1, "the dropped block is surfaced");
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
        // to schedule, and `CourseCycle` cannot hold it. Recognized, then
        // dropped on purpose — hence `None` rather than an anomaly, which
        // would fill the log with a case we understand perfectly.
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
    fn a_preuniversitaire_course_is_in_scope() {
        // a « cours d'appoint » (CHM-0150) declares « Préuniversitaire » — a
        // cycle below the first, which `CourseCycle` holds and the périmètre
        // keeps (ADR `2026-07-cours-dappoint-reintegres`)
        let doc = document(&cycle_card(&["Préuniversitaire"]));
        assert_eq!(
            parse_cycle(&doc).expect("cycle"),
            Some(CourseCycle::Preuniversity)
        );
    }

    #[test]
    fn the_lowest_listed_cycle_wins() {
        let doc =
            document(&cycle_card(&["Troisième cycle", "Deuxième cycle"]));
        assert_eq!(
            parse_cycle(&doc).expect("cycle"),
            Some(CourseCycle::Second)
        );
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
    fn preuniversitaire_prerequisites_merge_into_the_tree() {
        // « Préalables préuniversitaires nécessaires s'il y a lieu : … » sits
        // in a .fe--message box the regular selector never reads. The marker
        // and its sigles share one text node; the following prose is a
        // sibling node, so reading the element whole would glue
        // « PHY-0250.Cette » and lose the second sigle (gml-1001).
        let doc = document(
            r#"<div class="fe--message"><p>Préalables préuniversitaires nécessaires s'il y a lieu : CHM-0150 et PHY-0250.</p><p>Cette section de cours est offerte à distance.</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "CHM-0150 ET PHY-0250".to_string(),
                tree: all(vec![course("CHM-0150"), course("PHY-0250")]),
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn regular_and_preuniversitaire_prerequisites_combine_with_et() {
        // a regular préalable and a préuniversitaire one become a single
        // « (<régulier>) ET (<préuniv>) » expression, folded by the existing
        // grammar like any other course
        let doc = document(
            r#"<div class="fe--prealables"><p class="etiquette-container">GAE-1004 ET GAE-2000</p></div><div class="fe--message"><p>Préalables préuniversitaires nécessaires s'il y a lieu : CHM-0150 et PHY-0250.</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "(GAE-1004 ET GAE-2000) ET (CHM-0150 ET PHY-0250)"
                    .to_string(),
                tree: all(vec![
                    all(vec![course("GAE-1004"), course("GAE-2000")]),
                    all(vec![course("CHM-0150"), course("PHY-0250")]),
                ]),
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_preuniversitaire_marker_without_a_sigle_is_an_anomaly() {
        // the marker is there but no sigle comes out: surfaced as an anomaly,
        // never dropped in silence — and only once, however many sections
        // repeat the same message
        let doc = document(
            r#"<div class="fe--message"><p>Préalables préuniversitaires nécessaires s'il y a lieu : voir la direction.</p></div><div class="fe--message"><p>Préalables préuniversitaires nécessaires s'il y a lieu : voir la direction.</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(parse_prerequisites(&doc, &mut anomalies), None);
        assert_eq!(anomalies.len(), 1, "got {anomalies:?}");
        assert_eq!(
            malformed_entry(&anomalies[0]),
            ("préalables préuniversitaires", "voir la direction")
        );
    }

    #[test]
    fn a_marker_whose_expression_is_unreachable_is_an_anomaly() {
        // a marker node with no colon after « nécessaires » gives the
        // extraction nothing to anchor on: the whole node is surfaced (the
        // old walk returned nothing here, silently)
        let doc = document(
            r#"<div class="fe--message"><p>Préalables préuniversitaires nécessaires BIO-0150</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(parse_prerequisites(&doc, &mut anomalies), None);
        assert_eq!(anomalies.len(), 1, "got {anomalies:?}");
        assert_eq!(
            malformed_entry(&anomalies[0]),
            (
                "préalables préuniversitaires",
                "Préalables préuniversitaires nécessaires BIO-0150"
            )
        );
    }

    #[test]
    fn a_remarque_prefix_does_not_swallow_the_expression() {
        // bio-1003: « REMARQUE : » carries its own colon before the marker's,
        // so the expression starts at the colon *after* « nécessaires ». The
        // comma list is outside the grammar and survives as a Raw operand;
        // the « ou » alternative that follows it stays checkable — the old
        // walk dropped CHM-0170 without a trace.
        let doc = document(
            r#"<div class="fe--message"><p>REMARQUE : Préalables préuniversitaires nécessaires s'il y a lieu : BIO-0150, CHM-0150, CHM-0160 ou CHM-0170.</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "BIO-0150, CHM-0150, CHM-0160 OU CHM-0170".to_string(),
                tree: any(vec![
                    PrereqTree::Raw {
                        raw: "BIO-0150, CHM-0150, CHM-0160".to_string()
                    },
                    course("CHM-0170"),
                ]),
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn distinct_preuniversitaire_messages_all_fold_in() {
        // bio-1003 again: its sections carry two *different* messages — an
        // ambiguous comma list and a fully parenthesized expression. Repeats
        // dedupe; the distinct survivors are ET-joined so neither is lost.
        let doc = document(
            r#"<div class="fe--message"><p>REMARQUE : Préalables préuniversitaires nécessaires s'il y a lieu : BIO-0150, CHM-0150, CHM-0160 ou CHM-0170.</p></div><div class="fe--message"><p>REMARQUE : Préalables préuniversitaires nécessaires s'il y a lieu : BIO-0150, CHM-0150, CHM-0160 ou CHM-0170.</p></div><div class="fe--message"><p>Préalables préuniversitaires nécessaires : (BIO-0150 ou BIO-NYA ou équivalent) ET (CHM-0160 ou CHM-0170 ou CHM-NYB ou équivalent).</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "(BIO-0150, CHM-0150, CHM-0160 OU CHM-0170) ET \
                      ((BIO-0150 OU BIO-NYA OU équivalent) ET \
                      (CHM-0160 OU CHM-0170 OU CHM-NYB OU équivalent))"
                    .to_string(),
                tree: all(vec![
                    any(vec![
                        PrereqTree::Raw {
                            raw: "BIO-0150, CHM-0150, CHM-0160".to_string()
                        },
                        course("CHM-0170"),
                    ]),
                    all(vec![
                        any(vec![
                            course("BIO-0150"),
                            PrereqTree::Raw {
                                raw: "BIO-NYA".to_string()
                            },
                            PrereqTree::Raw {
                                raw: "équivalent".to_string()
                            },
                        ]),
                        any(vec![
                            course("CHM-0160"),
                            course("CHM-0170"),
                            PrereqTree::Raw {
                                raw: "CHM-NYB".to_string()
                            },
                            PrereqTree::Raw {
                                raw: "équivalent".to_string()
                            },
                        ]),
                    ]),
                ]),
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn a_preuniversitaire_sigle_glued_to_prose_is_still_read() {
        // ift-1903 / mat-1200: the marker, the sigle and the prose share one
        // text node, the sigle glued to the prose by a bare period (no
        // separator) — « MAT-0150.Cette section… ». Its leading code is read,
        // and the prose that follows is not mistaken for a préalable.
        let doc = document(
            r#"<div class="fe--message"><p>Préalables préuniversitaires nécessaires s'il y a lieu : MAT-0150.Cette section de cours est offerte à distance.</p></div>"#,
        );
        let mut anomalies = Vec::new();

        assert_eq!(
            parse_prerequisites(&doc, &mut anomalies),
            Some(Prerequisites::Parsed {
                raw: "MAT-0150".to_string(),
                tree: course("MAT-0150"),
            })
        );
        assert!(anomalies.is_empty(), "got {anomalies:?}");
    }

    #[test]
    fn an_unrelated_message_leaves_the_prerequisites_untouched() {
        // a .fe--message that is not the préuniversitaire marker must not be
        // read as one: the regular préalable stays exactly as before
        let doc = document(
            r#"<div class="fe--prealables"><p class="etiquette-container">GAE-1004 ET GAE-2000</p></div><div class="fe--message"><p>Cette section de cours est offerte à distance.</p></div>"#,
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
            let offering = &seasons[&Season::Fall];
            assert_eq!(
                offering.last_offered,
                Some(2026),
                "the 2026 session wins ({order})"
            );
            assert_eq!(
                options_of(offering)[0][0].nrc,
                "22222",
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
        assert_eq!(options_of(&seasons[&Season::Winter]).len(), 1);
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
            assert_eq!(options_of(&seasons[&Season::Fall]).len(), 1);
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

        let nrcs: Vec<Vec<&str>> = options_of(&seasons[&Season::Fall])
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

        let nrcs: Vec<&str> = options_of(&seasons[&Season::Fall])
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
        let section = &options_of(&seasons[&Season::Fall])[0][0];
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

    fn course(code: &str) -> PrereqTree {
        PrereqTree::Course(code.to_string())
    }

    fn all(trees: Vec<PrereqTree>) -> PrereqTree {
        PrereqTree::All { all: trees }
    }

    fn any(trees: Vec<PrereqTree>) -> PrereqTree {
        PrereqTree::Any { any: trees }
    }
}
