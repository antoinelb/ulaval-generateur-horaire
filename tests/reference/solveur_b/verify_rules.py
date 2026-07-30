"""Derives the expected of every rules/ coverage fixture.

Per scope (program, plus the chosen concentration/profile): mandatory
courses split satisfied/missing, and each rule reported as satisfied /
incomplete / reported per ADR
`2026-07-schema-du-rapport-de-couverture-en-fixtures`. Rule lists have set
semantics; references resolve to their target list; a credits sum above max
is a hard error (semantics undecided, fixtures stay within).

Usage: python verify_rules.py fill|check [fixture-stems...]
"""

import sys

from common import FIXTURES, dump_canonical, load_json, resolve_credits

DIR = FIXTURES / "rules"
SCOPES = (("concentration", "concentrations"), ("profile", "profiles"))


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
            print(f"{path.stem}: {len(fixture['expected']['rules'])} rule(s)")
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
    program = fixture["program"]
    selection = set(fixture["selection"])
    credits = {
        course["code"]: resolve_credits(course)
        for course in fixture.get("courses", [])
    }
    scopes = [("program", program)]
    for scope, key in SCOPES:
        title = fixture.get(scope)
        if title is not None:
            block = next(
                (b for b in program[key] if b["title"] == title), None
            )
            if block is None:
                raise ValueError(f"{scope} « {title} » not in the program")
            scopes.append((scope, block))
    expected: dict = {
        "mandatory": [
            mandatory_report(scope, block, selection)
            for scope, block in scopes
        ],
        "rules": [
            rule_report(scope, rule, program, selection, credits)
            for scope, block in scopes
            for rule in block["rules"]
        ],
    }
    if program.get("language_requirement"):
        expected["language_requirement"] = language_report(
            program["language_requirement"], selection
        )
    return expected


def mandatory_report(scope, block, selection):
    mandatory = set(block["mandatory"])
    return {
        "scope": scope,
        "satisfied": sorted(mandatory & selection),
        "missing": sorted(mandatory - selection),
    }


def rule_report(scope, rule, program, selection, credits):
    courses = rule.get("courses")
    constraint = rule.get("constraint")
    if isinstance(courses, dict):
        courses = resolve_reference(courses, program)
    if not isinstance(courses, list) or constraint is None:
        # Keyword (any/negotiated), raw-only, or a rule naming no number:
        # surfaced to the student, never invented
        entry = {"scope": scope, "title": rule["title"], "status": "reported"}
        if "raw" in rule:
            entry["raw"] = rule["raw"]
        return entry
    listed = set(courses)
    counted = sorted(listed & selection)
    status, missing = evaluate(constraint, counted, credits, rule)
    entry = {
        "scope": scope,
        "title": rule["title"],
        "status": status,
        "counted": counted,
    }
    if missing is not None:
        entry["missing"] = missing
    entry["candidates"] = sorted(listed - selection)
    return entry


def resolve_reference(reference, program):
    concentration = next(
        (
            c
            for c in program["concentrations"]
            if c["title"] == reference["concentration"]
        ),
        None,
    )
    if concentration is None:
        raise ValueError(f"reference to unknown {reference['concentration']}")
    target = next(
        (
            r
            for r in concentration["rules"]
            if r["title"] == reference["rule"]
        ),
        None,
    )
    if target is None:
        raise ValueError(f"reference to unknown rule {reference['rule']}")
    resolved = target.get("courses")
    if not isinstance(resolved, list):
        raise ValueError(
            f"{reference['rule']} of {reference['concentration']} is not a "
            "course list — a reference chase is an error"
        )
    return resolved


def evaluate(constraint, counted, credits, rule):
    if "count" in constraint:
        needed = constraint["count"]
        if len(counted) >= needed:
            return "satisfied", None
        return "incomplete", {"count": needed - len(counted)}
    total = 0
    for code in counted:
        if code not in credits:
            raise ValueError(
                f"{rule['title']}: {code} counts credits but has no Course "
                "object in the fixture"
            )
        total += credits[code]
    if total > constraint["max"]:
        raise ValueError(
            f"{rule['title']}: sum {total} exceeds max {constraint['max']} "
            "— semantics undecided, fixtures stay within"
        )
    if total >= constraint["min"]:
        return "satisfied", None
    return "incomplete", {"credits": constraint["min"] - total}


def language_report(requirement, selection):
    branches = [requirement["francophone"]["course"]]
    if requirement.get("non_francophone"):
        branches.append(requirement["non_francophone"]["course"])
    satisfied = any(course in selection for course in branches)
    return {"status": "satisfied" if satisfied else "reported"}


if __name__ == "__main__":
    main()
