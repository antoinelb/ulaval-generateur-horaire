use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;

use crate::course::{
    is_preuniversity, Course, PrereqTree, Prerequisites, Season,
};
use crate::feasibility::{FeasibilityCache, MAX_MEMBERS};
use crate::week::WeekMask;
use crate::weekly::build_domain;

// far above any real prerequisite tree; bounds the flatten loop
const MAX_TREE_NODES: usize = 10_000;

// Solver B's input (ADR `2026-07-schema-des-fixtures-de-placement`): a
// *given* course list — B never picks courses — sessions as an ordered
// list of seasons, and the student's constraints as domain reductions.
// `seed` is a value-ordering hint only (the reference cheminement_type):
// it shapes the search order so the first solution resembles the reference
// path, never the solution set — codes it names that are absent from
// `courses` are ignored (a full-bac seed over a partial list is normal).
#[derive(Debug, Clone, Copy)]
pub struct PlacementRequest<'a> {
    pub sessions: &'a [Season],
    pub credit_cap: u32,
    pub concomitant: bool,
    pub courses: &'a [Course],
    pub passed: &'a BTreeSet<String>,
    pub pinned: &'a BTreeMap<String, usize>,
    // codes whose domain is restricted to the Summer sessions of the
    // horizon — unless pinned: a pin is an explicit act and lifts the
    // restriction (ADR `2026-08-stage-place-en-ete-sauf-epinglage`)
    pub stages: &'a BTreeSet<String>,
    // 1-based indices (the `pinned` convention) of the étés open to
    // regular courses; an été absent from the set accepts only stages and
    // pinned courses (ADR `2026-08-stage-place-en-ete-sauf-epinglage`)
    pub open_summers: &'a BTreeSet<usize>,
    pub seed: &'a BTreeMap<String, usize>,
    // the double bound (ADR `2026-07-budget-de-b-en-double-borne`): work
    // (expanded partial assignments) and memory (returned solutions)
    pub max_nodes: u64,
    pub max_solutions: usize,
    // « remplir au mieux » : each candidate gains the sentinel session 0,
    // « not placed », tried last — so a leaf exists even when no complete
    // arrangement does. Every placed course still honours every constraint;
    // relaxing means leaving holes, never filling a slot with a violation
    // (ADR `2026-08-placement-au-mieux-en-repli`). `verify` and
    // `admissible_sessions` never set it: proving stays proving.
    pub allow_unplaced: bool,
    // Internal proposal/diagnostic fallback. Course prerequisites remain
    // strict; only a final `ProgramCredits` leaf may be recorded short.
    pub allow_credit_shortfall: bool,
}

// the domain value meaning « this course is not placed » — sessions are
// 1-based everywhere (`pinned`, `placement`, `domain`), so 0 is free
const UNPLACED: usize = 0;

// All feasible placements found, in search order, with the three outcomes
// never confused (ADR `2026-07-b-enumere-toutes-les-solutions`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Placement {
    pub completion: Completion,
    pub solutions: Vec<Solution>,
    // courses proven unplaceable before the search — a non-empty list is
    // an infeasibility proof that names its culprits (ADR
    // `2026-07-implacabilite-prouvee-avant-la-recherche`)
    pub blocked: Vec<Blocked>,
}

// A candidate no assignment can ever place, and why — surfaced so the
// harness and the UI name the culprit instead of grinding the node budget
// on an unwinnable enumeration.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Blocked {
    pub code: String,
    pub reason: BlockedReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "kebab-case")]
pub enum BlockedReason {
    // no listed session can host the course (offer and pin filtering
    // left nothing)
    EmptyDomain,
    // the prerequisite tree is False under every assignment — an unknown
    // university code, the course as its own prerequisite, or a credits
    // threshold above every credit in sight
    UnsatisfiablePrerequisites,
    // an unpinned stage found no été to land in — the horizon has no
    // summer session or none offers the stage; pinning it lifts the
    // restriction (ADR `2026-08-stage-place-en-ete-sauf-epinglage`)
    StageWithoutSummer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "kebab-case")]
pub enum Completion {
    // search exhausted: the set is total — empty means infeasibility proven
    Complete,
    // work bound hit: the set is partial, never « infeasible »
    NodeBudget,
    // memory bound hit: the set is partial, never « infeasible »
    SolutionCap,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Solution {
    // code → 1-based session number; passed courses do not appear.
    // `Record`: json_compatible serializes a map to a plain object
    #[cfg_attr(feature = "tsify", tsify(type = "Record<string, number>"))]
    pub placement: BTreeMap<String, usize>,
    // unverifiable operands this placement's verdict relied on — a raw
    // operand or a course neither listed nor passed, presumed satisfied
    // and surfaced, never imposed (ADR
    // `2026-07-prealable-inconnu-non-bloquant-remonte`)
    pub assumed: BTreeSet<String>,
    // codes this solution could not seat anywhere — only ever non-empty
    // under `allow_unplaced`, where a hole is the honest answer and a
    // violated slot would be a lie (ADR
    // `2026-08-placement-au-mieux-en-repli`). Named `left_out` and not
    // « unplaced » because the UI's `unplaced_codes` already means
    // something else: the plan's floating courses, whoever left them so.
    pub left_out: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credit_shortfalls: Vec<CreditShortfall>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct CreditShortfall {
    pub code: String,
    pub session: usize,
    pub earned_before: u32,
    pub required: u32,
}

// The static answer of `prerequisites_met` : met against what the student
// already holds, or not — `assumed` carries the operands the verdict had
// to presume (raw text, préuniversitaire codes), surfaced, never imposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrereqStatus {
    Met { assumed: BTreeSet<String> },
    Unmet,
}

// Inputs the placement refuses to guess about — surfaced, never invented.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlacementError {
    #[error("a placement needs at least one session and one course")]
    EmptyRequest,
    #[error("duplicated course code : {code}")]
    DuplicateCourse { code: String },
    #[error("{code} is passed or pinned but has no Course in the request")]
    UnknownCode { code: String },
    #[error("{code} cannot be both passed and pinned")]
    PassedAndPinned { code: String },
    #[error("{code} is pinned to session {session}, outside 1..={sessions}")]
    PinnedOutOfRange {
        code: String,
        session: usize,
        sessions: usize,
    },
    #[error("{code} is declared a stage but has no Course in the request")]
    StageWithoutCourse { code: String },
    #[error("open summer {session} is outside 1..={sessions}")]
    OpenSummerOutOfRange { session: usize, sessions: usize },
    #[error(
        "session {session} is open as an été but is a {season:?} session"
    )]
    OpenSummerNotSummer { session: usize, season: Season },
    #[error("the prerequisite tree of {code} exceeds {MAX_TREE_NODES} nodes")]
    PrerequisiteTreeTooLarge { code: String },
    #[error(
        "{count} courses to place exceed the supported {MAX_MEMBERS} \
         (the feasibility cache packs them in a single bitset)"
    )]
    TooManyCourses { count: usize },
}

// a course to place: its planning credits, its prerequisite tree flattened
// once, its value-ordered session domain, and its weekly option masks per
// offered season — precomputed once so the search's hot path never builds
// a domain or touches an option's NRC strings again
struct Candidate {
    code: String,
    credits: u32,
    tree: Option<FlatTree>,
    domain: Vec<usize>,
    masks: BTreeMap<Season, Vec<WeekMask>>,
}

// A prerequisite tree flattened breadth-first: children always sit after
// their parent, so one reverse scan evaluates children before parents —
// no recursion, no unbounded loop.
struct FlatTree {
    nodes: Vec<FlatNode>,
}

enum FlatNode {
    Course(String),
    Raw(String),
    Credits(u32),
    All(Vec<usize>),
    Any(Vec<usize>),
}

// Three-valued verdict of a (partial) assignment against a tree.
// `Sat` carries the unverifiable operands the verdict relied on — empty
// means proven from placed/passed facts alone. `False` is proven violated
// and *permanent* under extension (placements are only ever added, never
// moved), which is what makes pruning on it sound. `Unknown` may still
// turn either way with further placements.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Verdict {
    Sat(BTreeSet<String>),
    False,
    Unknown,
}

// everything immutable the search consults, built once
struct SearchCtx<'a> {
    request: &'a PlacementRequest<'a>,
    candidates: &'a [Candidate],
    // candidate index by code, for prerequisite leaves
    index_of: BTreeMap<String, usize>,
    // which candidates mention a code in their tree: placing that code
    // re-checks exactly these, nothing else
    referenced_by: BTreeMap<String, Vec<usize>>,
    // whether `finalize` must re-evaluate the candidate's tree: only a
    // `Credits` threshold resolves at the leaf, and only a `Raw` or an
    // assumed préuniversitaire leaf contributes operands — every other
    // tree's last incremental verdict is already final and operand-free
    needs_final: Vec<bool>,
    // suffix_credits[d] = credits of candidates d.. still to place — the
    // O(1) side of the remaining-credits bound in `expand`
    suffix_credits: Vec<u64>,
    passed_credits: u32,
    // candidates whose tree carries a « Crédits exigés » leaf: their
    // verdict depends on *every* assignment, not just on codes they
    // mention, so `referenced_by` alone would never re-check them — each
    // extension does (the potential bound in `credits_leaf` turns a
    // doomed threshold into a prune instead of a leaf-by-leaf rejection)
    credit_watch: Vec<usize>,
}

// one tree evaluation: candidate `evaluated`'s tree against `chosen`
struct EvalCtx<'a> {
    ctx: &'a SearchCtx<'a>,
    chosen: &'a [usize],
    evaluated: usize,
    complete: bool,
}

// B's placement search: systematic, complete, prune-never-repair. Returns
// every feasible placement found within the two bounds, in search order.
pub fn place(request: &PlacementRequest) -> Result<Placement, PlacementError> {
    validate(request)?;
    let candidates = build_candidates(request)?;
    if candidates.len() > MAX_MEMBERS {
        return Err(PlacementError::TooManyCourses {
            count: candidates.len(),
        });
    }
    let index_of: BTreeMap<String, usize> = candidates
        .iter()
        .enumerate()
        .map(|(i, candidate)| (candidate.code.clone(), i))
        .collect();
    let ctx = SearchCtx {
        request,
        candidates: &candidates,
        needs_final: needs_final_flags(&candidates, &index_of, request),
        suffix_credits: suffix_credit_sums(&candidates),
        index_of,
        referenced_by: referencing_map(&candidates),
        passed_credits: passed_credits(request),
        credit_watch: candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                candidate.tree.as_ref().is_some_and(|tree| {
                    tree.nodes
                        .iter()
                        .any(|node| matches!(node, FlatNode::Credits(_)))
                })
            })
            .map(|(index, _)| index)
            .collect(),
    };
    let blocked = blocked_candidates(&ctx);
    // relaxed, the screen stops short-circuiting and becomes pure report:
    // a candidate it names has only the sentinel left as a child anyway —
    // an empty domain offers nothing else, and an unsatisfiable tree has
    // every real child pruned by `precedence_admits`
    if !blocked.is_empty() && !request.allow_unplaced {
        return Ok(Placement {
            completion: Completion::Complete,
            solutions: Vec::new(),
            blocked,
        });
    }
    Ok(Placement {
        blocked,
        ..search(&ctx)
    })
}

// The static prerequisite question the UI asks per course row (jalon 6,
// « préalables non remplis ») : against what the student already holds —
// `satisfied` (passed and placed-before codes) and the credits accumulated
// so far — never against a hypothetical future placement. Leaf semantics
// mirror the search's: a code neither held nor préuniversitaire blocks
// (ADR `2026-07-presomption-limitee-au-preuniversitaire`); a raw operand
// or a presumed préuniversitaire code lands in `assumed`.
pub fn prerequisites_met(
    course: &Course,
    satisfied: &BTreeSet<String>,
    credits: u32,
) -> Result<PrereqStatus, PlacementError> {
    let Some(tree) = flat_tree(course)? else {
        return Ok(PrereqStatus::Met {
            assumed: BTreeSet::new(),
        });
    };
    let mut verdicts = vec![Verdict::Unknown; tree.nodes.len()];
    for i in (0..tree.nodes.len()).rev() {
        let verdict = match &tree.nodes[i] {
            FlatNode::Course(code) => static_course_leaf(code, satisfied),
            FlatNode::Raw(raw) => {
                Verdict::Sat(std::iter::once(raw.clone()).collect())
            }
            FlatNode::Credits(threshold) if credits >= *threshold => {
                Verdict::Sat(BTreeSet::new())
            }
            FlatNode::Credits(_) => Verdict::False,
            FlatNode::All(children) => all_verdict(children, &verdicts),
            FlatNode::Any(children) => any_verdict(children, &verdicts),
        };
        verdicts[i] = verdict;
    }
    match verdicts.into_iter().next().unwrap_or(Verdict::Unknown) {
        Verdict::Sat(assumed) => Ok(PrereqStatus::Met { assumed }),
        // static leaves never yield Unknown; rejecting is the safe reading
        // if one ever did
        Verdict::False | Verdict::Unknown => Ok(PrereqStatus::Unmet),
    }
}

// no candidate list here: a code is either held, presumed (préuniversitaire
// only), or a real university course still to take — which blocks
fn static_course_leaf(code: &str, satisfied: &BTreeSet<String>) -> Verdict {
    if satisfied.contains(code) {
        Verdict::Sat(BTreeSet::new())
    } else if is_preuniversity(code) {
        Verdict::Sat(std::iter::once(code.to_string()).collect())
    } else {
        Verdict::False
    }
}

// Which sessions could host `code` if the student pinned it there, the
// rest of the request held as given (the « + H28 » chips, jalons 6–7).
// One probe of `place` per session : a pin is exactly what accepting the
// chip would do, so pin semantics — offer intersection, summer restriction
// lifted — answer the very question the click asks. With every other
// course pinned, each probe validates a single assignment instead of
// searching; an unpinned course left in the request stays movable, and
// the probe then answers « could it fit with the others rearranged ».
pub fn admissible_sessions(
    request: &PlacementRequest,
    code: &str,
) -> Result<BTreeSet<usize>, PlacementError> {
    let mut admissible = BTreeSet::new();
    for session in 1..=request.sessions.len() {
        let mut pinned = request.pinned.clone();
        pinned.insert(code.to_string(), session);
        let probe = PlacementRequest {
            pinned: &pinned,
            // existence is the whole question
            max_solutions: 1,
            // a relaxed probe would call every session admissible by
            // leaving all the others out — the chip must mean « it fits »
            allow_unplaced: false,
            allow_credit_shortfall: false,
            ..*request
        };
        if !place(&probe)?.solutions.is_empty() {
            admissible.insert(session);
        }
    }
    Ok(admissible)
}

