use std::collections::{BTreeMap, BTreeSet};
use std::ops::ControlFlow;

use crate::course::{Course, PrereqTree, Prerequisites, Season};
use crate::feasibility::FeasibilityCache;

// far above any real prerequisite tree; bounds the flatten loop
const MAX_TREE_NODES: usize = 10_000;

// Solver B's input (ADR `2026-07-schema-des-fixtures-de-placement`): a
// *given* course list — B never picks courses — sessions as an ordered
// list of seasons, and the student's constraints as domain reductions.
// `seed` is a value-ordering hint only (the reference cheminement_type):
// it shapes the search order so the first solution resembles the reference
// path, never the solution set — codes it names that are absent from
// `courses` are ignored (a full-bac seed over a partial list is normal).
#[derive(Debug)]
pub struct PlacementRequest<'a> {
    pub sessions: &'a [Season],
    pub credit_cap: u32,
    pub concomitant: bool,
    pub courses: &'a [Course],
    pub passed: &'a BTreeSet<String>,
    pub pinned: &'a BTreeMap<String, usize>,
    pub seed: &'a BTreeMap<String, usize>,
    // the double bound (ADR `2026-07-budget-de-b-en-double-borne`): work
    // (expanded partial assignments) and memory (returned solutions)
    pub max_nodes: u64,
    pub max_solutions: usize,
}

// All feasible placements found, in search order, with the three outcomes
// never confused (ADR `2026-07-b-enumere-toutes-les-solutions`).
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub completion: Completion,
    pub solutions: Vec<Solution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    // search exhausted: the set is total — empty means infeasibility proven
    Complete,
    // work bound hit: the set is partial, never « infeasible »
    NodeBudget,
    // memory bound hit: the set is partial, never « infeasible »
    SolutionCap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solution {
    // code → 1-based session number; passed courses do not appear
    pub placement: BTreeMap<String, usize>,
    // unverifiable operands this placement's verdict relied on — a raw
    // operand or a course neither listed nor passed, presumed satisfied
    // and surfaced, never imposed (ADR
    // `2026-07-prealable-inconnu-non-bloquant-remonte`)
    pub assumed: BTreeSet<String>,
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
    #[error("the prerequisite tree of {code} exceeds {MAX_TREE_NODES} nodes")]
    PrerequisiteTreeTooLarge { code: String },
}

// a course to place: its planning credits, its prerequisite tree flattened
// once, and its value-ordered session domain
struct Candidate {
    code: String,
    credits: u32,
    tree: Option<FlatTree>,
    domain: Vec<usize>,
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
    by_code: BTreeMap<&'a str, &'a Course>,
    passed_credits: u32,
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
    let ctx = SearchCtx {
        request,
        candidates: &candidates,
        index_of: candidates
            .iter()
            .enumerate()
            .map(|(i, candidate)| (candidate.code.clone(), i))
            .collect(),
        referenced_by: referencing_map(&candidates),
        by_code: request
            .courses
            .iter()
            .map(|course| (course.code.as_str(), course))
            .collect(),
        passed_credits: request
            .courses
            .iter()
            .filter(|course| request.passed.contains(&course.code))
            .map(|course| course.credits.planning())
            .sum(),
    };
    Ok(search(&ctx))
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
    Ok(())
}

fn build_candidates(
    request: &PlacementRequest,
) -> Result<Vec<Candidate>, PlacementError> {
    request
        .courses
        .iter()
        .filter(|course| !request.passed.contains(&course.code))
        .map(|course| {
            Ok(Candidate {
                code: course.code.clone(),
                credits: course.credits.planning(),
                tree: flat_tree(course)?,
                domain: value_ordered_domain(course, request),
            })
        })
        .collect()
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
// the course never enters it.
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
        .collect();
    if let Some(&anchor) = request.seed.get(&course.code) {
        domain.sort_by_key(|&session| (session.abs_diff(anchor), session));
    }
    domain
}

