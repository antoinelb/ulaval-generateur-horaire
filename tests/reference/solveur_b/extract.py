"""Builds the objects embedded in solver-B fixtures, and verifies them.

A real course is the record of the newest 2026 snapshot containing it, its
`seasons` the union of each snapshot's own season subtree copied verbatim; a
program is its data/programmes file whole (ADR
`2026-07-familles-de-fixtures-organigrammes-et-regles`). Identity is checked
at the JSON-value level (fixtures re-indent).

Usage:
  python extract.py course CODE...   print merged courses as a JSON array
  python extract.py program SLUG     print a program
  python extract.py --verify         check every embedded object's provenance
"""

import sys

from common import FIXTURES, ROOT, dump_canonical, load_json

FIXTURE_DIRS = ("organigrammes", "rules")
# frozen fixture whose synthetic-but-frozen courses may be reused verbatim
FROZEN_COURSE_SOURCE = FIXTURES / "schedules" / "pairwise-infeasible.json"
SYNTHETIC_PREFIX = "TST-"
# GCI-1011 sits in the GEX organigramme and structure PDFs (3 cr, hiver) but
# in no snapshot — withdrawn from the current offering; usable as `passed`
# only, hence the empty `seasons`
HANDMADE = {
    "GCI-1011": {
        "code": "GCI-1011",
        "title": "SIG, territoire et infrastructures",
        "credits": 3,
        "cycle": 1,
        "prerequisites": None,
        "equivalents": [],
        "seasons": {},
    },
}


def main():
    args = sys.argv[1:]
    if args[:1] == ["--verify"]:
        verify()
    elif args[:1] == ["course"] and len(args) > 1:
        snapshots = load_snapshots()
        print(
            dump_canonical([merged_course(c, snapshots) for c in args[1:]]),
            end="",
        )
    elif args[:1] == ["program"] and len(args) == 2:
        print(dump_canonical(load_program(args[1])), end="")
    else:
        sys.exit((__doc__ or "").strip())


def verify():
    snapshots = load_snapshots()
    programs = {
        p["code"]: p
        for path in sorted((ROOT / "data" / "programmes").glob("*-2026.json"))
        for p in [load_json(path)]
    }
    frozen = {
        c["code"]: c for c in load_json(FROZEN_COURSE_SOURCE)["courses"]
    }
    problems = []
    paths = [
        path
        for family in FIXTURE_DIRS
        for path in sorted((FIXTURES / family).glob("*.json"))
    ]
    for path in paths:
        fixture = load_json(path)
        for course in fixture.get("courses", []):
            problems.extend(course_problems(course, snapshots, frozen, path))
        if "program" in fixture:
            embedded = fixture["program"]
            if embedded != programs.get(embedded.get("code")):
                problems.append(
                    f"{path.name}: program {embedded.get('code')} differs "
                    "from its data/programmes source"
                )
    if problems:
        sys.exit("\n".join(problems))
    print(f"all embedded objects verified across {len(paths)} fixtures")


def course_problems(course, snapshots, frozen, path):
    code = course["code"]
    if code.startswith(SYNTHETIC_PREFIX):
        if code in snapshots:
            return [f"{path.name}: {code} is synthetic yet exists in data"]
        return []
    if code in HANDMADE:
        if course != HANDMADE[code]:
            return [f"{path.name}: {code} differs from its handmade form"]
        return []
    candidates = []
    if code in snapshots:
        candidates.append(merged_course(code, snapshots))
    if code in frozen:
        candidates.append(frozen[code])
    if not candidates:
        return [f"{path.name}: {code} has no known provenance"]
    if all(course != candidate for candidate in candidates):
        return [f"{path.name}: {code} differs from every known source"]
    return []


def merged_course(code, snapshots):
    per_season = snapshots.get(code)
    if not per_season:
        raise ValueError(f"{code} is in no 2026 snapshot")
    base = dict(next(iter(per_season.values())))
    base["seasons"] = {
        season: record["seasons"][season]
        for season in ("fall", "winter", "summer")
        for record in [per_season.get(season)]
        if record is not None and season in record["seasons"]
    }
    return base


def load_snapshots():
    """code -> {season -> record}.

    data/cours.json holds each course whole (ADR
    `2026-07-snapshot-unique-des-cours-millesime-par-saison`), so every
    offered season maps to the same record and `merged_course` rebuilds the
    course identically — the interface the per-session files used to feed.
    """
    snapshots = {}
    for record in load_json(ROOT / "data" / "cours.json")["courses"]:
        for season in record["seasons"]:
            snapshots.setdefault(record["code"], {})[season] = record
    return snapshots


def load_program(slug):
    return load_json(ROOT / "data" / "programmes" / f"{slug}-2026.json")


if __name__ == "__main__":
    main()