fn validate(request: &PlacementRequest) -> Result<(), PlacementError> {
    if request.sessions.is_empty() || request.courses.is_empty() {
        return Err(PlacementError::EmptyRequest);
    }
    let mut codes: BTreeSet<&str> = BTreeSet::new();
    if let Some(course) = request
        .courses
        .iter()
        .find(|course| !codes.insert(course.code.as_str()))
    {
        return Err(PlacementError::DuplicateCourse {
            code: course.code.clone(),
        });
    }
    if let Some(code) = request
        .passed
        .iter()
        .chain(request.pinned.keys())
        .find(|code| !codes.contains(code.as_str()))
    {
        return Err(PlacementError::UnknownCode { code: code.clone() });
    }
    if let Some(code) = request
        .passed
        .iter()
        .find(|code| request.pinned.contains_key(*code))
    {
        return Err(PlacementError::PassedAndPinned { code: code.clone() });
    }
    if let Some((code, &session)) = request
        .pinned
        .iter()
        .find(|(_, &session)| session < 1 || session > request.sessions.len())
    {
        return Err(PlacementError::PinnedOutOfRange {
            code: code.clone(),
            session,
            sessions: request.sessions.len(),
        });
    }
    // a stage that is also passed is allowed — it never becomes a candidate
    if let Some(code) = request
        .stages
        .iter()
        .find(|code| !codes.contains(code.as_str()))
    {
        return Err(PlacementError::StageWithoutCourse { code: code.clone() });
    }
    if let Some(&session) = request
        .open_summers
        .iter()
        .find(|&&session| session < 1 || session > request.sessions.len())
    {
        return Err(PlacementError::OpenSummerOutOfRange {
            session,
            sessions: request.sessions.len(),
        });
    }
    if let Some(&session) = request
        .open_summers
        .iter()
        .find(|&&session| request.sessions[session - 1] != Season::Summer)
    {
        return Err(PlacementError::OpenSummerNotSummer {
            session,
            season: request.sessions[session - 1],
        });
    }
    Ok(())
}

fn build_candidates(
    request: &PlacementRequest,
) -> Result<Vec<Candidate>, PlacementError> {
    let held = u64::from(passed_credits(request));
    let prefix = prefix_capacity(request);
    let mut candidates: Vec<Candidate> = request
        .courses
        .iter()
        .filter(|course| !request.passed.contains(&course.code))
        .map(|course| {
            let tree = flat_tree(course)?;
            let domain = value_ordered_domain(course, request)
                .into_iter()
                .filter(|&session| {
                    request.allow_credit_shortfall
                        || tree_admits_ceiling(
                            tree.as_ref(),
                            held + prefix[session - 1],
                        )
                })
                .collect();
            Ok(Candidate {
                code: course.code.clone(),
                credits: course.credits.planning(),
                tree,
                domain,
                masks: course
                    .seasons
                    .iter()
                    .map(|(&season, offering)| {
                        let masks = build_domain(offering)
                            .iter()
                            .map(|opt| opt.mask)
                            .collect();
                        (season, masks)
                    })
                    .collect(),
            })
        })
        .collect::<Result<_, _>>()?;
    order_prerequisites_first(&mut candidates);
    Ok(candidates)
}

// A « Crédits exigés : N » leaf resolves only at a complete assignment —
// placed too early, its course survives the whole descent and rejects
// every leaf, and the DFS re-enumerates the tail forever (B-GMC's
// PHI-2910 and PHI-3900, « Crédits exigés : 30/60 », each rejected
// millions of leaves without one placement landing). The ceiling is
// static — strictly-before credits can never exceed passed plus the
// prefix capacity, and concomitance never relaxes program_credits — so a
// session where every reachable threshold still fails leaves the domain
// before the search starts (ADR
// `2026-08-seuil-de-credits-elague-au-domaine`). Optimistic everywhere
// else: only the ceiling is provable here.
fn tree_admits_ceiling(tree: Option<&FlatTree>, ceiling: u64) -> bool {
    let Some(tree) = tree else {
        return true;
    };
    let mut verdicts = vec![true; tree.nodes.len()];
    for i in (0..tree.nodes.len()).rev() {
        verdicts[i] = match &tree.nodes[i] {
            FlatNode::Credits(threshold) => ceiling >= u64::from(*threshold),
            FlatNode::Course(_) | FlatNode::Raw(_) => true,
            FlatNode::All(children) => {
                children.iter().all(|&child| verdicts[child])
            }
            FlatNode::Any(children) => {
                children.iter().any(|&child| verdicts[child])
            }
        };
    }
    verdicts.first().copied().unwrap_or(true)
}

// prefix_capacity[s] = an upper bound on the credits sessions 1..=s can
// hold together. Every session a regular course may enter counts the full
// cap; a closed été only holds stages — and only stages whose own
// threshold the capacity before that été can still meet — plus whatever
// is pinned into it, capped at the session cap. Without this refinement
// the (session − 1) × cap ceiling counts closed étés as full and admits
// sessions no assignment can ever reach (B-GMC's é3 holds nothing: its
// only stage needs 42 credits by then).
fn prefix_capacity(request: &PlacementRequest) -> Vec<u64> {
    let passed = u64::from(passed_credits(request));
    let mut prefix = vec![0u64; request.sessions.len() + 1];
    for session in 1..=request.sessions.len() {
        let summer = request.sessions[session - 1] == Season::Summer;
        let open = request.open_summers.contains(&session);
        let cap = u64::from(request.credit_cap);
        let slot = if !summer || open {
            cap
        } else {
            let before = passed + prefix[session - 1];
            request
                .courses
                .iter()
                .filter(|course| {
                    request.stages.contains(&course.code)
                        || request.pinned.get(&course.code) == Some(&session)
                })
                .filter(|course| {
                    // an unreadable tree admits: this is an upper bound
                    tree_admits_ceiling(
                        flat_tree(course).ok().flatten().as_ref(),
                        before,
                    )
                })
                .map(|course| u64::from(course.credits.planning()))
                .sum::<u64>()
                .min(cap)
        };
        prefix[session] = prefix[session - 1] + slot;
    }
    prefix
}

// « Crédits exigés » counts *program* credits: the préuniversitaire
// scolarité is en sus of the program (the UI's credit tally already keeps
// it out of the bac), so a checked « scolarité préparatoire faite » must
// not advance PHI-2910's 30-crédits threshold by 20 phantom credits — it
// widened every threshold domain and reopened the search blow-up (ADR
// `2026-08-seuil-de-credits-elague-au-domaine`).
fn passed_credits(request: &PlacementRequest) -> u32 {
    request
        .courses
        .iter()
        .filter(|course| request.passed.contains(&course.code))
        .filter(|course| !is_preuniversity(&course.code))
        .map(|course| course.credits.planning())
        .sum()
}

