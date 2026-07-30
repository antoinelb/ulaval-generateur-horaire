"""Shared primitives of the solver-B reference implementation.

Independent second reading of the frozen data (ADR
`2026-07-fixture-attendue-derivee-avant-le-parseur`): weekly feasibility by
brute force over exact minute intervals (equivalent to week.rs's 5-minute
buckets because every real boundary is a multiple of 5 — proven by the
18/18 anchor of check_anchor.py), three-valued prerequisite evaluation, and
a byte-stable canonical JSON writer. Stdlib only; no `while`, no recursion
(explicit bounded stacks, matching the Rust discipline).
"""

import itertools
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / "tests" / "fixtures" / "test_cases"
# 2026 snapshots newest first (fall > summer > winter within the civil year);
# each contributes only its own season's subtree to a merged course
SNAPSHOTS = (("a2026", "fall"), ("e2026", "summer"), ("h2026", "winter"))
DAY_INDEX = {
    "monday": 0,
    "tuesday": 1,
    "wednesday": 2,
    "thursday": 3,
    "friday": 4,
    "saturday": 5,
    "sunday": 6,
}
MINUTES_PER_DAY = 24 * 60
# far above any real prerequisite tree; bounds the traversal loops
MAX_TREE_NODES = 10_000


def load_json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def dump_canonical(obj):
    """The byte-stable form every fixture is written in."""
    return json.dumps(obj, indent=2, ensure_ascii=False) + "\n"


def write_canonical(path, obj):
    with open(path, "w", encoding="utf-8") as f:
        f.write(dump_canonical(obj))


def resolve_credits(course):
    credits = course["credits"]
    if not isinstance(credits, int):
        raise ValueError(
            f"{course['code']}: variable credits {credits} need a chosen "
            "weighting — an open question, fixtures avoid the case"
        )
    return credits


def is_feasible(option_lists):
    """Whether one option per course can be taken without any slot overlap.

    `option_lists` holds, per course, its season's options (each option a
    list of sections, taken whole). Mirrors weekly::is_feasible.
    """
    if any(not options for options in option_lists):
        return False
    interval_lists = [
        [option_intervals(option) for option in options]
        for options in option_lists
    ]
    for combo in itertools.product(*interval_lists):
        clash = any(
            overlaps(a, b)
            for i, j in itertools.combinations(range(len(combo)), 2)
            for a in combo[i]
            for b in combo[j]
        )
        if not clash:
            return True
    return False


def option_intervals(option):
    """Absolute minute intervals of every slot of an option's sections.

    A section without slots (remote) contributes none and thus never
    conflicts — the `remote-never-conflicts` phenomenon of schedules/.
    """
    return [
        (
            DAY_INDEX[slot["day"]] * MINUTES_PER_DAY + minutes(slot["start"]),
            DAY_INDEX[slot["day"]] * MINUTES_PER_DAY + minutes(slot["end"]),
        )
        for section in option
        for slot in section["slots"]
    ]


def minutes(hh_mm):
    hours, mins = hh_mm.split(":")
    return int(hours) * 60 + int(mins)


def overlaps(a, b):
    # end-exclusive: back-to-back slots do not conflict (schedules/ anchor)
    return a[0] < b[1] and b[0] < a[1]


def eval_prereq(tree, before, known, credits_before):
    """True/False verdict of a PrereqTree; three-valued inside.

    Leaves: a course in `before` (placed strictly before, or passed, or
    same-session under the concomitant relaxation) is satisfied; a course in
    `known` but not before is not; anything else — a `raw` operand or a code
    absent from the fixture — is undecidable. The tree combines with
    all/any/program_credits; if the FINAL verdict depends on an undecidable
    leaf the reference stops with an error instead of inventing one
    (undecidable branches that cannot change the outcome are fine: an `any`
    with one satisfied branch tolerates an unknown sibling).
    """
    order = flatten(tree)
    verdicts = {}
    for node in reversed(order):
        verdicts[id(node)] = node_verdict(
            node, verdicts, before, known, credits_before
        )
    verdict = verdicts[id(tree)]
    if isinstance(verdict, str):
        raise ValueError(f"prerequisite verdict is undecidable: {verdict}")
    return verdict


def flatten(tree):
    """Pre-order nodes via an explicit bounded stack (children before
    parents once reversed)."""
    stack = [tree]
    order = []
    for _ in range(MAX_TREE_NODES):
        if not stack:
            return order
        node = stack.pop()
        order.append(node)
        stack.extend(children(node))
    raise ValueError(f"prerequisite tree exceeds {MAX_TREE_NODES} nodes")


def children(node):
    if isinstance(node, dict):
        return node.get("all") or node.get("any") or []
    return []


def node_verdict(node, verdicts, before, known, credits_before):
    """True, False, or a string naming why the node is undecidable."""
    if isinstance(node, str):
        if node in before:
            return True
        if node in known:
            return False
        return f"course {node} is neither in the fixture nor passed"
    if "raw" in node:
        return f"raw operand « {node['raw']} »"
    if "program_credits" in node:
        return credits_before >= node["program_credits"]["credits"]
    if "all" in node:
        kids = [verdicts[id(child)] for child in node["all"]]
        if any(kid is False for kid in kids):
            return False
        return next((kid for kid in kids if isinstance(kid, str)), True)
    if "any" in node:
        kids = [verdicts[id(child)] for child in node["any"]]
        if any(kid is True for kid in kids):
            return True
        return next((kid for kid in kids if isinstance(kid, str)), False)
    raise ValueError(f"unrecognized prerequisite node: {node!r}")
