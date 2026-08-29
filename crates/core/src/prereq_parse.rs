// The préalables grammar — « GCI-1000 ET (MAT-1900 OU MAT-1902) » — turned
// into a `PrereqTree`. It lives in core, not in the scraper that first read
// it: the same grammar serves a student rewriting a préalable his own
// program vintage never had (ADR
// `2026-08-parseur-de-prealables-deplace-dans-core`).
use crate::course::{PrereqTree, ProgramCredits};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Malformed prerequisites {error}: {raw}")]
pub struct PrereqParseError {
    pub error: String,
    pub raw: String,
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

// Only a broken structure — an unclosed group, an operator missing an
// operand — can fail: it has no local repair, nothing says which operands
// the group was meant to hold. Everything else ends up in the tree.
pub fn parse_prereq_tree(raw: &str) -> Result<PrereqTree, PrereqParseError> {
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
// is padded first because the source glues it to the sigle; the `*` stays
// glued, it is the operand's own mark of concomitance.
fn tokenize_prereq_raw(raw: &str) -> Vec<PrereqToken> {
    let padded = raw.replace('(', " ( ").replace(')', " ) ");
    let words: Vec<&str> = padded.split_whitespace().collect();
    let words = group_enumerated_sigles(&words);
    let mut tokens: Vec<PrereqToken> = Vec::new();
    let mut operand: Vec<&str> = Vec::new();

    for word in words {
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

// The répertoire enumerates sigles with commas and closes the run with a
// single connector — « MAT-0130, MAT-0150 ET MAT-0260 » (MAT-1900),
// « CHM-0150, CHM-0160 OU CHM-0170 » (CHM-1003) — and that last connector
// governs the whole enumeration (ADR
// `2026-08-virgule-selon-le-connecteur-final`). The run is rewritten into
// the group it means — « ( MAT-0130 ET MAT-0150 ET MAT-0260 ) » — before a
// single operand is classified: the parenthesis already says exactly what
// the décision asks, that the enumeration is *one* operand of the
// surrounding ET/OU precedence (CHM-1901's « CHM-0150, CHM-0160 OU
// CHM-0170 ET PHY-0150 » is a OU of three, and *then* ET PHY-0150).
fn group_enumerated_sigles<'a>(words: &[&'a str]) -> Vec<&'a str> {
    let mut grouped: Vec<&'a str> = Vec::with_capacity(words.len());
    let mut index = 0;

    // one pass per word at most: each turn consumes at least one word
    for _ in 0..words.len() {
        let Some(word) = words.get(index) else {
            break;
        };
        match enumeration_at(words, index) {
            Some(enumeration) => {
                index += enumeration.consumed;
                grouped.extend(enumeration.words);
            }
            None => {
                index += 1;
                grouped.push(word);
            }
        }
    }

    grouped
}

// the words a recognized enumeration becomes, and how many it replaces
struct Enumeration<'a> {
    words: Vec<&'a str>,
    consumed: usize,
}

// A comma is read only in the one shape the décision covers: a run of
// sigles each closed by a comma, then the sigle the last comma introduces,
// then the connector that fixes the run's meaning, then the operand it
// joins. Anything else keeps its commas in the operand's own text —
// « Réussir 2 parmi CTB-6112, CTB-6116 » (CTB-6113) enumerates behind prose
// no grammar reads, and « BIO-0150, CHM-0150, CHM-0160 » (BCM-1903) closes
// on no connector at all, so neither is interpreted.
fn enumeration_at<'a>(
    words: &[&'a str],
    start: usize,
) -> Option<Enumeration<'a>> {
    // an enumeration is a whole operand, so it starts where one can: at the
    // beginning of the expression, or right after a separator
    if start > 0 && !is_separator(words[start - 1]) {
        return None;
    }
    let listed: Vec<&'a str> = words[start..]
        .iter()
        .map_while(|word| word.strip_suffix(',').filter(|w| is_sigle(w)))
        .collect();
    if listed.is_empty() {
        return None;
    }
    let last = words
        .get(start + listed.len())
        .copied()
        .filter(|word| is_sigle(word))?;
    let connector = words
        .get(start + listed.len() + 1)
        .copied()
        .filter(|word| matches!(*word, "ET" | "OU"))?;
    // the operand the connector joins to the enumeration, read to the next
    // separator like any other. An empty run — the connector opening a
    // group, or ending the text — leaves the enumeration unclosed, hence
    // uninterpreted.
    let joined_at = start + listed.len() + 2;
    let joined: Vec<&'a str> = words
        .get(joined_at..)
        .unwrap_or_default()
        .iter()
        .copied()
        .take_while(|word| !is_separator(word))
        .collect();
    if joined.is_empty() {
        return None;
    }

    let mut grouped = Vec::with_capacity(2 * listed.len() + joined.len() + 5);
    grouped.push("(");
    for item in listed.iter().copied().chain(std::iter::once(last)) {
        grouped.push(item);
        grouped.push(connector);
    }
    grouped.extend(joined.iter().copied());
    grouped.push(")");

    Some(Enumeration {
        consumed: joined_at + joined.len() - start,
        words: grouped,
    })
}

