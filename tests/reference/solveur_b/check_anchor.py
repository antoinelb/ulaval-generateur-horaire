"""Credibility anchor of the reference (ADR
`2026-07-fixture-attendue-derivee-avant-le-parseur`): before deriving any
expected output, the reference's weekly feasibility must reproduce the
`expected.valid` verdict of every frozen schedules/ fixture, 18/18.
Run: python check_anchor.py
"""

import sys

from common import FIXTURES, is_feasible, load_json

EXPECTED_FIXTURE_COUNT = 18


def main():
    paths = sorted((FIXTURES / "schedules").glob("*.json"))
    if len(paths) != EXPECTED_FIXTURE_COUNT:
        sys.exit(
            f"expected {EXPECTED_FIXTURE_COUNT} schedules fixtures, "
            f"found {len(paths)}"
        )
    failures = []
    for path in paths:
        fixture = load_json(path)
        got = verdict(fixture)
        want = fixture["expected"]["valid"]
        mark = "ok" if got == want else "FAIL"
        print(f"{mark:4} {path.stem}: got {got}, expected {want}")
        if got != want:
            failures.append(path.stem)
    if failures:
        sys.exit(f"anchor broken on: {', '.join(failures)}")
    print(f"anchor holds: {len(paths)}/{EXPECTED_FIXTURE_COUNT}")


def verdict(fixture):
    season = fixture["season"]
    chosen = fixture.get("chosen", {})
    option_lists = [
        pinned_options(course, season, chosen.get(course["code"], []))
        for course in fixture["courses"]
    ]
    return is_feasible(option_lists)


def pinned_options(course, season, nrcs):
    """weekly::force_nrc — keep options whose section set holds every
    pinned NRC (never « option k »: an NRC may sit in several options)."""
    options = course["seasons"].get(season, {}).get("options", [])
    return [
        option
        for option in options
        if all(nrc in {section["nrc"] for section in option} for nrc in nrcs)
    ]


if __name__ == "__main__":
    main()