// The variable order is the search's fate: a dependent expanded before its
// prerequisite seats itself early, and when the prerequisite's turn comes
// no session precedes — every branch dies and the DFS thrashes. On a real
// program it is fatal: B-GMC's MAT-11xx prerequisites sort after every
// GMC code, and the first relaxed leaf sat beyond a 20-million-node budget.
// Prerequisites first (by depth of the listed-prerequisite chains inside
// the candidate set), the input order breaking ties, keeps every descent
// constructive (ADR `2026-08-candidats-ordonnes-prealables-d-abord`).
fn order_prerequisites_first(candidates: &mut [Candidate]) {
    let index_of: BTreeMap<&str, usize> = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| (candidate.code.as_str(), index))
        .collect();
    let mentions: Vec<Vec<usize>> = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            candidate
                .tree
                .iter()
                .flat_map(|tree| tree.nodes.iter())
                .filter_map(|node| match node {
                    FlatNode::Course(code) => index_of.get(code.as_str()),
                    _ => None,
                })
                .copied()
                // a course naming itself must not deepen forever
                .filter(|&mentioned| mentioned != index)
                .collect()
        })
        .collect();
    let mut depths = vec![0usize; candidates.len()];
    // bounded relaxation: n passes settle any acyclic chain; a cycle in
    // the data simply stops deepening at the cap instead of looping
    for _ in 0..candidates.len() {
        let mut changed = false;
        for index in 0..candidates.len() {
            let deepest = mentions[index]
                .iter()
                .map(|&mentioned| depths[mentioned] + 1)
                .max()
                .unwrap_or(0);
            if deepest > depths[index] && deepest <= candidates.len() {
                depths[index] = deepest;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let key_of: BTreeMap<String, (bool, usize)> = candidates
        .iter()
        .zip(&depths)
        .map(|(candidate, &depth)| {
            // a « Crédits exigés » course constrains the aggregate: it can
            // only judge its session once everything else sits — assigned
            // first it pins itself blind and rejects the whole subtree at
            // the leaves. It goes last, whatever its chain depth.
            let aggregate = candidate.tree.as_ref().is_some_and(|tree| {
                tree.nodes
                    .iter()
                    .any(|node| matches!(node, FlatNode::Credits(_)))
            });
            (candidate.code.clone(), (aggregate, depth))
        })
        .collect();
    // stable: ties keep the intake's order
    candidates.sort_by_key(|candidate| {
        key_of.get(&candidate.code).copied().unwrap_or((false, 0))
    });
}

fn flat_tree(course: &Course) -> Result<Option<FlatTree>, PlacementError> {
    match &course.prerequisites {
        None => Ok(None),
        // a whole prerequisite outside the grammar: one unverifiable
        // operand, presumed and surfaced like any other
        Some(Prerequisites::Raw { raw }) => Ok(Some(FlatTree {
            nodes: vec![FlatNode::Raw(raw.clone())],
        })),
        Some(Prerequisites::Parsed { tree, .. }) => {
            flatten(&course.code, tree).map(Some)
        }
    }
}

fn flatten(code: &str, tree: &PrereqTree) -> Result<FlatTree, PlacementError> {
    let mut pending: Vec<&PrereqTree> = vec![tree];
    let mut nodes: Vec<FlatNode> = Vec::new();
    for cursor in 0..MAX_TREE_NODES {
        if cursor >= pending.len() {
            return Ok(FlatTree { nodes });
        }
        let node = pending[cursor];
        let flat = match node {
            PrereqTree::Course(course) => FlatNode::Course(course.clone()),
            PrereqTree::Raw { raw } => FlatNode::Raw(raw.clone()),
            PrereqTree::ProgramCredits { program_credits } => {
                FlatNode::Credits(program_credits.credits)
            }
            PrereqTree::All { all } => {
                let children =
                    (pending.len()..pending.len() + all.len()).collect();
                pending.extend(all.iter());
                FlatNode::All(children)
            }
            PrereqTree::Any { any } => {
                let children =
                    (pending.len()..pending.len() + any.len()).collect();
                pending.extend(any.iter());
                FlatNode::Any(children)
            }
        };
        nodes.push(flat);
    }
    Err(PlacementError::PrerequisiteTreeTooLarge {
        code: code.to_string(),
    })
}

// Value order (conception §5.2) : the seed's session first, then
// neighbours by distance (earlier wins ties) — so the first solution
// resembles the reference cheminement; without a seed, earliest offered
// first. A pin reduces the domain to a singleton; a season not offering
// the course never enters it; the summer rules apply to unpinned courses
// only, so a pin still intersects with the offer. An open été is a last
// resort for a regular course: it enters the order after every regular
// session, the seed included — a course seeded into an été walks back out
// the moment a regular seat frees up (ADR
// `2026-08-ete-en-dernier-recours-dans-lordre-des-valeurs`). A stage has
// the étés for home and is never demoted.
fn value_ordered_domain(
    course: &Course,
    request: &PlacementRequest,
) -> Vec<usize> {
    let mut domain: Vec<usize> = (1..=request.sessions.len())
        .filter(|&session| {
            course.seasons.contains_key(&request.sessions[session - 1])
        })
        .filter(|&session| {
            request
                .pinned
                .get(&course.code)
                .is_none_or(|&pin| pin == session)
        })
        .filter(|&session| {
            request.pinned.contains_key(&course.code)
                || summer_admits(&course.code, session, request)
        })
        .collect();
    let demoted = |session: usize| {
        request.sessions[session - 1] == Season::Summer
            && !request.stages.contains(&course.code)
    };
    let has_credit_threshold =
        course.prerequisites.as_ref().is_some_and(|prerequisites| {
            match prerequisites {
                Prerequisites::Parsed { tree, .. } => tree_has_credits(tree),
                Prerequisites::Raw { .. } => false,
            }
        });
    if request.allow_credit_shortfall
        && has_credit_threshold
        && !request.pinned.contains_key(&course.code)
    {
        domain.sort_by_key(|&session| {
            (demoted(session), std::cmp::Reverse(session))
        });
    } else {
        match request.seed.get(&course.code) {
            Some(&anchor) => domain.sort_by_key(|&session| {
                (demoted(session), session.abs_diff(anchor), session)
            }),
            None => {
                domain.sort_by_key(|&session| (demoted(session), session));
            }
        }
    }
    domain
}

fn tree_has_credits(tree: &PrereqTree) -> bool {
    let mut pending = vec![tree];
    for _ in 0..MAX_TREE_NODES {
        let Some(node) = pending.pop() else {
            break;
        };
        match node {
            PrereqTree::ProgramCredits { .. } => return true,
            PrereqTree::All { all } => pending.extend(all),
            PrereqTree::Any { any } => pending.extend(any),
            PrereqTree::Course(_) | PrereqTree::Raw { .. } => {}
        }
    }
    false
}

// the summer rules for an unpinned course: a stage goes to the étés only;
// a regular course avoids them unless the session was explicitly opened
// (ADR `2026-08-stage-place-en-ete-sauf-epinglage`)
fn summer_admits(
    code: &str,
    session: usize,
    request: &PlacementRequest,
) -> bool {
    let summer = request.sessions[session - 1] == Season::Summer;
    if request.stages.contains(code) {
        summer
    } else {
        !summer || request.open_summers.contains(&session)
    }
}

// Which trees `finalize` must re-evaluate (the flag is static: a leaf's
// category never depends on the assignment). A tree of pure course leaves
// — passed, listed, or blocked — resolves entirely during the search: its
// last incremental verdict is final (placements are only added, and the
// referencing map re-checked it at every relevant addition) and it can
// carry no assumed operand, so re-evaluating it per solution is pure
// waste.
// one entry per depth plus a final zero, so `expand` reads the credits
// still to place in O(1) whatever the depth
fn suffix_credit_sums(candidates: &[Candidate]) -> Vec<u64> {
    let mut sums = vec![0u64; candidates.len() + 1];
    for depth in (0..candidates.len()).rev() {
        sums[depth] = sums[depth + 1] + u64::from(candidates[depth].credits);
    }
    sums
}

fn needs_final_flags(
    candidates: &[Candidate],
    index_of: &BTreeMap<String, usize>,
    request: &PlacementRequest,
) -> Vec<bool> {
    candidates
        .iter()
        .map(|candidate| {
            candidate.tree.as_ref().is_some_and(|tree| {
                tree.nodes.iter().any(|node| match node {
                    // resolves only at the leaf of a complete assignment
                    FlatNode::Credits(_) => true,
                    // contributes an assumed operand to the solution
                    FlatNode::Raw(_) => true,
                    FlatNode::Course(code) => {
                        !request.passed.contains(code)
                            && !index_of.contains_key(code)
                            && is_preuniversity(code)
                    }
                    FlatNode::All(_) | FlatNode::Any(_) => false,
                })
            })
        })
        .collect()
}

// The pre-search screen: a candidate with no admissible session, or whose
// tree no assignment can satisfy, proves the whole instance infeasible in
// O(courses × tree) — where the search would grind its node budget
// rediscovering the same dead end under every arrangement of the other
// courses. The screen is sound, never complete: what it misses (mutually
// exclusive precedences, capacity knots) still falls to the search.
fn blocked_candidates(ctx: &SearchCtx) -> Vec<Blocked> {
    ctx.candidates
        .iter()
        .filter_map(|candidate| {
            // the unsatisfiable tree is the deeper proof and comes first:
            // a threshold no assignment can reach also empties the domain
            // (`tree_admits_ceiling` filters it), and « aucune session »
            // would send the student adjusting sessions for nothing
            if candidate
                .tree
                .as_ref()
                .is_some_and(|tree| !ever_satisfiable(tree, candidate, ctx))
            {
                return Some(Blocked {
                    code: candidate.code.clone(),
                    reason: BlockedReason::UnsatisfiablePrerequisites,
                });
            }
            if candidate.domain.is_empty() {
                // an unpinned stage names the summer restriction as the
                // culprit — pinning it is the way out the student can act on
                let stage_starved =
                    ctx.request.stages.contains(&candidate.code)
                        && !ctx.request.pinned.contains_key(&candidate.code);
                return Some(Blocked {
                    code: candidate.code.clone(),
                    reason: if stage_starved {
                        BlockedReason::StageWithoutSummer
                    } else {
                        BlockedReason::EmptyDomain
                    },
                });
            }
            None
        })
        .collect()
}

// The optimistic two-valued collapse of `node_verdict` : every leaf some
// assignment could satisfy counts as satisfied, so a false root is
// permanent under any search. Only three leaves poison a branch — an
// unknown university code (never presumed, ADR
// `2026-07-presomption-limitee-au-preuniversitaire`), the course as its
// own prerequisite, and a credits threshold above passed plus every other
// candidate. Same child-after-parent reverse scan as `eval`.
fn ever_satisfiable(
    tree: &FlatTree,
    candidate: &Candidate,
    ctx: &SearchCtx,
) -> bool {
    let optimistic_credits = u64::from(ctx.passed_credits)
        + ctx.suffix_credits[0]
        - u64::from(candidate.credits);
    let mut verdicts = vec![false; tree.nodes.len()];
    for i in (0..tree.nodes.len()).rev() {
        verdicts[i] = match &tree.nodes[i] {
            FlatNode::Raw(_) => true,
            FlatNode::Course(code) => {
                ctx.request.passed.contains(code)
                    || (code != &candidate.code
                        && ctx.index_of.contains_key(code))
                    || (!ctx.index_of.contains_key(code)
                        && is_preuniversity(code))
            }
            FlatNode::Credits(threshold) => {
                ctx.request.allow_credit_shortfall
                    || optimistic_credits >= u64::from(*threshold)
            }
            FlatNode::All(children) => {
                children.iter().all(|&child| verdicts[child])
            }
            FlatNode::Any(children) => {
                children.iter().any(|&child| verdicts[child])
            }
        };
    }
    verdicts.first().copied().unwrap_or(true)
}

// Depth-first over an explicit stack, inside a fold bounded by the node
// budget (ADR `2026-07-budget-de-b-en-double-borne`) : one iteration = one
// partial assignment expanded (or one complete one finalized). Depth-first
// keeps the found solutions when a bound stops the search, where A's
// frontier-by-course fold would lose them all.
fn search(ctx: &SearchCtx) -> Placement {
    let mut cache = FeasibilityCache::new();
    // A frame (len, session) names the partial assignment « path[..len-1]
    // then candidates[len-1] ← session » : the shared `path` buffer is
    // re-truncated on pop (ancestor prefixes are LIFO-intact), so pushing
    // a child costs two words instead of cloning the whole prefix.
    // (0, 0) is the empty root assignment — sessions are 1-based.
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    let mut path: Vec<usize> = Vec::with_capacity(ctx.candidates.len());
    let mut solutions: Vec<Solution> = Vec::new();
    let flow = (0..ctx.request.max_nodes).try_fold((), |(), _| {
        match stack.pop() {
            // the stack ran dry: the search is exhausted, the set proven
            // total — exhaustion wins over a simultaneously full cap
            None => ControlFlow::Break(Completion::Complete),
            Some((len, session)) => {
                path.truncate(len.saturating_sub(1));
                if len > 0 {
                    path.push(session);
                }
                if path.len() == ctx.candidates.len() {
                    // `None` (a violated tree at the leaf) is provably
                    // unreachable — the incremental layer re-checks every
                    // tree at its last relevant assignment, `complete`
                    // flagged — but the leaf check stays: « jamais un
                    // placement en violation » deserves its last line
                    solutions.extend(finalize(&path, ctx));
                } else {
                    expand(&path, ctx, &mut cache, &mut stack);
                }
                if solutions.len() >= ctx.request.max_solutions
                    && !stack.is_empty()
                {
                    ControlFlow::Break(Completion::SolutionCap)
                } else {
                    ControlFlow::Continue(())
                }
            }
        }
    });
    let completion = match flow {
        ControlFlow::Break(completion) => completion,
        // budget spent to the last node: exhausted work still proves the
        // set total when nothing is left to expand
        ControlFlow::Continue(()) if stack.is_empty() => Completion::Complete,
        ControlFlow::Continue(()) => Completion::NodeBudget,
    };
    Placement {
        completion,
        solutions,
        blocked: Vec::new(),
    }
}

// One extension step: try every session of the next course's domain, keep
// the children that survive the cheap structural filters — capacity,
// precedence, then the memoized weekly veto (costliest last). Children are
// pushed in reverse so the first domain value is explored first.
fn expand(
    chosen: &[usize],
    ctx: &SearchCtx,
    cache: &mut FeasibilityCache,
    stack: &mut Vec<(usize, usize)>,
) {
    let depth = chosen.len();
    let candidate = &ctx.candidates[depth];
    let loads = session_loads(chosen, ctx);
    // no completion exists when the credits still to place exceed the
    // capacity left across every session: prune the whole subtree before
    // generating a single child (u64: the cap is caller-supplied, its
    // product with the session count may overflow u32)
    let used: u64 = loads.iter().map(|&load| u64::from(load)).sum();
    let capacity =
        u64::from(ctx.request.credit_cap) * ctx.request.sessions.len() as u64;
    // relaxed, the excess is left out rather than making the subtree
    // impossible — the bound would cut the very branches we are after
    if !ctx.request.allow_unplaced
        && ctx.suffix_credits[depth] > capacity - used
    {
        return;
    }
    // one scratch child per expansion, its last slot rewritten per session
    let mut child = Vec::with_capacity(depth + 1);
    child.extend_from_slice(chosen);
    child.push(0);
    let children: Vec<usize> = candidate
        .domain
        .iter()
        .copied()
        .filter(|&session| {
            loads[session - 1] + candidate.credits <= ctx.request.credit_cap
        })
        .filter(|&session| {
            child[depth] = session;
            precedence_admits(&child, depth, ctx)
                && weekly_admits(&child, depth, ctx, cache)
        })
        .collect();
    // pushed first so it is popped last: leaving a course out is the
    // move of last resort, tried only once every real session failed.
    // It goes through `precedence_admits` like any child — skipping a
    // course can falsify the tree of a placed course that requires it.
    // ponytail: greedy in candidate order (itself seeded by the
    // cheminement type), so the first leaf is a maximal — not a
    // maximum — filling; branch-and-bound on the skip count if a real
    // case ever needs the difference.
    if ctx.request.allow_unplaced {
        child[depth] = UNPLACED;
        if precedence_admits(&child, depth, ctx) {
            stack.push((depth + 1, UNPLACED));
        }
    }
    stack.extend(
        children
            .into_iter()
            .rev()
            .map(|session| (depth + 1, session)),
    );
}

fn session_loads(chosen: &[usize], ctx: &SearchCtx) -> Vec<u32> {
    chosen.iter().enumerate().fold(
        vec![0u32; ctx.request.sessions.len()],
        |mut loads, (i, &session)| {
            if session != UNPLACED {
                loads[session - 1] += ctx.candidates[i].credits;
            }
            loads
        },
    )
}

// Placing course `depth` can only violate its own tree or the tree of an
// already-placed course that references it — `False` being permanent,
// checking exactly those keeps every prune sound and the work per node
// tiny.
fn precedence_admits(child: &[usize], depth: usize, ctx: &SearchCtx) -> bool {
    let complete = child.len() == ctx.candidates.len();
    // A course left out constrains in one direction only: its dependents
    // must see it missing (`course_leaf`), but its own prerequisites stop
    // applying — nothing is required in order to *not* take a course.
    // Evaluating them anyway kills every branch of a candidate the
    // pre-screen already knows is unsatisfiable, and the relaxed search
    // silently falls back to finding nothing at all.
    let own = (child[depth] != UNPLACED).then_some(depth);
    let rechecked = ctx
        .referenced_by
        .get(&ctx.candidates[depth].code)
        .into_iter()
        .flatten()
        .filter(|&&i| i < depth)
        // a credits threshold watches every assignment: each extension
        // shrinks what could still land before it
        .chain(ctx.credit_watch.iter().filter(|&&i| i < depth))
        .chain(own.iter());
    rechecked.into_iter().all(|&evaluated| {
        // a course left out has no tree left to satisfy — the mirror of
        // `finalize`'s skip; without it a watched threshold whose course
        // the relaxation set aside would veto every branch below
        if child[evaluated] == UNPLACED {
            return true;
        }
        match &ctx.candidates[evaluated].tree {
            None => true,
            Some(tree) => {
                let eval_ctx = EvalCtx {
                    ctx,
                    chosen: child,
                    evaluated,
                    complete,
                };
                eval(tree, &eval_ctx) != Verdict::False
            }
        }
    })
}

// the A-veto, memoized on (season, bitset of candidate indices) — checked
// on the partial session content at every addition: the veto is monotone
// (adding a course never repairs a conflict), so an early veto is already
// final
fn weekly_admits(
    child: &[usize],
    depth: usize,
    ctx: &SearchCtx,
    cache: &mut FeasibilityCache,
) -> bool {
    let session = child[depth];
    let members = child
        .iter()
        .enumerate()
        .filter(|&(_, &placed)| placed == session)
        .fold(0u128, |bits, (i, _)| bits | 1u128 << i);
    let season = ctx.request.sessions[session - 1];
    // indexing is safe: every member was placed at this session through
    // its own domain, and `value_ordered_domain` only admits sessions
    // whose season the course offers — `masks` covers exactly those
    cache.term_feasible(season, members, |i| {
        ctx.candidates[i].masks[&season].as_slice()
    })
}

// A complete assignment survived every incremental filter; the final pass
// resolves the verdicts that only a complete assignment can decide
// (`program_credits` thresholds) and collects the operands the placement
// had to presume.
fn finalize(chosen: &[usize], ctx: &SearchCtx) -> Option<Solution> {
    let (assumed, credit_shortfalls) =
        ctx.candidates.iter().enumerate().try_fold(
            (BTreeSet::new(), BTreeSet::new()),
            |(mut assumed, mut shortfalls), (evaluated, candidate)| {
                match &candidate.tree {
                    None => Some((assumed, shortfalls)),
                    // left out: its own tree no longer applies (see
                    // `precedence_admits`), and it contributes no assumption
                    Some(_) if chosen[evaluated] == UNPLACED => {
                        Some((assumed, shortfalls))
                    }
                    // already proven Sat(∅) incrementally — see `needs_final_flags`
                    Some(_) if !ctx.needs_final[evaluated] => {
                        Some((assumed, shortfalls))
                    }
                    Some(tree) => {
                        let eval_ctx = EvalCtx {
                            ctx,
                            chosen,
                            evaluated,
                            complete: true,
                        };
                        if ctx.request.allow_credit_shortfall {
                            match final_eval(tree, &eval_ctx) {
                                FinalVerdict::Sat(evidence) => {
                                    assumed.extend(evidence.assumed);
                                    shortfalls.extend(evidence.shortfalls);
                                    return Some((assumed, shortfalls));
                                }
                                FinalVerdict::False => return None,
                            }
                        }
                        match eval(tree, &eval_ctx) {
                            Verdict::Sat(operands) => {
                                assumed.extend(operands);
                                Some((assumed, shortfalls))
                            }
                            // False: a credits threshold resolved short at the
                            // leaf. Unknown cannot survive a complete assignment;
                            // rejecting is the safe reading if it ever did.
                            Verdict::False | Verdict::Unknown => None,
                        }
                    }
                }
            },
        )?;
    let (placed, left_out) = chosen
        .iter()
        .enumerate()
        .partition::<Vec<(usize, &usize)>, _>(|&(_, &session)| {
            session != UNPLACED
        });
    Some(Solution {
        placement: placed
            .into_iter()
            .map(|(i, &session)| (ctx.candidates[i].code.clone(), session))
            .collect(),
        assumed,
        left_out: left_out
            .into_iter()
            .map(|(i, _)| ctx.candidates[i].code.clone())
            .collect(),
        credit_shortfalls: credit_shortfalls.into_iter().collect(),
    })
}

#[derive(Clone, Default)]
struct FinalEvidence {
    assumed: BTreeSet<String>,
    shortfalls: BTreeSet<CreditShortfall>,
}

enum FinalVerdict {
    Sat(FinalEvidence),
    False,
}

fn final_eval(tree: &FlatTree, eval_ctx: &EvalCtx) -> FinalVerdict {
    let mut verdicts: Vec<FinalVerdict> =
        (0..tree.nodes.len()).map(|_| FinalVerdict::False).collect();
    for index in (0..tree.nodes.len()).rev() {
        verdicts[index] = match &tree.nodes[index] {
            FlatNode::Course(code) => match course_leaf(code, eval_ctx) {
                Verdict::Sat(assumed) => FinalVerdict::Sat(FinalEvidence {
                    assumed,
                    shortfalls: BTreeSet::new(),
                }),
                Verdict::False | Verdict::Unknown => FinalVerdict::False,
            },
            FlatNode::Raw(raw) => FinalVerdict::Sat(FinalEvidence {
                assumed: BTreeSet::from([raw.clone()]),
                shortfalls: BTreeSet::new(),
            }),
            FlatNode::Credits(required) => {
                let earned_before = earned_before(eval_ctx);
                let mut shortfalls = BTreeSet::new();
                if earned_before < *required {
                    shortfalls.insert(CreditShortfall {
                        code: eval_ctx.ctx.candidates[eval_ctx.evaluated]
                            .code
                            .clone(),
                        session: eval_ctx.chosen[eval_ctx.evaluated],
                        earned_before,
                        required: *required,
                    });
                }
                FinalVerdict::Sat(FinalEvidence {
                    assumed: BTreeSet::new(),
                    shortfalls,
                })
            }
            FlatNode::All(children) => combine_all(children, &verdicts),
            FlatNode::Any(children) => combine_any(children, &verdicts),
        };
    }
    verdicts.into_iter().next().unwrap_or(FinalVerdict::False)
}

fn combine_all(children: &[usize], verdicts: &[FinalVerdict]) -> FinalVerdict {
    use FinalVerdict::{False, Sat};

    let mut combined = FinalEvidence::default();
    let mut all_satisfied = true;
    for &child in children {
        all_satisfied &= matches!(verdicts[child], Sat(_));
        if let Sat(evidence) = &verdicts[child] {
            combined.assumed.extend(evidence.assumed.iter().cloned());
            combined
                .shortfalls
                .extend(evidence.shortfalls.iter().cloned());
        }
    }
    all_satisfied.then_some(combined).map(Sat).unwrap_or(False)
}

fn combine_any(children: &[usize], verdicts: &[FinalVerdict]) -> FinalVerdict {
    children
        .iter()
        .filter_map(|&child| match &verdicts[child] {
            FinalVerdict::Sat(evidence) => Some(evidence),
            FinalVerdict::False => None,
        })
        .min_by_key(|evidence| {
            (evidence.shortfalls.len(), evidence.assumed.len())
        })
        .cloned()
        .map(FinalVerdict::Sat)
        .unwrap_or(FinalVerdict::False)
}

fn earned_before(eval_ctx: &EvalCtx) -> u32 {
    let session = eval_ctx.chosen[eval_ctx.evaluated];
    eval_ctx.ctx.passed_credits
        + eval_ctx
            .chosen
            .iter()
            .enumerate()
            .filter(|&(_, &placed)| placed != UNPLACED && placed < session)
            .filter(|&(index, _)| {
                !is_preuniversity(&eval_ctx.ctx.candidates[index].code)
            })
            .map(|(index, _)| eval_ctx.ctx.candidates[index].credits)
            .sum::<u32>()
}

// children always sit after their parent in the flat tree, so one reverse
// scan has every child's verdict ready when its parent combines them
fn eval(tree: &FlatTree, eval_ctx: &EvalCtx) -> Verdict {
    let mut verdicts = vec![Verdict::Unknown; tree.nodes.len()];
    for i in (0..tree.nodes.len()).rev() {
        let verdict = node_verdict(&tree.nodes[i], &verdicts, eval_ctx);
        verdicts[i] = verdict;
    }
    verdicts.into_iter().next().unwrap_or(Verdict::Unknown)
}

fn node_verdict(
    node: &FlatNode,
    verdicts: &[Verdict],
    eval_ctx: &EvalCtx,
) -> Verdict {
    match node {
        FlatNode::Course(code) => course_leaf(code, eval_ctx),
        // an operand no rule can check — an examination, a range of course
        // numbers: presumed satisfied and surfaced, never imposed
        FlatNode::Raw(raw) => {
            Verdict::Sat(std::iter::once(raw.clone()).collect())
        }
        FlatNode::Credits(threshold) => credits_leaf(*threshold, eval_ctx),
        FlatNode::All(children) => all_verdict(children, verdicts),
        FlatNode::Any(children) => any_verdict(children, verdicts),
    }
}

fn course_leaf(code: &str, eval_ctx: &EvalCtx) -> Verdict {
    if eval_ctx.ctx.request.passed.contains(code) {
        return Verdict::Sat(BTreeSet::new());
    }
    let Some(&index) = eval_ctx.ctx.index_of.get(code) else {
        // neither listed nor passed: only a préuniversitaire code (0xxx)
        // is presumed satisfied and surfaced — any other unknown code is
        // a real university course the student would still have to take,
        // so it blocks (ADR
        // `2026-07-presomption-limitee-au-preuniversitaire`)
        return if is_preuniversity(code) {
            Verdict::Sat(std::iter::once(code.to_string()).collect())
        } else {
            Verdict::False
        };
    };
    let Some(&placed) = eval_ctx.chosen.get(index) else {
        return Verdict::Unknown;
    };
    // the sentinel is not a session: `placed < session` would read « left
    // out » as « placed before » and hand this prerequisite to every
    // dependent of a course nobody is taking. Leaving a course out
    // cascades onto whatever required it — that cascade is the honesty of
    // the relaxation, not a defect of it.
    if placed == UNPLACED {
        return Verdict::False;
    }
    let session = eval_ctx.chosen[eval_ctx.evaluated];
    let self_code = &eval_ctx.ctx.candidates[eval_ctx.evaluated].code;
    // « strictly before », relaxed to « before or same » under the global
    // concomitant option — a course never being its own concomitant
    let before = placed < session
        || (eval_ctx.ctx.request.concomitant
            && placed == session
            && code != self_code);
    if before {
        Verdict::Sat(BTreeSet::new())
    } else {
        Verdict::False
    }
}

// accumulated *program* credits strictly before the evaluated course's
// session: passed credits count in full, préuniversitaire ones never —
// the scolarité préparatoire is en sus of the program, whether passed or
// seated in a session; placements at the same session never count,
// concomitant or not
fn credits_leaf(threshold: u32, eval_ctx: &EvalCtx) -> Verdict {
    let session = eval_ctx.chosen[eval_ctx.evaluated];
    let before = earned_before(eval_ctx);
    if before >= threshold {
        Verdict::Sat(BTreeSet::new())
    } else if eval_ctx.ctx.request.allow_credit_shortfall {
        Verdict::Unknown
    } else if eval_ctx.complete {
        Verdict::False
    } else {
        // an unassigned course may still land before and lift the total —
        // but only one whose domain reaches an earlier session. Once even
        // all of those cannot bridge the gap, the verdict is False *now*:
        // the potential only shrinks as the descent assigns, so pruning
        // here is what keeps a doomed threshold from rejecting its whole
        // subtree leaf by leaf (B-GMC's PHI-3900 rejected 3 million
        // leaves before this bound existed).
        let potential: u32 = before
            + eval_ctx.ctx.candidates[eval_ctx.chosen.len()..]
                .iter()
                .filter(|candidate| {
                    candidate.domain.iter().any(|&value| value < session)
                })
                .filter(|candidate| !is_preuniversity(&candidate.code))
                .map(|candidate| candidate.credits)
                .sum::<u32>();
        if potential < threshold {
            Verdict::False
        } else {
            Verdict::Unknown
        }
    }
}

fn all_verdict(children: &[usize], verdicts: &[Verdict]) -> Verdict {
    if children
        .iter()
        .any(|&child| verdicts[child] == Verdict::False)
    {
        return Verdict::False;
    }
    if children
        .iter()
        .any(|&child| verdicts[child] == Verdict::Unknown)
    {
        return Verdict::Unknown;
    }
    // every child is Sat here: their assumptions accumulate
    Verdict::Sat(
        children
            .iter()
            .filter_map(|&child| sat_operands(&verdicts[child]))
            .flat_map(|operands| operands.iter().cloned())
            .collect(),
    )
}

// a branch proven without assumptions wins outright; else the first
// assumed branch (deterministic surfacing); an Unknown branch may still
// prove itself later, so it beats False
fn any_verdict(children: &[usize], verdicts: &[Verdict]) -> Verdict {
    if children
        .iter()
        .any(|&child| verdicts[child] == Verdict::Sat(BTreeSet::new()))
    {
        return Verdict::Sat(BTreeSet::new());
    }
    if let Some(operands) = children
        .iter()
        .find_map(|&child| sat_operands(&verdicts[child]))
    {
        return Verdict::Sat(operands.clone());
    }
    if children
        .iter()
        .any(|&child| verdicts[child] == Verdict::Unknown)
    {
        return Verdict::Unknown;
    }
    Verdict::False
}

fn sat_operands(verdict: &Verdict) -> Option<&BTreeSet<String>> {
    match verdict {
        Verdict::Sat(operands) => Some(operands),
        Verdict::False | Verdict::Unknown => None,
    }
}

fn referencing_map(candidates: &[Candidate]) -> BTreeMap<String, Vec<usize>> {
    candidates.iter().enumerate().fold(
        BTreeMap::new(),
        |mut map, (i, candidate)| {
            let leaves = candidate
                .tree
                .iter()
                .flat_map(|tree| &tree.nodes)
                .filter_map(|node| match node {
                    FlatNode::Course(code) => Some(code.clone()),
                    _ => None,
                });
            // a tree naming the same code twice re-checks twice: harmless,
            // and simpler than deduplicating
            for code in leaves {
                map.entry(code).or_default().push(i);
            }
            map
        },
    )
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // one in-person option on `day`, offered in both fall and winter
    fn anytime(code: &str, day: &str) -> Course {
        with_prereq(code, day, "null")
    }

    fn with_prereq(code: &str, day: &str, prerequisites: &str) -> Course {
        let offering = format!(
            r#"{{"last_offered":2026,
                 "options":[[{{"nrc":"1","section":"A","mode":"in-person",
                "slots":[{{"day":"{day}","start":"08:30",
                           "end":"11:20"}}]}}]]}}"#
        );
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":{prerequisites},"equivalents":[],
                 "seasons":{{"fall":{offering},"winter":{offering}}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn parsed(tree: &str) -> String {
        format!(r#"{{"raw":"source","tree":{tree}}}"#)
    }

    // one in-person option on `day`, offered in fall, winter and summer
    fn all_seasons(code: &str, day: &str) -> Course {
        let offering = format!(
            r#"{{"last_offered":2026,
                 "options":[[{{"nrc":"1","section":"A","mode":"in-person",
                "slots":[{{"day":"{day}","start":"08:30",
                           "end":"11:20"}}]}}]]}}"#
        );
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"fall":{offering},"winter":{offering},
                             "summer":{offering}}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    const FALL_WINTER: [Season; 2] = [Season::Fall, Season::Winter];
    const FALL_WINTER_SUMMER: [Season; 3] =
        [Season::Fall, Season::Winter, Season::Summer];

    fn stages(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|code| code.to_string()).collect()
    }

    struct Inputs {
        sessions: Vec<Season>,
        courses: Vec<Course>,
        passed: BTreeSet<String>,
        pinned: BTreeMap<String, usize>,
        stages: BTreeSet<String>,
        open_summers: BTreeSet<usize>,
        seed: BTreeMap<String, usize>,
        credit_cap: u32,
        concomitant: bool,
        max_nodes: u64,
        max_solutions: usize,
        allow_unplaced: bool,
        allow_credit_shortfall: bool,
    }

    impl Inputs {
        fn new(sessions: &[Season], courses: Vec<Course>) -> Self {
            Inputs {
                sessions: sessions.to_vec(),
                courses,
                passed: BTreeSet::new(),
                pinned: BTreeMap::new(),
                stages: BTreeSet::new(),
                open_summers: BTreeSet::new(),
                seed: BTreeMap::new(),
                credit_cap: 30,
                concomitant: false,
                max_nodes: 100_000,
                max_solutions: 10_000,
                allow_unplaced: false,
                allow_credit_shortfall: false,
            }
        }

        fn place(&self) -> Result<Placement, PlacementError> {
            place(&PlacementRequest {
                sessions: &self.sessions,
                credit_cap: self.credit_cap,
                concomitant: self.concomitant,
                courses: &self.courses,
                passed: &self.passed,
                pinned: &self.pinned,
                stages: &self.stages,
                open_summers: &self.open_summers,
                seed: &self.seed,
                max_nodes: self.max_nodes,
                max_solutions: self.max_solutions,
                allow_unplaced: self.allow_unplaced,
                allow_credit_shortfall: self.allow_credit_shortfall,
            })
        }

        fn solve(&self) -> Placement {
            self.place().unwrap_or_else(|e| panic!("{e}"))
        }

        fn admissible(
            &self,
            code: &str,
        ) -> Result<BTreeSet<usize>, PlacementError> {
            admissible_sessions(
                &PlacementRequest {
                    sessions: &self.sessions,
                    credit_cap: self.credit_cap,
                    concomitant: self.concomitant,
                    courses: &self.courses,
                    passed: &self.passed,
                    pinned: &self.pinned,
                    stages: &self.stages,
                    open_summers: &self.open_summers,
                    seed: &self.seed,
                    max_nodes: self.max_nodes,
                    max_solutions: self.max_solutions,
                    // the chip means « it fits », never « the rest is out »
                    allow_unplaced: false,
                    allow_credit_shortfall: false,
                },
                code,
            )
        }
    }

    fn passed(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|code| code.to_string()).collect()
    }

    fn sorted_placements(placement: &Placement) -> Vec<Vec<(String, usize)>> {
        let mut all: Vec<Vec<(String, usize)>> = placement
            .solutions
            .iter()
            .map(|solution| {
                solution
                    .placement
                    .iter()
                    .map(|(code, &session)| (code.clone(), session))
                    .collect()
            })
            .collect();
        all.sort();
        all
    }

    fn pairs(entries: &[(&str, usize)]) -> Vec<(String, usize)> {
        entries
            .iter()
            .map(|(code, session)| (code.to_string(), *session))
            .collect()
    }

    // --- « remplir au mieux » (ADR `2026-08-placement-au-mieux-en-repli`)

    // Relaxed, the sentinel is a domain value of last resort: what fits is
    // still placed under every constraint, what does not is named. Without
    // it this same request answers nothing at all.
    #[test]
    fn a_relaxed_search_places_what_fits_and_names_the_rest() {
        // B-2 requires a code nobody takes, C-3 requires B-2: leaving B-2
        // out must cascade onto C-3 — a sentinel read as « placed before »
        // would hand C-3 its prerequisite and seat it on nothing
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq("B-2", "tuesday", &parsed(r#""Z-9""#)),
            with_prereq("C-3", "wednesday", &parsed(r#""B-2""#)),
        ];
        let mut inputs = Inputs::new(&FALL_WINTER, courses);
        inputs.max_solutions = 1;

        // exact: the pre-screen names B-2 and the search never runs
        let exact = inputs.solve();
        assert!(exact.solutions.is_empty());
        assert_eq!(exact.blocked[0].code, "B-2");
        assert_eq!(
            exact.blocked[0].reason,
            BlockedReason::UnsatisfiablePrerequisites
        );

        inputs.allow_unplaced = true;
        let relaxed = inputs.solve();
        let solution = &relaxed.solutions[0];
        assert_eq!(
            solution.placement,
            BTreeMap::from([("A-1".to_string(), 1)])
        );
        assert_eq!(
            solution.left_out,
            BTreeSet::from(["B-2".to_string(), "C-3".to_string()]),
            "leaving a course out cascades onto what required it"
        );
        // the screen still reports, it just no longer short-circuits
        assert_eq!(relaxed.blocked[0].code, "B-2");
    }

    // The sentinel is tried last at every depth: a request that has a
    // complete arrangement finds it, relaxed or not.
    #[test]
    fn a_relaxed_search_still_prefers_placing_everything() {
        let inputs = {
            let mut inputs = Inputs::new(
                &FALL_WINTER,
                vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
            );
            inputs.allow_unplaced = true;
            inputs.max_solutions = 1;
            inputs
        };
        let solution = &inputs.solve().solutions[0];
        assert!(solution.left_out.is_empty(), "{:?}", solution.left_out);
        assert_eq!(solution.placement.len(), 2);
    }

    // Candidate order is the caller's course order, which nothing forces
    // to be topological: a dependent can be decided before its own
    // prerequisite is even reached. Leaving that prerequisite out must
    // then be refused, or the search answers with a placement whose
    // prerequisite is nowhere — the relaxation is allowed to drop courses,
    // never to keep one whose tree it just falsified.
    #[test]
    fn skipping_a_course_a_placed_one_already_requires_is_refused() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                // the dependent first, so it is placed at depth 0 while
                // A-1 is still undecided
                with_prereq("B-2", "monday", &parsed("\"A-1\"")),
                anytime("A-1", "tuesday"),
            ],
        );
        inputs.allow_unplaced = true;
        let placement = inputs.solve();
        assert!(!placement.solutions.is_empty());
        for solution in &placement.solutions {
            assert!(
                !(solution.placement.contains_key("B-2")
                    && solution.left_out.contains("A-1")),
                "B-2 kept while its prerequisite was dropped: {solution:?}"
            );
        }
        // and the arrangement that places both is still found
        assert!(placement.solutions.iter().any(|solution| {
            solution.left_out.is_empty() && solution.placement.len() == 2
        }));
    }

    // The capacity bound is unsound once the excess may be left out: it
    // would prune the very branches the relaxation exists to reach.
    #[test]
    fn a_relaxed_search_leaves_out_what_the_cap_cannot_hold() {
        let mut inputs = Inputs::new(
            &[Season::Fall],
            vec![
                anytime("A-1", "monday"),
                anytime("B-2", "tuesday"),
                anytime("C-3", "wednesday"),
            ],
        );
        inputs.credit_cap = 6;
        inputs.allow_unplaced = true;
        inputs.max_solutions = 1;
        let solution = &inputs.solve().solutions[0];
        assert_eq!(solution.placement.len(), 2, "the cap holds two");
        assert_eq!(solution.left_out.len(), 1);
    }

    // --- validation: surfaced, never guessed ---

    #[test]
    fn an_empty_request_is_an_error() {
        let inputs = Inputs::new(&FALL_WINTER, Vec::new());
        assert_eq!(inputs.place(), Err(PlacementError::EmptyRequest));
        let inputs = Inputs::new(&[], vec![anytime("A-1", "monday")]);
        assert_eq!(inputs.place(), Err(PlacementError::EmptyRequest));
    }

    #[test]
    fn a_duplicated_course_is_an_error() {
        let inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("A-1", "tuesday")],
        );
        assert_eq!(
            inputs.place(),
            Err(PlacementError::DuplicateCourse {
                code: "A-1".to_string()
            })
        );
    }

    #[test]
    fn a_passed_or_pinned_code_without_a_course_is_an_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.passed = passed(&["Z-9"]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::UnknownCode {
                code: "Z-9".to_string()
            })
        );
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.pinned = BTreeMap::from([("Z-9".to_string(), 1)]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::UnknownCode {
                code: "Z-9".to_string()
            })
        );
    }

    #[test]
    fn a_course_both_passed_and_pinned_is_an_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.passed = passed(&["A-1"]);
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 1)]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::PassedAndPinned {
                code: "A-1".to_string()
            })
        );
    }

    #[test]
    fn a_pin_outside_the_sessions_is_an_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 3)]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::PinnedOutOfRange {
                code: "A-1".to_string(),
                session: 3,
                sessions: 2,
            })
        );
    }

    #[test]
    fn a_stage_code_without_a_course_is_an_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.stages = stages(&["Z-9"]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::StageWithoutCourse {
                code: "Z-9".to_string()
            })
        );
    }

    #[test]
    fn an_open_summer_outside_the_sessions_is_an_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.open_summers = BTreeSet::from([3]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::OpenSummerOutOfRange {
                session: 3,
                sessions: 2,
            })
        );
    }

    #[test]
    fn an_open_summer_on_a_study_season_is_an_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.open_summers = BTreeSet::from([2]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::OpenSummerNotSummer {
                session: 2,
                season: Season::Winter,
            })
        );
    }

    #[test]
    fn every_placement_error_names_its_subject() {
        let code = || "A-1".to_string();
        for error in [
            PlacementError::DuplicateCourse { code: code() },
            PlacementError::UnknownCode { code: code() },
            PlacementError::PassedAndPinned { code: code() },
            PlacementError::PinnedOutOfRange {
                code: code(),
                session: 3,
                sessions: 2,
            },
            PlacementError::StageWithoutCourse { code: code() },
            PlacementError::PrerequisiteTreeTooLarge { code: code() },
        ] {
            assert!(error.to_string().contains("A-1"), "{error}");
        }
        assert!(PlacementError::EmptyRequest.to_string().contains("needs"));
        let too_many = PlacementError::TooManyCourses { count: 129 };
        assert!(too_many.to_string().contains("129"), "{too_many}");
        let out_of_range = PlacementError::OpenSummerOutOfRange {
            session: 3,
            sessions: 2,
        };
        assert!(out_of_range.to_string().contains("3"), "{out_of_range}");
        let not_summer = PlacementError::OpenSummerNotSummer {
            session: 2,
            season: Season::Winter,
        };
        assert!(not_summer.to_string().contains("2"), "{not_summer}");
    }

    #[test]
    fn more_candidates_than_the_bitset_width_is_an_error() {
        // the feasibility cache packs candidate indices in a u128: a list
        // past 128 to-place courses is refused loudly, never truncated
        let courses: Vec<Course> = (0..129)
            .map(|i| anytime(&format!("C-{i}"), "monday"))
            .collect();
        let inputs = Inputs::new(&FALL_WINTER, courses);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::TooManyCourses { count: 129 })
        );
    }

    // --- structural filters ---

    #[test]
    fn a_season_not_offering_the_course_never_enters_its_domain() {
        // fall-only course over [fall, winter]: winter never appears
        let fall_only: Course = serde_json::from_str(
            r#"{"code":"A-1","title":"T","credits":3,"cycle":1,
                "prerequisites":null,"equivalents":[],
                "seasons":{"fall":{"last_offered":2026,"options":[[
                  {"nrc":"1","section":"A","mode":"in-person",
                   "slots":[{"day":"monday","start":"08:30",
                             "end":"11:20"}]}]]}}}"#,
        )
        .unwrap_or_else(|e| panic!("course literal: {e}"));
        let placement = Inputs::new(&FALL_WINTER, vec![fall_only]).solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(sorted_placements(&placement), vec![pairs(&[("A-1", 1)])]);
    }

    #[test]
    fn an_unpublished_schedule_still_places() {
        // the GCI-1011 shape (new-course rule): fall+winter, no vintage, no
        // schedule — the placeholder domain occupies nothing, so the course
        // places alongside anything instead of being set aside
        let new_course: Course = serde_json::from_str(
            r#"{"code":"N-1011","title":"T","credits":3,"cycle":1,
                "prerequisites":null,"equivalents":[],
                "seasons":{"fall":{"last_offered":null,"options":null},
                           "winter":{"last_offered":null,"options":null}}}"#,
        )
        .unwrap_or_else(|e| panic!("course literal: {e}"));
        let placement = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), new_course],
        )
        .solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(!placement.solutions.is_empty());
    }

    #[test]
    fn a_course_offered_in_no_listed_season_is_proven_infeasible() {
        let summer_only: Course = serde_json::from_str(
            r#"{"code":"A-1","title":"T","credits":3,"cycle":1,
                "prerequisites":null,"equivalents":[],
                "seasons":{"summer":{"last_offered":2026,"options":[[]]}}}"#,
        )
        .unwrap_or_else(|e| panic!("course literal: {e}"));
        let placement = Inputs::new(&FALL_WINTER, vec![summer_only]).solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(placement.solutions.is_empty());
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "A-1".to_string(),
                reason: BlockedReason::EmptyDomain,
            }]
        );
    }

    // --- summer rules: stages to the étés, regular courses out of them ---

    #[test]
    fn an_unpinned_stage_lands_only_in_summer() {
        let mut inputs = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("S-1580", "monday")],
        );
        inputs.stages = stages(&["S-1580"]);
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("S-1580", 3)])]
        );
    }

    #[test]
    fn a_pinned_stage_escapes_the_summer_restriction() {
        let mut inputs = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("S-1580", "monday")],
        );
        inputs.stages = stages(&["S-1580"]);
        inputs.pinned = BTreeMap::from([("S-1580".to_string(), 1)]);
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("S-1580", 1)])]
        );
    }

    #[test]
    fn a_stage_without_a_summer_session_is_blocked_as_such() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![all_seasons("S-1580", "monday")]);
        inputs.stages = stages(&["S-1580"]);
        let placement = inputs.solve();
        assert!(placement.solutions.is_empty());
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "S-1580".to_string(),
                reason: BlockedReason::StageWithoutSummer,
            }]
        );
    }

    #[test]
    fn a_regular_course_avoids_a_closed_summer() {
        let placement = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("A-1", "monday")],
        )
        .solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1)]), pairs(&[("A-1", 2)])]
        );
    }

    #[test]
    fn an_open_summer_admits_a_regular_course() {
        let mut inputs = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("A-1", "monday")],
        );
        inputs.open_summers = BTreeSet::from([3]);
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![
                pairs(&[("A-1", 1)]),
                pairs(&[("A-1", 2)]),
                pairs(&[("A-1", 3)])
            ]
        );
    }

    #[test]
    fn a_seeded_summer_course_walks_back_to_a_regular_seat() {
        // the escalation seeds what it placed in an été; the demotion
        // primes the seed, so the course leaves the été the moment a
        // regular seat is free
        let mut inputs = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("A-1", "monday")],
        );
        inputs.open_summers = BTreeSet::from([3]);
        inputs.seed = BTreeMap::from([("A-1".to_string(), 3)]);
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(sorted_placements(&placement), vec![pairs(&[("A-1", 2)])]);
    }

    #[test]
    fn prerequisites_are_assigned_before_their_dependents() {
        // Z-1 is A-2's prerequisite but sorts after it alphabetically —
        // exactly B-GMC's MAT-after-GMC pathology. The first solution of a
        // one-solution search only comes out constructive if Z-1 is
        // assigned first.
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                with_prereq("A-2", "monday", &parsed(r#""Z-1""#)),
                anytime("Z-1", "tuesday"),
            ],
        );
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-2", 2), ("Z-1", 1)])]
        );
    }

    #[test]
    fn a_credits_threshold_course_is_assigned_last() {
        // the aggregate can only judge its seat once everything else sits;
        // assigned first it pins itself blind and grinds the node budget
        // (B-GMC's PHI-2910/PHI-3900)
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                with_prereq(
                    "AAA-9",
                    "monday",
                    &parsed(r#"{"program_credits":{"credits":3}}"#),
                ),
                anytime("B-1", "tuesday"),
            ],
        );
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        // B-1 first (session 1), AAA-9 after its threshold is met
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("AAA-9", 2), ("B-1", 1)])]
        );
    }

    #[test]
    fn a_threshold_prunes_the_sessions_its_prefix_cannot_reach() {
        // 6 credits demanded before T-9: session 1 (nothing before) and
        // session 2 (one 3-credit course at most under cap 3) are pruned
        // from the domain — the search never grinds them
        let mut inputs = Inputs::new(
            &[Season::Fall, Season::Winter, Season::Fall],
            vec![
                anytime("A-1", "monday"),
                anytime("B-1", "tuesday"),
                with_prereq(
                    "T-9",
                    "wednesday",
                    &parsed(r#"{"program_credits":{"credits":6}}"#),
                ),
            ],
        );
        inputs.credit_cap = 3;
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![
                pairs(&[("A-1", 1), ("B-1", 2), ("T-9", 3)]),
                pairs(&[("A-1", 2), ("B-1", 1), ("T-9", 3)]),
            ],
            "3 + 3 credits strictly before T-9, both ways"
        );
    }

    #[test]
    fn a_closed_summer_counts_nothing_toward_a_threshold_prefix() {
        // horizon A-H-É-A, étés closed: the é holds no credits, so the 7
        // demanded credits are not reachable by session 4 (3 + 3 + 0) and
        // the search proves infeasibility instead of grinding leaves
        let mut inputs = Inputs::new(
            &[Season::Fall, Season::Winter, Season::Summer, Season::Fall],
            vec![
                anytime("A-1", "monday"),
                anytime("B-1", "tuesday"),
                with_prereq(
                    "T-9",
                    "wednesday",
                    &parsed(r#"{"program_credits":{"credits":7}}"#),
                ),
            ],
        );
        inputs.credit_cap = 3;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(placement.solutions.is_empty());
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "T-9".to_string(),
                reason: BlockedReason::UnsatisfiablePrerequisites,
            }],
            "the threshold-emptied domain names the true culprit"
        );
    }

    #[test]
    fn an_eligible_stage_lifts_a_closed_summers_threshold_prefix() {
        // same horizon, but a 3-credit stage may sit in the closed é3
        // (its own threshold of 3 is met by then): 3 + 3 + 3 credits
        // before session 4 reach the 7 demanded
        let mut inputs = Inputs::new(
            &[Season::Fall, Season::Winter, Season::Summer, Season::Fall],
            vec![
                anytime("A-1", "monday"),
                anytime("B-1", "tuesday"),
                {
                    let mut stage = all_seasons("S-1", "thursday");
                    stage.prerequisites = serde_json::from_str(&parsed(
                        r#"{"program_credits":{"credits":3}}"#,
                    ))
                    .unwrap_or_else(|e| panic!("{e}"));
                    stage
                },
                with_prereq(
                    "T-9",
                    "wednesday",
                    &parsed(r#"{"program_credits":{"credits":7}}"#),
                ),
            ],
        );
        inputs.credit_cap = 3;
        inputs.stages = stages(&["S-1"]);
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-1", 2), ("S-1", 3), ("T-9", 4)])]
        );
    }

    #[test]
    fn a_doomed_threshold_prunes_mid_search_not_leaf_by_leaf() {
        // T-8 sits in session 2 needing what only session 1 can hold; the
        // moment too much is assigned elsewhere the potential bound turns
        // its verdict False and the branch dies before any leaf
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                anytime("A-1", "monday"),
                anytime("B-1", "tuesday"),
                with_prereq(
                    "T-8",
                    "wednesday",
                    &parsed(r#"{"program_credits":{"credits":6}}"#),
                ),
            ],
        );
        inputs.credit_cap = 6;
        let placement = inputs.solve();
        // both 3-credit courses must precede T-8: one solution
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-1", 1), ("T-8", 2)])]
        );
    }

    #[test]
    fn a_mutual_prerequisite_pair_relaxes_to_both_left_out() {
        // A-1 and B-2 each require the other strictly before — no real
        // arrangement exists, and leaving only one of them out is vetoed
        // the moment the sentinel falsifies the placed one's tree
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                with_prereq("A-1", "monday", &parsed(r#""B-2""#)),
                with_prereq("B-2", "tuesday", &parsed(r#""A-1""#)),
            ],
        );
        inputs.allow_unplaced = true;
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        let solution = placement
            .solutions
            .first()
            .unwrap_or_else(|| panic!("the relaxation must still answer"));
        assert!(solution.placement.is_empty());
        assert_eq!(
            solution.left_out,
            BTreeSet::from(["A-1".to_string(), "B-2".to_string()])
        );
    }

    #[test]
    fn an_all_with_an_undecided_child_stays_undecided() {
        // the prerequisites-first order assigns an ET's operands before
        // their dependent, so this row of the truth table now only
        // arises through mention cycles — pinned directly
        assert_eq!(
            all_verdict(
                &[1, 2],
                &[
                    Verdict::Unknown,
                    Verdict::Unknown,
                    Verdict::Sat(BTreeSet::new()),
                ]
            ),
            Verdict::Unknown
        );
    }

    #[test]
    fn an_undecided_threshold_waits_for_the_remaining_aggregates() {
        // three aggregates: while the last is still unassigned, an
        // earlier one short of its threshold stays undecided as long as
        // what remains could land before it — the enumeration walks that
        // branch and still proves the full set
        let mut inputs = Inputs::new(
            &[Season::Fall, Season::Winter, Season::Fall, Season::Winter],
            vec![
                anytime("A-1", "monday"),
                with_prereq(
                    "T-7",
                    "tuesday",
                    &parsed(r#"{"program_credits":{"credits":3}}"#),
                ),
                with_prereq(
                    "T-8",
                    "wednesday",
                    &parsed(r#"{"program_credits":{"credits":3}}"#),
                ),
                with_prereq(
                    "T-9",
                    "thursday",
                    &parsed(r#"{"program_credits":{"credits":6}}"#),
                ),
            ],
        );
        inputs.credit_cap = 3;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(!placement.solutions.is_empty());
        for solution in &placement.solutions {
            assert!(solution.left_out.is_empty());
            assert_eq!(solution.placement.len(), 4);
        }
    }

    #[test]
    fn an_all_branch_not_yet_assigned_stays_undecided_mid_search() {
        // A-1 needs both C-3 (already seated) and B-2 — and the A↔B cycle
        // defeats the prerequisites-first order, so B-2 is still
        // unassigned when A-1's ET is evaluated: the ET must stay
        // undecided rather than veto, and the impossible pair still
        // relaxes to both left out
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                with_prereq(
                    "A-1",
                    "monday",
                    &parsed(r#"{"all":["B-2","C-3"]}"#),
                ),
                with_prereq("B-2", "tuesday", &parsed(r#""A-1""#)),
                anytime("C-3", "wednesday"),
            ],
        );
        inputs.allow_unplaced = true;
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        let solution = placement
            .solutions
            .first()
            .unwrap_or_else(|| panic!("the relaxation must still answer"));
        assert_eq!(
            solution.placement,
            BTreeMap::from([("C-3".to_string(), 1)])
        );
        assert_eq!(
            solution.left_out,
            BTreeSet::from(["A-1".to_string(), "B-2".to_string()])
        );
    }

    #[test]
    fn the_leaf_check_still_refuses_a_violated_threshold() {
        // unreachable through `place` today — the incremental layer
        // refuses first — but the leaf check is the last line of « jamais
        // un placement en violation », so it is pinned directly
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq(
                "T-9",
                "tuesday",
                &parsed(r#"{"program_credits":{"credits":6}}"#),
            ),
        ];
        let sessions = FALL_WINTER.to_vec();
        let empty_set = BTreeSet::new();
        let empty_map = BTreeMap::new();
        let request = PlacementRequest {
            sessions: &sessions,
            credit_cap: 30,
            concomitant: false,
            courses: &courses,
            passed: &empty_set,
            pinned: &empty_map,
            stages: &empty_set,
            open_summers: &BTreeSet::new(),
            seed: &empty_map,
            max_nodes: 0,
            max_solutions: 1,
            allow_unplaced: false,
            allow_credit_shortfall: false,
        };
        let candidates =
            build_candidates(&request).unwrap_or_else(|e| panic!("{e}"));
        let index_of: BTreeMap<String, usize> = candidates
            .iter()
            .enumerate()
            .map(|(i, candidate)| (candidate.code.clone(), i))
            .collect();
        let ctx = SearchCtx {
            request: &request,
            candidates: &candidates,
            needs_final: needs_final_flags(&candidates, &index_of, &request),
            suffix_credits: suffix_credit_sums(&candidates),
            index_of,
            referenced_by: referencing_map(&candidates),
            passed_credits: 0,
            credit_watch: vec![1],
        };
        // A-1 in session 1 gives 3 credits before T-9's session 2 — short
        // of the 6 demanded: the leaf must refuse, never lie
        assert!(finalize(&[1, 2], &ctx).is_none());
        assert!(finalize(&[1, UNPLACED], &ctx).is_some());
    }

    #[test]
    fn a_soft_threshold_is_placed_latest_and_records_the_gap() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                anytime("A-1", "monday"),
                with_prereq(
                    "T-9",
                    "tuesday",
                    &parsed(r#"{"program_credits":{"credits":6}}"#),
                ),
            ],
        );
        assert!(inputs.solve().solutions.is_empty(), "strict stays strict");
        inputs.allow_credit_shortfall = true;
        inputs.max_solutions = 1;
        let solution = inputs.solve().solutions.remove(0);
        assert_eq!(solution.placement["T-9"], 2);
        assert_eq!(
            solution.credit_shortfalls,
            [CreditShortfall {
                code: "T-9".to_string(),
                session: 2,
                earned_before: 3,
                required: 6,
            }]
        );
    }

    #[test]
    fn a_satisfied_or_branch_wins_without_a_false_shortfall() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                anytime("A-1", "monday"),
                with_prereq(
                    "T-9",
                    "tuesday",
                    &parsed(
                        r#"{"any":["A-1",{"program_credits":{"credits":60}}]}"#,
                    ),
                ),
            ],
        );
        inputs.allow_credit_shortfall = true;
        inputs.max_solutions = 1;
        let solution = inputs.solve().solutions.remove(0);
        assert!(solution.credit_shortfalls.is_empty());
        assert_eq!(solution.placement["A-1"], 1);
        assert_eq!(solution.placement["T-9"], 2);
    }

    #[test]
    fn the_soft_final_guard_rejects_a_hard_leaf_and_tree_scan_is_bounded() {
        let oversized = PrereqTree::All {
            all: (0..MAX_TREE_NODES)
                .map(|_| PrereqTree::Course("A-1".to_string()))
                .collect(),
        };
        assert!(!tree_has_credits(&oversized));

        let courses = vec![with_prereq(
            "T-9",
            "tuesday",
            &parsed(
                r#"{"all":[{"program_credits":{"credits":6}},"ZZZ-9999"]}"#,
            ),
        )];
        let sessions = FALL_WINTER.to_vec();
        let empty_set = BTreeSet::new();
        let empty_summers: BTreeSet<usize> = BTreeSet::new();
        let empty_map = BTreeMap::new();
        let request = PlacementRequest {
            sessions: &sessions,
            credit_cap: 30,
            concomitant: false,
            courses: &courses,
            passed: &empty_set,
            pinned: &empty_map,
            stages: &empty_set,
            open_summers: &empty_summers,
            seed: &empty_map,
            max_nodes: 0,
            max_solutions: 1,
            allow_unplaced: false,
            allow_credit_shortfall: true,
        };
        let candidates =
            build_candidates(&request).unwrap_or_else(|e| panic!("{e}"));
        let index_of = BTreeMap::from([("T-9".to_string(), 0)]);
        let ctx = SearchCtx {
            request: &request,
            candidates: &candidates,
            needs_final: needs_final_flags(&candidates, &index_of, &request),
            suffix_credits: suffix_credit_sums(&candidates),
            index_of,
            referenced_by: referencing_map(&candidates),
            passed_credits: 0,
            credit_watch: vec![0],
        };
        assert!(finalize(&[2], &ctx).is_none());
    }

    #[test]
    fn a_left_out_threshold_course_no_longer_vetoes_its_branch() {
        // the relaxed pass sets the stage aside: its own threshold stops
        // applying (nothing is required in order to *not* take a course)
        // — the guard that unlocked B-GMC's first relaxed leaf
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                anytime("A-1", "monday"),
                with_prereq(
                    "T-9",
                    "wednesday",
                    &parsed(r#"{"program_credits":{"credits":30}}"#),
                ),
            ],
        );
        inputs.allow_unplaced = true;
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        let solution = placement
            .solutions
            .first()
            .unwrap_or_else(|| panic!("the relaxation must still answer"));
        assert_eq!(
            solution.placement,
            pairs(&[("A-1", 1)]).into_iter().collect::<BTreeMap<_, _>>()
        );
        assert_eq!(solution.left_out, BTreeSet::from(["T-9".to_string()]));
    }

    #[test]
    fn an_early_open_summer_still_comes_last_in_the_order() {
        // without a seed the order used to be plainly ascending — an été
        // at the head of the horizon would have been tried first
        let mut inputs = Inputs::new(
            &[Season::Summer, Season::Fall],
            vec![all_seasons("A-1", "monday")],
        );
        inputs.open_summers = BTreeSet::from([1]);
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(sorted_placements(&placement), vec![pairs(&[("A-1", 2)])]);
    }

    #[test]
    fn a_stage_is_never_demoted_out_of_its_summers() {
        let mut inputs = Inputs::new(
            &[Season::Fall, Season::Summer, Season::Fall, Season::Summer],
            vec![all_seasons("S-1580", "monday")],
        );
        inputs.stages = stages(&["S-1580"]);
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("S-1580", 2)])]
        );
    }

    #[test]
    fn a_pinned_regular_course_enters_a_closed_summer() {
        // pinning is as explicit an act as opening the été: the same
        // exception in both directions
        let mut inputs = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("A-1", "monday")],
        );
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 3)]);
        let placement = inputs.solve();
        assert_eq!(sorted_placements(&placement), vec![pairs(&[("A-1", 3)])]);
    }

    #[test]
    fn a_passed_stage_is_neither_placed_nor_an_error() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![all_seasons("S-1580", "monday"), anytime("A-1", "tuesday")],
        );
        inputs.stages = stages(&["S-1580"]);
        inputs.passed = passed(&["S-1580"]);
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1)]), pairs(&[("A-1", 2)])]
        );
    }

    #[test]
    fn the_credit_cap_forces_a_split() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
        );
        inputs.credit_cap = 3;
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![
                pairs(&[("A-1", 1), ("B-2", 2)]),
                pairs(&[("A-1", 2), ("B-2", 1)]),
            ]
        );
    }

    #[test]
    fn the_remaining_credits_bound_prunes_at_the_root() {
        // two 3-credit courses, one 3-credit session: 6 credits can never
        // fit — the bound proves it at the root, one node suffices, where
        // the per-session filter alone would still expand a child
        let mut inputs = Inputs::new(
            &[Season::Fall],
            vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
        );
        inputs.credit_cap = 3;
        inputs.max_nodes = 1;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(placement.solutions.is_empty());
    }

    #[test]
    fn the_weekly_veto_splits_two_clashing_courses() {
        // same monday slot in both seasons: never the same session
        let inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "monday")],
        );
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![
                pairs(&[("A-1", 1), ("B-2", 2)]),
                pairs(&[("A-1", 2), ("B-2", 1)]),
            ]
        );
    }

    #[test]
    fn a_pinned_session_reduces_the_domain_to_a_singleton() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
        );
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 2)]);
        let placement = inputs.solve();
        assert!(placement
            .solutions
            .iter()
            .all(|solution| solution.placement["A-1"] == 2));
        assert_eq!(placement.solutions.len(), 2);
    }

    #[test]
    fn a_pin_against_the_offer_is_proven_infeasible_not_an_error() {
        // fall-only course pinned to the winter session: empty domain,
        // exhaustive search, « aucun cheminement faisable » proven
        let fall_only: Course = serde_json::from_str(
            r#"{"code":"A-1","title":"T","credits":3,"cycle":1,
                "prerequisites":null,"equivalents":[],
                "seasons":{"fall":{"last_offered":2026,"options":[[]]}}}"#,
        )
        .unwrap_or_else(|e| panic!("course literal: {e}"));
        let mut inputs = Inputs::new(&FALL_WINTER, vec![fall_only]);
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 2)]);
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(placement.solutions.is_empty());
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "A-1".to_string(),
                reason: BlockedReason::EmptyDomain,
            }]
        );
    }

    // --- precedence ---

    #[test]
    fn a_prerequisite_chain_forces_the_order() {
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq("B-2", "tuesday", &parsed("\"A-1\"")),
        ];
        let placement = Inputs::new(&FALL_WINTER, courses).solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-2", 2)])]
        );
    }

    #[test]
    fn placing_the_dependent_before_its_prerequisite_still_orders_them() {
        // the dependent comes *first* in input order: its verdict stays
        // Unknown until A-1 lands, and the re-check through the
        // referencing map prunes the misordered branches
        let courses = vec![
            with_prereq("B-2", "tuesday", &parsed(r#"{"all":["A-1"]}"#)),
            anytime("A-1", "monday"),
        ];
        let placement = Inputs::new(&FALL_WINTER, courses).solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-2", 2)])]
        );
    }

    #[test]
    fn an_any_tree_is_satisfied_by_a_single_branch() {
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq("B-2", "tuesday", &parsed(r#"{"any":["A-1","C-3"]}"#)),
            anytime("C-3", "wednesday"),
        ];
        let placement = Inputs::new(&FALL_WINTER, courses).solve();
        // every solution has A-1 or C-3 strictly before B-2, and none is
        // rejected for the other branch being late
        assert!(!placement.solutions.is_empty());
        for solution in &placement.solutions {
            let b = solution.placement["B-2"];
            assert!(
                solution.placement["A-1"] < b || solution.placement["C-3"] < b,
                "{:?}",
                solution.placement
            );
            assert!(solution.assumed.is_empty());
        }
    }

    #[test]
    fn an_any_tree_with_no_branch_before_blocks() {
        // single session: nothing can be strictly before
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq("B-2", "tuesday", &parsed(r#"{"any":["A-1","C-3"]}"#)),
            anytime("C-3", "wednesday"),
        ];
        let placement = Inputs::new(&[Season::Fall], courses).solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(placement.solutions.is_empty());
    }

    #[test]
    fn the_concomitant_option_relaxes_strictly_before_to_same_session() {
        let courses = || {
            vec![
                anytime("A-1", "monday"),
                with_prereq("B-2", "tuesday", &parsed("\"A-1\"")),
            ]
        };
        let strict = Inputs::new(&[Season::Fall], courses()).solve();
        assert!(strict.solutions.is_empty());
        let mut relaxed = Inputs::new(&[Season::Fall], courses());
        relaxed.concomitant = true;
        let placement = relaxed.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-2", 1)])]
        );
    }

    #[test]
    fn a_course_is_never_its_own_concomitant_prerequisite() {
        // a degenerate self-referencing tree must not satisfy itself at
        // the same session under the concomitant relaxation
        let courses = vec![with_prereq("B-2", "tuesday", &parsed("\"B-2\""))];
        let mut inputs = Inputs::new(&[Season::Fall], courses);
        inputs.concomitant = true;
        let placement = inputs.solve();
        assert!(placement.solutions.is_empty());
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "B-2".to_string(),
                reason: BlockedReason::UnsatisfiablePrerequisites,
            }]
        );
    }

    #[test]
    fn a_program_credits_threshold_gates_the_session() {
        // B-2 needs 6 credits strictly before its session, and sits first
        // in input order so its verdict resolves only at the leaf
        let courses = vec![
            with_prereq(
                "B-2",
                "tuesday",
                &parsed(r#"{"program_credits":{"credits":6}}"#),
            ),
            anytime("A-1", "monday"),
            anytime("C-3", "wednesday"),
        ];
        let sessions = [Season::Fall, Season::Winter, Season::Fall];
        let placement = Inputs::new(&sessions, courses).solve();
        for solution in &placement.solutions {
            let b = solution.placement["B-2"];
            assert!(
                solution.placement["A-1"] < b && solution.placement["C-3"] < b,
                "{:?}",
                solution.placement
            );
        }
        // A and C both strictly before B: {1,1}→B∈{2,3}, {1,2}/{2,1}/{2,2}→B=3
        assert_eq!(placement.solutions.len(), 5);
    }

    // --- passed courses: removed, precounted, satisfying ---

    #[test]
    fn a_passed_course_satisfies_the_prerequisite_and_is_not_placed() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                anytime("A-1", "monday"),
                with_prereq("B-2", "tuesday", &parsed("\"A-1\"")),
            ],
        );
        inputs.passed = passed(&["A-1"]);
        let placement = inputs.solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("B-2", 1)]), pairs(&[("B-2", 2)])]
        );
    }

    #[test]
    fn passed_credits_count_toward_a_program_credits_threshold() {
        let mut inputs = Inputs::new(
            &[Season::Fall],
            vec![
                anytime("A-1", "monday"),
                with_prereq(
                    "B-2",
                    "tuesday",
                    &parsed(r#"{"program_credits":{"credits":3}}"#),
                ),
            ],
        );
        inputs.passed = passed(&["A-1"]);
        let placement = inputs.solve();
        assert_eq!(sorted_placements(&placement), vec![pairs(&[("B-2", 1)])]);
    }

    #[test]
    fn an_all_passed_request_yields_one_empty_placement() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.passed = passed(&["A-1"]);
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(placement.solutions.len(), 1);
        assert!(placement.solutions[0].placement.is_empty());
        assert!(placement.solutions[0].assumed.is_empty());
    }

    // --- unverifiable operands: presumed, surfaced, never imposed ---

    #[test]
    fn an_unknown_preuniversity_code_is_assumed_and_surfaced() {
        // ADR `2026-07-presomption-limitee-au-preuniversitaire`
        let courses =
            vec![with_prereq("B-2", "tuesday", &parsed("\"MAT-0130\""))];
        let placement = Inputs::new(&[Season::Fall], courses).solve();
        assert_eq!(placement.solutions.len(), 1);
        assert_eq!(placement.solutions[0].assumed, passed(&["MAT-0130"]),);
    }

    #[test]
    fn an_unknown_university_code_blocks_the_placement() {
        // a 1xxx+ code absent from the list is a course the student would
        // still have to take: never presumed, the placement is rejected
        let courses =
            vec![with_prereq("B-2", "tuesday", &parsed("\"CEG-1101\""))];
        let placement = Inputs::new(&[Season::Fall], courses).solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert!(placement.solutions.is_empty());
        // proven before the search, with the culprit named
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "B-2".to_string(),
                reason: BlockedReason::UnsatisfiablePrerequisites,
            }]
        );
    }

    #[test]
    fn a_threshold_above_every_credit_in_sight_blocks_before_the_search() {
        // 100 credits demanded, 6 in the whole request: no assignment can
        // ever satisfy it, proven without expanding a node
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq(
                "B-2",
                "tuesday",
                &parsed(r#"{"program_credits":{"credits":100}}"#),
            ),
        ];
        let mut inputs = Inputs::new(&FALL_WINTER, courses);
        inputs.max_nodes = 0;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(
            placement.blocked,
            vec![Blocked {
                code: "B-2".to_string(),
                reason: BlockedReason::UnsatisfiablePrerequisites,
            }]
        );
    }

    #[test]
    fn an_unknown_university_code_inside_an_any_blocks_only_its_branch() {
        // the pre-search screen keeps the candidate (the other branch is
        // satisfiable), so the search itself walks the unknown leaf: the
        // university code refuses (never presumed), A-1 satisfies
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq(
                "B-2",
                "tuesday",
                &parsed(r#"{"any":["CEG-1101","A-1"]}"#),
            ),
        ];
        let placement = Inputs::new(&FALL_WINTER, courses).solve();
        assert!(placement.blocked.is_empty());
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-2", 2)])]
        );
    }

    #[test]
    fn a_raw_only_prerequisite_is_assumed_and_surfaced() {
        let courses = vec![with_prereq(
            "B-2",
            "tuesday",
            r#"{"raw":"Examen Test français"}"#,
        )];
        let placement = Inputs::new(&[Season::Fall], courses).solve();
        assert_eq!(placement.solutions.len(), 1);
        assert_eq!(
            placement.solutions[0].assumed,
            passed(&["Examen Test français"]),
        );
    }

    #[test]
    fn a_raw_operand_inside_a_tree_is_assumed_through_all() {
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq(
                "B-2",
                "tuesday",
                &parsed(r#"{"all":["A-1",{"raw":"ESG-2020 à 3799"}]}"#),
            ),
        ];
        let placement = Inputs::new(&FALL_WINTER, courses).solve();
        assert_eq!(
            sorted_placements(&placement),
            vec![pairs(&[("A-1", 1), ("B-2", 2)])]
        );
        assert_eq!(
            placement.solutions[0].assumed,
            passed(&["ESG-2020 à 3799"]),
        );
    }

    #[test]
    fn a_satisfied_any_branch_shadows_an_unverifiable_sibling() {
        // the ECN-2901 tolerance: nothing was presumed when a clean
        // branch already satisfies the any
        let courses = vec![
            anytime("A-1", "monday"),
            with_prereq(
                "B-2",
                "tuesday",
                &parsed(r#"{"any":["A-1","MAT-0130"]}"#),
            ),
        ];
        let placement = Inputs::new(&FALL_WINTER, courses).solve();
        let ordered: Vec<&Solution> = placement
            .solutions
            .iter()
            .filter(|solution| {
                solution.placement["A-1"] < solution.placement["B-2"]
            })
            .collect();
        assert!(!ordered.is_empty());
        for solution in ordered {
            assert!(solution.assumed.is_empty(), "{solution:?}");
        }
    }

    // --- the double bound (ADR `2026-07-budget-de-b-en-double-borne`) ---

    #[test]
    fn a_tiny_node_budget_reports_budget_never_infeasible() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
        );
        inputs.max_nodes = 1;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::NodeBudget);
        assert!(placement.solutions.is_empty());
    }

    #[test]
    fn the_solution_cap_returns_a_partial_set() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
        );
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::SolutionCap);
        assert_eq!(placement.solutions.len(), 1);
    }

    #[test]
    fn a_cap_reached_exactly_at_exhaustion_is_still_complete() {
        // one solution exists and the cap is one: the search finished, so
        // the set is total, not partial
        let mut inputs =
            Inputs::new(&[Season::Fall], vec![anytime("A-1", "monday")]);
        inputs.max_solutions = 1;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(placement.solutions.len(), 1);
    }

    #[test]
    fn an_exactly_sufficient_budget_still_proves_completeness() {
        // one course, one session: root + leaf = 2 nodes exactly — the
        // budget spent to the last node still proves the set total
        let mut inputs =
            Inputs::new(&[Season::Fall], vec![anytime("A-1", "monday")]);
        inputs.max_nodes = 2;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::Complete);
        assert_eq!(placement.solutions.len(), 1);
    }

    #[test]
    fn a_zero_node_budget_expands_nothing() {
        let mut inputs =
            Inputs::new(&[Season::Fall], vec![anytime("A-1", "monday")]);
        inputs.max_nodes = 0;
        let placement = inputs.solve();
        assert_eq!(placement.completion, Completion::NodeBudget);
        assert!(placement.solutions.is_empty());
    }

    // --- search order: seed first, then neighbours, earlier on ties ---

    #[test]
    fn the_seed_session_is_explored_first() {
        let sessions = [Season::Fall, Season::Winter, Season::Fall];
        let mut inputs =
            Inputs::new(&sessions, vec![anytime("A-1", "monday")]);
        inputs.seed = BTreeMap::from([("A-1".to_string(), 2)]);
        let placement = inputs.solve();
        let order: Vec<usize> = placement
            .solutions
            .iter()
            .map(|solution| solution.placement["A-1"])
            .collect();
        assert_eq!(order, [2, 1, 3]);
    }

    #[test]
    fn without_a_seed_the_earliest_offered_session_comes_first() {
        let placement =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]).solve();
        let order: Vec<usize> = placement
            .solutions
            .iter()
            .map(|solution| solution.placement["A-1"])
            .collect();
        assert_eq!(order, [1, 2]);
    }

    #[test]
    fn a_seed_naming_an_absent_course_is_an_ordering_hint_not_an_error() {
        // a full-bac seed over a partial list is the normal case
        let mut inputs =
            Inputs::new(&[Season::Fall], vec![anytime("A-1", "monday")]);
        inputs.seed = BTreeMap::from([("Z-9".to_string(), 1)]);
        assert_eq!(inputs.solve().solutions.len(), 1);
    }

    // --- flatten: bounded, loud when exceeded ---

    #[test]
    fn a_prerequisite_tree_past_the_bound_is_an_error() {
        let tree = (0..MAX_TREE_NODES)
            .fold(PrereqTree::Course("X-1".to_string()), |child, _| {
                PrereqTree::All { all: vec![child] }
            });
        let mut course = anytime("B-2", "tuesday");
        course.prerequisites = Some(Prerequisites::Parsed {
            raw: "deep".to_string(),
            tree,
        });
        let inputs = Inputs::new(&[Season::Fall], vec![course]);
        assert_eq!(
            inputs.place(),
            Err(PlacementError::PrerequisiteTreeTooLarge {
                code: "B-2".to_string()
            })
        );
    }

    // --- the static prerequisite question (UI seam, jalon 6) ---

    fn met_with(assumed: &[&str]) -> Result<PrereqStatus, PlacementError> {
        Ok(PrereqStatus::Met {
            assumed: passed(assumed),
        })
    }

    #[test]
    fn a_course_without_prerequisites_is_statically_met() {
        assert_eq!(
            prerequisites_met(&anytime("A-1", "monday"), &passed(&[]), 0),
            met_with(&[])
        );
    }

    #[test]
    fn a_held_course_leaf_meets_and_a_missing_one_blocks() {
        let course = with_prereq("B-2", "tuesday", &parsed("\"A-1\""));
        assert_eq!(
            prerequisites_met(&course, &passed(&["A-1"]), 0),
            met_with(&[])
        );
        assert_eq!(
            prerequisites_met(&course, &passed(&[]), 0),
            Ok(PrereqStatus::Unmet)
        );
    }

    #[test]
    fn preuniversity_and_raw_operands_are_presumed_and_surfaced() {
        let preuniversity =
            with_prereq("B-2", "tuesday", &parsed("{\"all\":[\"MAT-0130\"]}"));
        assert_eq!(
            prerequisites_met(&preuniversity, &passed(&[]), 0),
            met_with(&["MAT-0130"])
        );
        // a whole prerequisite outside the grammar: one presumed operand
        let raw = with_prereq("B-2", "tuesday", "{\"raw\":\"un examen\"}");
        assert_eq!(
            prerequisites_met(&raw, &passed(&[]), 0),
            met_with(&["un examen"])
        );
    }

    #[test]
    fn a_credits_threshold_is_checked_against_the_accumulated_credits() {
        let course = with_prereq(
            "B-2",
            "tuesday",
            &parsed("{\"program_credits\":{\"program\":null,\"credits\":24}}"),
        );
        assert_eq!(
            prerequisites_met(&course, &passed(&[]), 24),
            met_with(&[])
        );
        assert_eq!(
            prerequisites_met(&course, &passed(&[]), 23),
            Ok(PrereqStatus::Unmet)
        );
    }

    #[test]
    fn an_any_branch_proven_without_assumptions_wins() {
        let course = with_prereq(
            "B-2",
            "tuesday",
            &parsed("{\"any\":[\"MAT-0130\",\"A-1\"]}"),
        );
        // the clean branch beats the presumed préuniversitaire one
        assert_eq!(
            prerequisites_met(&course, &passed(&["A-1"]), 0),
            met_with(&[])
        );
        // without it, the presumed branch carries its operand
        assert_eq!(
            prerequisites_met(&course, &passed(&[]), 0),
            met_with(&["MAT-0130"])
        );
    }

    #[test]
    fn a_static_tree_past_the_bound_is_the_same_error() {
        let tree = (0..MAX_TREE_NODES)
            .fold(PrereqTree::Course("X-1".to_string()), |child, _| {
                PrereqTree::All { all: vec![child] }
            });
        let mut course = anytime("B-2", "tuesday");
        course.prerequisites = Some(Prerequisites::Parsed {
            raw: "deep".to_string(),
            tree,
        });
        assert_eq!(
            prerequisites_met(&course, &passed(&[]), 0),
            Err(PlacementError::PrerequisiteTreeTooLarge {
                code: "B-2".to_string()
            })
        );
    }

    // --- the session probes behind the « + H28 » chips (UI seam) ---

    #[test]
    fn admissible_sessions_respect_offer_and_precedence() {
        // A-1 pinned to session 1; B-2 requires it strictly before
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![
                anytime("A-1", "monday"),
                with_prereq("B-2", "tuesday", &parsed("\"A-1\"")),
            ],
        );
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 1)]);
        assert_eq!(inputs.admissible("B-2"), Ok(BTreeSet::from([2])));
    }

    #[test]
    fn a_full_session_is_not_admissible() {
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "tuesday")],
        );
        inputs.credit_cap = 3;
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 1)]);
        assert_eq!(inputs.admissible("B-2"), Ok(BTreeSet::from([2])));
    }

    #[test]
    fn a_weekly_conflict_excludes_the_session_from_the_probe() {
        // same monday slot: the A-veto refuses the shared session
        let mut inputs = Inputs::new(
            &FALL_WINTER,
            vec![anytime("A-1", "monday"), anytime("B-2", "monday")],
        );
        inputs.pinned = BTreeMap::from([("A-1".to_string(), 1)]);
        assert_eq!(inputs.admissible("B-2"), Ok(BTreeSet::from([2])));
    }

    #[test]
    fn probing_a_stage_lifts_the_summer_restriction_like_the_pin_it_stands_for(
    ) {
        let mut inputs = Inputs::new(
            &FALL_WINTER_SUMMER,
            vec![all_seasons("S-1", "monday")],
        );
        inputs.stages = stages(&["S-1"]);
        // unpinned the stage goes to the été only; the chip's pin opens all
        assert_eq!(inputs.admissible("S-1"), Ok(BTreeSet::from([1, 2, 3])));
    }

    #[test]
    fn probing_a_passed_course_is_the_typed_error() {
        let mut inputs =
            Inputs::new(&FALL_WINTER, vec![anytime("A-1", "monday")]);
        inputs.passed = passed(&["A-1"]);
        assert_eq!(
            inputs.admissible("A-1"),
            Err(PlacementError::PassedAndPinned {
                code: "A-1".to_string()
            })
        );
    }

    // --- cross-cutting properties: the search against an independent
    // --- brute force (soundness *and* completeness of the returned set)

    use crate::course::{
        CourseCycle, Credits, Day, Mode, SeasonOffering, Section, Slot, Time,
    };
    use proptest::prelude::*;

    // a prerequisite shape the naive checker can evaluate without
    // re-implementing the production tree walk — indices name courses
    #[derive(Debug, Clone, Copy)]
    enum Spec {
        Free,
        Needs(usize),
        Either(usize, usize),
        CreditsBefore(u32),
    }

    #[derive(Debug, Clone)]
    struct Generated {
        // study sessions, plus one trailing Summer when `summer` is set
        sessions: Vec<Season>,
        summer: bool,
        specs: Vec<Spec>,
        credits: Vec<u32>,
        offered: Vec<(bool, bool, bool)>,
        days: Vec<usize>,
        passed: Vec<bool>,
        pin: Option<(usize, usize)>,
        // at most one stage — enough to cross the summer restriction with
        // every other rule (pin, passed, offer, cap)
        stage: Option<usize>,
        open_summer: bool,
        credit_cap: u32,
        concomitant: bool,
    }

    const DAYS: [Day; 3] = [Day::Monday, Day::Tuesday, Day::Wednesday];

    fn code_of(index: usize) -> String {
        format!("C-{index}")
    }

    fn arb_generated() -> impl Strategy<Value = Generated> {
        // raw parts first, normalized in the map so every value is valid
        // whatever the drawn sizes
        (1usize..=3, 1usize..=4).prop_flat_map(|(n_sessions, n_courses)| {
            (
                proptest::collection::vec(
                    proptest::bool::ANY,
                    n_sessions..=n_sessions,
                ),
                proptest::collection::vec(
                    (0u8..4, proptest::num::usize::ANY, 1u32..=6),
                    n_courses..=n_courses,
                ),
                proptest::collection::vec(
                    (1u32..=3, 0u8..8, 0usize..3, proptest::bool::ANY),
                    n_courses..=n_courses,
                ),
                proptest::option::of((0usize..n_courses, 1usize..=n_sessions)),
                proptest::bool::ANY,
                proptest::option::of(0usize..n_courses),
                proptest::bool::ANY,
                3u32..=9,
                proptest::bool::ANY,
            )
                .prop_map(
                    move |(
                        winters,
                        prereqs,
                        shapes,
                        pin,
                        summer,
                        stage,
                        open_summer,
                        cap,
                        concomitant,
                    )| {
                        let specs = prereqs
                            .iter()
                            .enumerate()
                            .map(|(i, &(kind, other, credits))| {
                                spec(i, n_courses, kind, other, credits)
                            })
                            .collect();
                        let mut sessions: Vec<Season> = winters
                            .iter()
                            .map(|&winter| {
                                if winter {
                                    Season::Winter
                                } else {
                                    Season::Fall
                                }
                            })
                            .collect();
                        if summer {
                            sessions.push(Season::Summer);
                        }
                        Generated {
                            sessions,
                            summer,
                            specs,
                            credits: shapes
                                .iter()
                                .map(|&(credits, ..)| credits)
                                .collect(),
                            offered: shapes
                                .iter()
                                .map(|&(_, mask, ..)| {
                                    // 0 = nowhere: the infeasible case
                                    (
                                        mask & 1 != 0,
                                        mask & 2 != 0,
                                        mask & 4 != 0,
                                    )
                                })
                                .collect(),
                            days: shapes
                                .iter()
                                .map(|&(.., day, _)| day)
                                .collect(),
                            passed: shapes
                                .iter()
                                .map(|&(.., passed)| passed)
                                .collect(),
                            pin,
                            stage,
                            open_summer,
                            credit_cap: cap,
                            concomitant,
                        }
                    },
                )
        })
    }

    fn spec(
        index: usize,
        n_courses: usize,
        kind: u8,
        other: usize,
        credits: u32,
    ) -> Spec {
        // references may point anywhere — before or after the course in
        // input order — except at the course itself
        let target = |seed: usize| {
            let candidates: Vec<usize> =
                (0..n_courses).filter(|&j| j != index).collect();
            candidates.get(seed % candidates.len().max(1)).copied()
        };
        match (kind, target(other), target(other / 2)) {
            (1, Some(a), _) => Spec::Needs(a),
            (2, Some(a), Some(b)) => Spec::Either(a, b),
            (3, ..) => Spec::CreditsBefore(credits),
            _ => Spec::Free,
        }
    }

    fn generated_courses(generated: &Generated) -> Vec<Course> {
        generated
            .specs
            .iter()
            .enumerate()
            .map(|(i, &spec)| {
                let offering = SeasonOffering {
                    last_offered: Some(2026),
                    options: Some(vec![vec![Section {
                        nrc: format!("{i}"),
                        section: None,
                        mode: Mode::InPerson,
                        slots: vec![Slot {
                            day: DAYS[generated.days[i]],
                            start: Time {
                                hour: 8,
                                minute: 30,
                            },
                            end: Time {
                                hour: 11,
                                minute: 20,
                            },
                        }],
                    }]]),
                };
                let mut seasons = BTreeMap::new();
                if generated.offered[i].0 {
                    seasons.insert(Season::Fall, offering.clone());
                }
                if generated.offered[i].1 {
                    seasons.insert(Season::Winter, offering.clone());
                }
                if generated.offered[i].2 {
                    seasons.insert(Season::Summer, offering.clone());
                }
                Course {
                    code: code_of(i),
                    title: "T".to_string(),
                    credits: Credits::Fixed(generated.credits[i]),
                    cycle: CourseCycle::First,
                    prerequisites: prerequisites_of(spec),
                    equivalents: Vec::new(),
                    seasons,
                }
            })
            .collect()
    }

    fn prerequisites_of(spec: Spec) -> Option<Prerequisites> {
        let tree = match spec {
            Spec::Free => return None,
            Spec::Needs(a) => PrereqTree::Course(code_of(a)),
            Spec::Either(a, b) => PrereqTree::Any {
                any: vec![
                    PrereqTree::Course(code_of(a)),
                    PrereqTree::Course(code_of(b)),
                ],
            },
            Spec::CreditsBefore(credits) => PrereqTree::ProgramCredits {
                program_credits: crate::course::ProgramCredits {
                    program: None,
                    credits,
                },
            },
        };
        Some(Prerequisites::Parsed {
            raw: "generated".to_string(),
            tree,
        })
    }

    fn generated_inputs(generated: &Generated) -> Inputs {
        let mut inputs =
            Inputs::new(&generated.sessions, generated_courses(generated));
        inputs.credit_cap = generated.credit_cap;
        inputs.concomitant = generated.concomitant;
        inputs.passed = generated
            .passed
            .iter()
            .enumerate()
            .filter(|&(_, &passed)| passed)
            .map(|(i, _)| code_of(i))
            .collect();
        if let Some((index, session)) = generated.pin {
            if !generated.passed[index] {
                inputs.pinned = BTreeMap::from([(code_of(index), session)]);
            }
        }
        if let Some(index) = generated.stage {
            inputs.stages = BTreeSet::from([code_of(index)]);
        }
        if generated.summer && generated.open_summer {
            inputs.open_summers = BTreeSet::from([generated.sessions.len()]);
        }
        inputs
    }

    // Every assignment of the unpruned cartesian product, kept iff the
    // naive rules hold — evaluated straight off the specs, sharing nothing
    // with the production tree walk.
    fn brute_force(
        generated: &Generated,
        courses: &[Course],
        passed: &BTreeSet<String>,
        pinned: &BTreeMap<String, usize>,
    ) -> BTreeSet<BTreeMap<String, usize>> {
        let to_place: Vec<usize> = (0..generated.specs.len())
            .filter(|&i| !generated.passed[i])
            .collect();
        let assignments = to_place.iter().fold(
            vec![BTreeMap::<usize, usize>::new()],
            |partials, &i| {
                partials
                    .iter()
                    .flat_map(|partial| {
                        (1..=generated.sessions.len()).map(move |session| {
                            let mut next = partial.clone();
                            next.insert(i, session);
                            next
                        })
                    })
                    .collect()
            },
        );
        assignments
            .into_iter()
            .filter(|assignment| {
                naive_valid(assignment, generated, courses, passed, pinned)
            })
            .map(|assignment| {
                assignment
                    .iter()
                    .map(|(&i, &session)| (code_of(i), session))
                    .collect()
            })
            .collect()
    }

    fn naive_valid(
        assignment: &BTreeMap<usize, usize>,
        generated: &Generated,
        courses: &[Course],
        passed: &BTreeSet<String>,
        pinned: &BTreeMap<String, usize>,
    ) -> bool {
        let season_of = |session: usize| generated.sessions[session - 1];
        let offered = |i: usize, session: usize| match season_of(session) {
            Season::Fall => generated.offered[i].0,
            Season::Winter => generated.offered[i].1,
            Season::Summer => generated.offered[i].2,
        };
        // program credits only: C-0 reads as préuniversitaire and stays
        // out of the thresholds, passed or placed, like the engine
        let passed_credits: u32 = generated
            .passed
            .iter()
            .enumerate()
            .filter(|&(_, &passed)| passed)
            .filter(|&(i, _)| !is_preuniversity(&code_of(i)))
            .map(|(i, _)| generated.credits[i])
            .sum();
        let before_or_with = |q: usize, session: usize| {
            passed.contains(&code_of(q))
                || assignment.get(&q).is_some_and(|&t| {
                    t < session || (generated.concomitant && t == session)
                })
        };
        let offer_ok =
            assignment.iter().all(|(&i, &session)| offered(i, session));
        let pin_ok = assignment.iter().all(|(&i, &session)| {
            pinned.get(&code_of(i)).is_none_or(|&pin| pin == session)
        });
        let precedence_ok =
            assignment
                .iter()
                .all(|(&i, &session)| match generated.specs[i] {
                    Spec::Free => true,
                    Spec::Needs(a) => before_or_with(a, session),
                    Spec::Either(a, b) => {
                        before_or_with(a, session)
                            || before_or_with(b, session)
                    }
                    Spec::CreditsBefore(threshold) => {
                        let before: u32 = assignment
                            .iter()
                            .filter(|&(_, &t)| t < session)
                            .filter(|&(&j, _)| !is_preuniversity(&code_of(j)))
                            .map(|(&j, _)| generated.credits[j])
                            .sum();
                        passed_credits + before >= threshold
                    }
                });
        let capacity_ok = (1..=generated.sessions.len()).all(|session| {
            assignment
                .iter()
                .filter(|&(_, &t)| t == session)
                .map(|(&i, _)| generated.credits[i])
                .sum::<u32>()
                <= generated.credit_cap
        });
        // naive mirror of `summer_admits`: a pin lifts both summer rules
        let summer_ok = assignment.iter().all(|(&i, &session)| {
            if pinned.contains_key(&code_of(i)) {
                return true;
            }
            let summer = season_of(session) == Season::Summer;
            if generated.stage == Some(i) {
                summer
            } else {
                !summer
                    || (generated.open_summer
                        && session == generated.sessions.len())
            }
        });
        let weekly_ok = (1..=generated.sessions.len()).all(|session| {
            let domains: Vec<Vec<crate::weekly::Opt>> = assignment
                .iter()
                .filter(|&(_, &t)| t == session)
                .map(|(&i, _)| {
                    courses[i]
                        .seasons
                        .get(&season_of(session))
                        .map(crate::weekly::build_domain)
                        .unwrap_or_default()
                })
                .collect();
            crate::weekly::is_feasible(&domains)
        });
        offer_ok
            && pin_ok
            && precedence_ok
            && capacity_ok
            && summer_ok
            && weekly_ok
    }

    // The JSON names are the contract the WASM boundary hands to JS (ADR
    // `2026-08-module-wasm-quatre-fonctions-js`) — pinned here so a rename
    // breaks a test rather than a caller.
    #[test]
    fn a_placement_serializes_under_its_published_names() {
        let placement = Placement {
            completion: Completion::Complete,
            solutions: vec![Solution {
                placement: BTreeMap::from([("TST-1001".to_string(), 1)]),
                assumed: BTreeSet::from(["FRN-1904".to_string()]),
                left_out: BTreeSet::from(["TST-1002".to_string()]),
                credit_shortfalls: Vec::new(),
            }],
            blocked: vec![
                Blocked {
                    code: "A-1".to_string(),
                    reason: BlockedReason::EmptyDomain,
                },
                Blocked {
                    code: "B-2".to_string(),
                    reason: BlockedReason::UnsatisfiablePrerequisites,
                },
                Blocked {
                    code: "C-3".to_string(),
                    reason: BlockedReason::StageWithoutSummer,
                },
            ],
        };
        let json = serde_json::to_value(&placement)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        assert_eq!(
            json,
            serde_json::json!({
                "completion": "complete",
                "solutions": [{
                    "placement": {"TST-1001": 1},
                    "assumed": ["FRN-1904"],
                    "left_out": ["TST-1002"],
                }],
                "blocked": [
                    {"code": "A-1", "reason": "empty-domain"},
                    {"code": "B-2",
                     "reason": "unsatisfiable-prerequisites"},
                    {"code": "C-3", "reason": "stage-without-summer"},
                ],
            })
        );
        for (completion, name) in [
            (Completion::NodeBudget, "node-budget"),
            (Completion::SolutionCap, "solution-cap"),
        ] {
            let json = serde_json::to_value(completion)
                .unwrap_or_else(|e| panic!("serialize: {e}"));
            assert_eq!(json, serde_json::json!(name));
        }
    }

    proptest! {
        #[test]
        fn the_solution_set_matches_an_independent_brute_force(
            generated in arb_generated(),
        ) {
            let inputs = generated_inputs(&generated);
            let placement = inputs.place().unwrap_or_else(|e| panic!("{e}"));
            prop_assert_eq!(placement.completion, Completion::Complete);
            let got: BTreeSet<BTreeMap<String, usize>> = placement
                .solutions
                .iter()
                .map(|solution| solution.placement.clone())
                .collect();
            // no unknown code and no raw operand is ever generated, so
            // nothing may have been presumed
            for solution in &placement.solutions {
                prop_assert!(solution.assumed.is_empty());
            }
            let expected = brute_force(
                &generated,
                &inputs.courses,
                &inputs.passed,
                &inputs.pinned,
            );
            prop_assert_eq!(got, expected);
        }

        #[test]
        fn the_search_is_deterministic(generated in arb_generated()) {
            let inputs = generated_inputs(&generated);
            prop_assert_eq!(inputs.place(), inputs.place());
        }

        #[test]
        fn a_smaller_node_budget_returns_a_prefix_of_the_full_set(
            generated in arb_generated(),
            budget in 0u64..40,
        ) {
            let inputs = generated_inputs(&generated);
            let full = inputs.place().unwrap_or_else(|e| panic!("{e}"));
            let mut bounded = generated_inputs(&generated);
            bounded.max_nodes = budget;
            let partial =
                bounded.place().unwrap_or_else(|e| panic!("{e}"));
            prop_assert!(
                partial.solutions.len() <= full.solutions.len()
            );
            prop_assert_eq!(
                &partial.solutions[..],
                &full.solutions[..partial.solutions.len()]
            );
        }
    }
}