// « MAT-0260 » or « MAT-0260* » : one sigle, star included, and nothing
// else — the star is left glued for `checkable_operand` to read
fn is_sigle(word: &str) -> bool {
    is_course_code(word.strip_suffix('*').unwrap_or(word))
}

fn is_separator(word: &str) -> bool {
    matches!(word, "(" | ")" | "ET" | "OU")
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
        // « GCI-2010* » : the répertoire's mark for « peut être suivi en
        // concomitance ». A star anywhere else — on an operand no shape
        // reads, « IFT 10426* » (MAT-2910) — is not stripped either: the
        // operand keeps it verbatim in its raw text.
        [code] => match code.strip_suffix('*') {
            Some(sigle) if is_course_code(sigle) => {
                Some(PrereqTree::Concomitant {
                    concomitant: sigle.to_string(),
                })
            }
            None if is_course_code(code) => {
                Some(PrereqTree::Course(code.to_string()))
            }
            _ => None,
        },
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

fn malformed_prereq(error: &str, raw: &str) -> PrereqParseError {
    PrereqParseError {
        error: error.to_string(),
        raw: raw.to_string(),
    }
}

fn is_program_code(word: &str) -> bool {
    word.len() == 3 && word.chars().all(|c| c.is_ascii_uppercase())
}

// Public because the scraper reads the same shape out of the equivalents
// cards, where a card holding anything else is malformed markup.
pub fn is_course_code(word: &str) -> bool {
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

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
    fn a_starred_sigle_is_a_concomitant_leaf() {
        // « * Indique un préalable qui peut être suivi simultanément »
        // (GEX-3001, ACT-1002) : the star belongs to its own operand, and
        // an operand without one keeps meaning « strictement avant »
        assert_eq!(
            parse_prereq_tree("GCI-2010*")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            PrereqTree::Concomitant {
                concomitant: "GCI-2010".to_string()
            }
        );
        assert_eq!(
            parse_prereq_tree("(ACT-1003* OU MAT-1110)")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            any(vec![
                PrereqTree::Concomitant {
                    concomitant: "ACT-1003".to_string()
                },
                course("MAT-1110"),
            ])
        );
    }

    #[test]
    fn a_star_on_an_operand_no_shape_reads_stays_in_its_text() {
        // MAT-2910 lists « IFT 10426* », a sigle the source mistyped and
        // starred: nothing is stripped on a guess, the operand keeps the
        // star the student has to judge
        assert_kept_as_text("IFT 10426*");
    }

    fn concomitant(code: &str) -> PrereqTree {
        PrereqTree::Concomitant {
            concomitant: code.to_string(),
        }
    }

    #[test]
    fn a_comma_enumeration_takes_the_meaning_of_its_closing_connector() {
        // the décision of 2026-08-29: the last separator governs the whole
        // run (ADR `2026-08-virgule-selon-le-connecteur-final`)
        assert_eq!(
            parse_prereq_tree("MAT-0130, MAT-0150 ET MAT-0260")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                course("MAT-0130"),
                course("MAT-0150"),
                course("MAT-0260"),
            ]),
            "MAT-1900"
        );
        assert_eq!(
            parse_prereq_tree(
                "MAT-0130, MAT-0150, MAT-0260, PHY-0150 ET PHY-0250"
            )
            .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                course("MAT-0130"),
                course("MAT-0150"),
                course("MAT-0260"),
                course("PHY-0150"),
                course("PHY-0250"),
            ]),
            "GMN-2000"
        );
        assert_eq!(
            parse_prereq_tree("BIO-0150, CHM-0150, CHM-0160 OU CHM-0170")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            any(vec![
                course("BIO-0150"),
                course("CHM-0150"),
                course("CHM-0160"),
                course("CHM-0170"),
            ]),
            "BCM-1001"
        );
    }

    #[test]
    fn an_enumeration_is_one_operand_of_the_surrounding_precedence() {
        // CHM-1901: the run closes on OU, and the group it forms is what
        // « ET PHY-0150 » then binds to — not the last sigle alone
        assert_eq!(
            parse_prereq_tree("CHM-0150, CHM-0160 OU CHM-0170 ET PHY-0150")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                any(vec![
                    course("CHM-0150"),
                    course("CHM-0160"),
                    course("CHM-0170"),
                ]),
                course("PHY-0150"),
            ])
        );
    }

    #[test]
    fn an_enumerated_sigle_keeps_its_own_star() {
        // GMC-1001 stars the third item of the run: the enumeration groups
        // the operands, it does not rewrite them
        assert_eq!(
            parse_prereq_tree("MAT-0130, MAT-0150, MAT-0260* ET PHY-0150")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                course("MAT-0130"),
                course("MAT-0150"),
                concomitant("MAT-0260"),
                course("PHY-0150"),
            ])
        );
    }

    #[test]
    fn an_enumeration_reads_the_same_inside_a_group() {
        // GEL-1000 and BIO-1003 both parenthesize the run
        assert_eq!(
            parse_prereq_tree(
                "(MAT-1900* OU PHY-1002*) ET (MAT-0130, MAT-0150, MAT-0260 ET PHY-0250)"
            )
            .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                any(vec![
                    concomitant("MAT-1900"),
                    concomitant("PHY-1002"),
                ]),
                all(vec![
                    course("MAT-0130"),
                    course("MAT-0150"),
                    course("MAT-0260"),
                    course("PHY-0250"),
                ]),
            ]),
            "GEL-1000"
        );
        assert_eq!(
            parse_prereq_tree(
                "(BIO-0150, CHM-0150, CHM-0160 OU CHM-0170) ET GCI-1000"
            )
            .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                any(vec![
                    course("BIO-0150"),
                    course("CHM-0150"),
                    course("CHM-0160"),
                    course("CHM-0170"),
                ]),
                course("GCI-1000"),
            ]),
            "BIO-1003"
        );
    }

    #[test]
    fn an_enumeration_no_connector_closes_stays_verbatim() {
        // BCM-1903 and CHM-1001 stop on the last sigle: « A, B, C » alone
        // says neither ET nor OU, and guessing one would invent a
        // requirement the répertoire never wrote
        assert_kept_as_text("BIO-0150, CHM-0150, CHM-0160");
        assert_kept_as_text("CHM-0150, CHM-0160");
        // a run the text ends on, and one a group follows: neither closes
        assert!(parse_prereq_tree("MAT-0130, MAT-0150 ET").is_err());
        assert_eq!(
            parse_prereq_tree("MAT-0130, MAT-0150 ET (GCI-1000 OU GCI-2000)")
                .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                PrereqTree::Raw {
                    raw: "MAT-0130, MAT-0150".to_string()
                },
                any(vec![course("GCI-1000"), course("GCI-2000")]),
            ])
        );
    }

    #[test]
    fn a_comma_inside_prose_is_never_read_as_an_enumeration() {
        // CTB-6113 reads « Réussir 2 parmi CTB-6112, CTB-6116, … »: the
        // commas sit behind prose no grammar covers, so the operand keeps
        // them — the « N parmi » form is not the décision's subject
        assert_kept_as_text("Réussir 2 parmi CTB-6112, CTB-6116");
        assert_eq!(
            parse_prereq_tree(
                "Réussir 2 parmi CTB-6112, CTB-6116 ET GCI-1000"
            )
            .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                PrereqTree::Raw {
                    raw: "Réussir 2 parmi CTB-6112, CTB-6116".to_string()
                },
                course("GCI-1000"),
            ])
        );
        // a run whose last item is not a sigle is not an enumeration
        // either, comma or no comma
        assert_kept_as_text("MAT-0130, Examen de langue");
        assert_kept_as_text("MAT-0130,");
        assert_kept_as_text("MAT-0130, MAT-0150 MAT-0260");
    }

    #[test]
    fn the_operand_an_enumeration_joins_is_read_whole() {
        // the connector closes the run on whatever operand follows it, read
        // to the next separator like any other — a credits requirement
        // here, five words long
        assert_eq!(
            parse_prereq_tree(
                "MAT-0130, MAT-0150 ET GEX, Crédits exigés : 45"
            )
            .unwrap_or_else(|e| panic!("parse: {e}")),
            all(vec![
                course("MAT-0130"),
                course("MAT-0150"),
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
                    Err(PrereqParseError { raw: got, .. })
                        if got.contains(raw)
                ),
                "expected a malformed-prerequisites error for {raw:?}, got {result:?}"
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
                Err(PrereqParseError { error, .. }) => {
                    assert_eq!(
                        error, expected_error,
                        "wrong error label for {raw:?}"
                    );
                }
                other => panic!(
                    "expected a malformed-prerequisites error for {raw:?}, got {other:?}"
                ),
            }
        }
    }
}
