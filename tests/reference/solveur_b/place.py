"""Derives the expected of every organigrammes/ placement fixture.

Exhaustive enumeration by frontier folding (no `while`, no recursion —
the shape of the future Rust `fold`): every course-to-session assignment
whose seasons match, whose per-session credits fit the cap, then a final
check of prerequisites (three-valued, undecidable = hard error), credit
thresholds and the weekly A-veto. `complete` is always true: the search is
exhaustive, so an empty set is infeasibility proven (ADR
`2026-07-schema-des-fixtures-de-placement`).

Usage: python place.py fill|check [fixture-stems...]
"""

import sys

from common import (
    FIXTURES,
    dump_canonical,
    eval_prereq,
    is_feasible,
    load_json,
    resolve_credits,
)

DIR = FIXTURES / "organigrammes"


def main():
    mode, paths = parse_args(sys.argv[1:])
    failures = []
    for path in paths:
        fixture = load_json(path)
        fixture["expected"] = solve(fixture)
        content = dump_canonical(fixture)
        if mode == "fill":
            with open(path, "w", encoding="utf-8") as f:
                f.write(content)
            count = len(fixture["expected"]["solutions"])
            print(f"{path.stem}: {count} solution(s)")
        else:
            with open(path, encoding="utf-8") as f:
                on_disk = f.read()
            mark = "ok" if on_disk == content else "FAIL"
            print(f"{mark:4} {path.stem}")
            if on_disk != content:
                failures.append(path.stem)
    if failures:
        sys.exit(f"not bit-for-bit: {', '.join(failures)}")


def parse_args(args):
    if not args or args[0] not in ("fill", "check"):
        sys.exit((__doc__ or "").strip())
    stems = args[1:]
    paths = [DIR / f"{stem}.json" for stem in stems] or sorted(
        DIR.glob("*.json")
    )
    if not paths:
        sys.exit(f"no fixtures under {DIR}")
    return args[0], paths


def solve(fixture):
    sessions = fixture["sessions"]
    cap = fixture["credit_cap"]
    concomitant = fixture.get("concomitant", False)
    by_code = {course["code"]: course for course in fixture["courses"]}
    passed = set(fixture.get("passed", []))
    pinned = fixture.get("pinned", {})
    validate(fixture, sessions, by_code, passed, pinned)
    credits = {code: resolve_credits(c) for code, c in by_code.items()}
    passed_credits = sum(credits[code] for code in passed)
    to_place = [c["code"] for c in fixture["courses"] if c["code"] not in passed]
    domains = {
        code: [
            s
            for s in range(1, len(sessions) + 1)
            if sessions[s - 1] in by_code[code]["seasons"]
            and (code not in pinned or pinned[code] == s)
        ]
        for code in to_place
    }
    frontier = [{}]
    for code in to_place:
        frontier = [
            {**partial, code: s}
            for partial in frontier
            for s in domains[code]
            if session_load(partial, s, credits) + credits[code] <= cap
        ]
    valid = [
        assignment
        for assignment in frontier
        if assignment_valid(
            assignment,
            sessions,
            by_code,
            credits,
            passed,
            passed_credits,
            concomitant,
        )
    ]
    ordered = sorted(tuple(sorted(a.items())) for a in valid)
    return {"complete": True, "solutions": [dict(sol) for sol in ordered]}


def validate(fixture, sessions, by_code, passed, pinned):
    if not sessions or not by_code:
        raise ValueError("a fixture needs sessions and courses")
    if len(by_code) != len(fixture["courses"]):
        raise ValueError("duplicate course codes")
    stray = (passed | set(pinned)) - set(by_code)
    if stray:
        raise ValueError(f"passed/pinned codes without a Course: {stray}")
    if passed & set(pinned):
        raise ValueError("a passed course cannot also be pinned")
    out_of_range = {
        code: s for code, s in pinned.items() if not 1 <= s <= len(sessions)
    }
    if out_of_range:
        raise ValueError(f"pinned outside 1..{len(sessions)}: {out_of_range}")


def session_load(assignment, session, credits):
    return sum(
        credits[code] for code, s in assignment.items() if s == session
    )


def assignment_valid(
    assignment, sessions, by_code, credits, passed, passed_credits, concomitant
):
    known = set(by_code)
    for code, s in assignment.items():
        prerequisites = by_code[code].get("prerequisites")
        if not prerequisites:
            continue
        tree = prerequisites.get("tree")
        if tree is None:
            raise ValueError(
                f"{code}: raw-only prerequisites are undecidable — "
                "fixtures avoid the case"
            )
        before = passed | {
            other
            for other, t in assignment.items()
            if t < s or (concomitant and t == s and other != code)
        }
        credits_before = passed_credits + sum(
            credits[other] for other, t in assignment.items() if t < s
        )
        if not eval_prereq(tree, before, known, credits_before):
            return False
    for s in range(1, len(sessions) + 1):
        season = sessions[s - 1]
        option_lists = [
            by_code[code]["seasons"][season]["options"]
            for code, t in assignment.items()
            if t == s
        ]
        if not is_feasible(option_lists):
            return False
    return True


if __name__ == "__main__":
    main()