// Depth-first over an explicit stack, inside a fold bounded by the node
// budget (ADR `2026-07-budget-de-b-en-double-borne`) : one iteration = one
// partial assignment expanded (or one complete one finalized). Depth-first
// keeps the found solutions when a bound stops the search, where A's
// frontier-by-course fold would lose them all.
fn search(ctx: &SearchCtx) -> Placement {
    let mut cache = FeasibilityCache::new();
    let mut stack: Vec<Vec<usize>> = vec![Vec::new()];
    let mut solutions: Vec<Solution> = Vec::new();
    let flow = (0..ctx.request.max_nodes).try_fold((), |(), _| {
        match stack.pop() {
            // the stack ran dry: the search is exhausted, the set proven
            // total — exhaustion wins over a simultaneously full cap
            None => ControlFlow::Break(Completion::Complete),
            Some(chosen) => {
                if chosen.len() == ctx.candidates.len() {
                    if let Some(solution) = finalize(&chosen, ctx) {
                        solutions.push(solution);
                    }
                } else {
                    expand(&chosen, ctx, &mut cache, &mut stack);
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
    stack: &mut Vec<Vec<usize>>,
) {
    let depth = chosen.len();
    let candidate = &ctx.candidates[depth];
    let loads = session_loads(chosen, ctx);
    let children: Vec<Vec<usize>> = candidate
        .domain
        .iter()
        .filter(|&&session| {
            loads[session - 1] + candidate.credits <= ctx.request.credit_cap
        })
        .map(|&session| {
            let mut child = chosen.to_vec();
            child.push(session);
            child
        })
        .filter(|child| precedence_admits(child, depth, ctx))
        .filter(|child| weekly_admits(child, depth, ctx, cache))
        .collect();
    stack.extend(children.into_iter().rev());
}

fn session_loads(chosen: &[usize], ctx: &SearchCtx) -> Vec<u32> {
    chosen.iter().enumerate().fold(
        vec![0u32; ctx.request.sessions.len()],
        |mut loads, (i, &session)| {
            loads[session - 1] += ctx.candidates[i].credits;
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
    let rechecked = ctx
        .referenced_by
        .get(&ctx.candidates[depth].code)
        .into_iter()
        .flatten()
        .filter(|&&i| i < depth)
        .chain(std::iter::once(&depth));
    rechecked.into_iter().all(|&evaluated| {
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

// the A-veto, memoized on (season, set of codes) — checked on the partial
// session content at every addition: `is_feasible` is monotone (adding a
// course never repairs a conflict), so an early veto is already final
fn weekly_admits(
    child: &[usize],
    depth: usize,
    ctx: &SearchCtx,
    cache: &mut FeasibilityCache,
) -> bool {
    let session = child[depth];
    let codes: BTreeSet<String> = child
        .iter()
        .enumerate()
        .filter(|&(_, &placed)| placed == session)
        .map(|(i, _)| ctx.candidates[i].code.clone())
        .collect();
    cache.term_feasible(
        ctx.request.sessions[session - 1],
        &codes,
        &ctx.by_code,
    )
}

// A complete assignment survived every incremental filter; the final pass
// resolves the verdicts that only a complete assignment can decide
// (`program_credits` thresholds) and collects the operands the placement
// had to presume.
fn finalize(chosen: &[usize], ctx: &SearchCtx) -> Option<Solution> {
    let assumed = ctx.candidates.iter().enumerate().try_fold(
        BTreeSet::new(),
        |mut assumed, (evaluated, candidate)| match &candidate.tree {
            None => Some(assumed),
            Some(tree) => {
                let eval_ctx = EvalCtx {
                    ctx,
                    chosen,
                    evaluated,
                    complete: true,
                };
                match eval(tree, &eval_ctx) {
                    Verdict::Sat(operands) => {
                        assumed.extend(operands);
                        Some(assumed)
                    }
                    // False: a credits threshold resolved short at the
                    // leaf. Unknown cannot survive a complete assignment;
                    // rejecting is the safe reading if it ever did.
                    Verdict::False | Verdict::Unknown => None,
                }
            }
        },
    )?;
    Some(Solution {
        placement: chosen
            .iter()
            .enumerate()
            .map(|(i, &session)| (ctx.candidates[i].code.clone(), session))
            .collect(),
        assumed,
    })
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

// préuniversitaire numbers start with 0 (`MAT-0130`); for a code absent
// from the snapshot the number is the only signal we have of its cycle
fn is_preuniversity(code: &str) -> bool {
    code.split_once('-')
        .is_some_and(|(_, number)| number.starts_with('0'))
}

// accumulated credits strictly before the evaluated course's session:
// passed credits count in full (ADR fixture family), placements at the
// same session never do, concomitant or not
fn credits_leaf(threshold: u32, eval_ctx: &EvalCtx) -> Verdict {
    let session = eval_ctx.chosen[eval_ctx.evaluated];
    let before: u32 = eval_ctx.ctx.passed_credits
        + eval_ctx
            .chosen
            .iter()
            .enumerate()
            .filter(|&(_, &placed)| placed < session)
            .map(|(i, _)| eval_ctx.ctx.candidates[i].credits)
            .sum::<u32>();
    if before >= threshold {
        Verdict::Sat(BTreeSet::new())
    } else if eval_ctx.complete {
        Verdict::False
    } else {
        // an unplaced course may still land before and lift the total
        Verdict::Unknown
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

    const FALL_WINTER: [Season; 2] = [Season::Fall, Season::Winter];

    struct Inputs {
        sessions: Vec<Season>,
        courses: Vec<Course>,
        passed: BTreeSet<String>,
        pinned: BTreeMap<String, usize>,
        seed: BTreeMap<String, usize>,
        credit_cap: u32,
        concomitant: bool,
        max_nodes: u64,
        max_solutions: usize,
    }

    impl Inputs {
        fn new(sessions: &[Season], courses: Vec<Course>) -> Self {
            Inputs {
                sessions: sessions.to_vec(),
                courses,
                passed: BTreeSet::new(),
                pinned: BTreeMap::new(),
                seed: BTreeMap::new(),
                credit_cap: 30,
                concomitant: false,
                max_nodes: 100_000,
                max_solutions: 10_000,
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
                seed: &self.seed,
                max_nodes: self.max_nodes,
                max_solutions: self.max_solutions,
            })
        }

        fn solve(&self) -> Placement {
            self.place().unwrap_or_else(|e| panic!("{e}"))
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
            PlacementError::PrerequisiteTreeTooLarge { code: code() },
        ] {
            assert!(error.to_string().contains("A-1"), "{error}");
        }
        assert!(PlacementError::EmptyRequest.to_string().contains("needs"));
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
        sessions: Vec<Season>,
        specs: Vec<Spec>,
        credits: Vec<u32>,
        offered: Vec<(bool, bool)>,
        days: Vec<usize>,
        passed: Vec<bool>,
        pin: Option<(usize, usize)>,
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
                    (1u32..=3, 0u8..4, 0usize..3, proptest::bool::ANY),
                    n_courses..=n_courses,
                ),
                proptest::option::of((0usize..n_courses, 1usize..=n_sessions)),
                3u32..=9,
                proptest::bool::ANY,
            )
                .prop_map(
                    move |(
                        winters,
                        prereqs,
                        shapes,
                        pin,
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
                        Generated {
                            sessions: winters
                                .iter()
                                .map(|&winter| {
                                    if winter {
                                        Season::Winter
                                    } else {
                                        Season::Fall
                                    }
                                })
                                .collect(),
                            specs,
                            credits: shapes
                                .iter()
                                .map(|&(credits, ..)| credits)
                                .collect(),
                            offered: shapes
                                .iter()
                                .map(|&(_, mask, ..)| {
                                    // 0 = nowhere: the infeasible case
                                    (mask & 1 != 0, mask & 2 != 0)
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
            Season::Summer => false,
        };
        let passed_credits: u32 = generated
            .passed
            .iter()
            .enumerate()
            .filter(|&(_, &passed)| passed)
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
        offer_ok && pin_ok && precedence_ok && capacity_ok && weekly_ok
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
