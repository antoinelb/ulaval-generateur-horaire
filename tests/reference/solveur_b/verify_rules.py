"""Derives the expected of every rules/ coverage fixture.

Per scope (program, plus the chosen concentration/profile): mandatory
courses split satisfied/missing, and each rule reported as satisfied /
incomplete / reported per ADR
`2026-07-schema-du-rapport-de-couverture-en-fixtures`. Rule lists have set
semantics; references resolve to their target list; a credits sum or course
count above max is that rule's own verdict, `over_max` — a violation the
student can undo, never a refusal of the whole report (ADR
`2026-08-depassement-de-regle-en-statut-rouge`, which arbitrated ADR
`2026-07-somme-au-dessus-du-max-en-erreur-typee`). A rule whose own data
defeats the count reports `uncounted` and names its `defect`.
Within a scope, a selected course is claimed by the first evaluated rule
that lists it; later rules of the same scope report it as `elsewhere`
instead of counting it again (ADR
`2026-08-un-cours-compte-dans-une-seule-regle-par-portee`).
The « Stages » rule of the génie bacs lists its graduation stage first and
its optional companions after: the minimum alone would let an optional stage
satisfy the diploma requirement, so the first sigle is required on its own
(ADR `2026-08-stage-obligatoire-compte-dans-le-rapport-de-couverture`).

Usage: python verify_rules.py fill|check [fixture-stems...]
"""

import sys

from common import FIXTURES, dump_canonical, load_json, resolve_credits

DIR = FIXTURES / "rules"
SCOPES = (("concentration", "concentrations"), ("profile", "profiles"))
STAGES_RULE_TITLE = "Stages"


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
            entry
            for scope, block in scopes
            for entry in scope_reports(
                scope, block, program, selection, credits
            )
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


def scope_reports(scope, block, program, selection, credits):
    # a scope's rules, in order, each attributed against what earlier rules
    # of *this same scope* already claimed — the set starts empty per scope
    # so a course counts once in the concentration and once in the profile
    # (decision d'Antoine 2026-08-23)
    claimed = set()
    entries = []
    for rule in block["rules"]:
        entry = rule_report(scope, rule, program, selection, credits, claimed)
        if rule.get("constraint") is not None:
            claimed.update(entry.get("counted", []))
        entries.append(entry)
    return entries


def rule_report(scope, rule, program, selection, credits, claimed):
    courses = rule.get("courses")
    constraint = rule.get("constraint")
    if isinstance(courses, dict):
        try:
            courses = resolve_reference(courses, program)
        except BrokenReference as broken:
            # this rule alone loses its verdict; every other one stands
            entry = {
                "scope": scope,
                "title": rule["title"],
                "status": "uncounted",
            }
            if "raw" in rule:
                entry["raw"] = rule["raw"]
            entry["defect"] = broken.defect
            return entry
    if not isinstance(courses, list):
        # Keyword (any/negotiated) or raw-only: surfaced to the student,
        # never invented
        entry = {"scope": scope, "title": rule["title"], "status": "reported"}
        if "raw" in rule:
            entry["raw"] = rule["raw"]
        return entry
    listed = set(courses)
    if constraint is None:
        # a list naming no number (« Scolarité préparatoire ») : no verdict,
        # but the split is still shown (ADR
        # `2026-08-regle-sans-contrainte-comptee-mais-reportee`) — it neither
        # claims a code nor reports `elsewhere`, so counting stays global
        counted = sorted(listed & selection)
        entry = {
            "scope": scope,
            "title": rule["title"],
            "status": "reported",
            "counted": counted,
            "candidates": sorted(listed - selection),
        }
        if "raw" in rule:
            entry["raw"] = rule["raw"]
        return entry
    # a code an earlier rule of this scope already claimed no longer counts
    # here — shown as `elsewhere` instead so the student sees it, but the
    # verdict is computed on the reduced set (that is the whole point, ADR
    # `2026-08-un-cours-compte-dans-une-seule-regle-par-portee`)
    counted = sorted((listed & selection) - claimed)
    elsewhere = sorted(listed & selection & claimed)
    status, missing, defect = evaluate(constraint, counted, credits, rule)
    entry = {
        "scope": scope,
        "title": rule["title"],
        "status": status,
        "counted": counted,
    }
    if elsewhere:
        entry["elsewhere"] = elsewhere
    if missing is not None:
        entry["missing"] = missing
    entry["candidates"] = sorted(listed - selection)
    # a credits rule whose codes carry no Course keeps its rows — the
    # student still sees the list — but says why no sum was possible
    if defect is not None:
        entry["defect"] = defect
    return entry


class BrokenReference(Exception):
    """The chase failed; the rule alone is uncounted, the report stands."""

    def __init__(self, reference):
        super().__init__(reference["rule"])
        self.defect = {
            "broken_reference": {
                "concentration": reference["concentration"],
                "target": reference["rule"],
            }
        }


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
        raise BrokenReference(reference)
    target = next(
        (
            r
            for r in concentration["rules"]
            if r["title"] == reference["rule"]
        ),
        None,
    )
    if target is None:
        raise BrokenReference(reference)
    resolved = target.get("courses")
    if not isinstance(resolved, list):
        raise BrokenReference(reference)
    return resolved


def evaluate(constraint, counted, credits, rule):
    """Returns (status, missing, defect) — and never raises."""
    if constraint["type"] == "course":
        total = len(counted)
        status, missing = over_or(
            total, constraint, {"count": constraint["min"] - total}
        )
        # only a course rule can be a stage rule, so the credits branch
        # below never needs the check
        status, missing = required_stage_held(rule, counted, status, missing)
        return status, missing, None
    total = 0
    for code in counted:
        if code not in credits:
            return "uncounted", None, {"missing_course": {"code": code}}
        total += credits[code]
    status, missing = over_or(
        total, constraint, {"credits": constraint["min"] - total}
    )
    return status, missing, None


def required_stage_held(rule, counted, status, missing):
    """Downgrades a stage rule satisfied without its graduation stage."""
    required = mandatory_stage(rule)
    if required is None or required in counted or status != "satisfied":
        return status, missing
    return "incomplete", {"count": 1}


def mandatory_stage(rule):
    """The graduation stage the « Stages » rule lists first, if any."""
    if rule["title"] != STAGES_RULE_TITLE:
        return None
    constraint = rule.get("constraint")
    if constraint is None or constraint["type"] != "course":
        return None
    if constraint["min"] <= 0:
        return None
    courses = rule.get("courses")
    if not isinstance(courses, list) or not courses:
        return None
    return courses[0]


def over_or(total, constraint, shortfall):
    if total > constraint["max"]:
        return "over_max", None
    if total >= constraint["min"]:
        return "satisfied", None
    return "incomplete", shortfall


def language_report(requirement, selection):
    branches = [requirement["francophone"]["course"]]
    if requirement.get("non_francophone"):
        branches.append(requirement["non_francophone"]["course"])
    satisfied = any(course in selection for course in branches)
    return {"status": "satisfied" if satisfied else "reported"}


if __name__ == "__main__":
    main()
