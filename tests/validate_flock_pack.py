#!/usr/bin/env python3
"""Offline conformance proof for the calm flock care pack."""

from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACK_ROOT = ROOT / "packs" / "stewardship" / "flock"


def load(name):
    with (PACK_ROOT / name).open("rb") as source:
        return tomllib.load(source)


def copied_claims(case):
    return int(case["truth"] == "confirmed" and case["fresh"])


def main():
    pack = load("pack.toml")
    shapes = load("shapes.toml")["shape"]
    by_id = {shape["id"]: shape for shape in shapes}
    assert {
        "flock", "bird-identity", "common-care-check", "observation",
        "supply", "deviation", "factual-handoff", "attention",
        "confirmed-egg-availability",
    } == by_id.keys()
    assert by_id["bird-identity"]["forbidden"] == [
        "productivity", "rank", "cull-status", "animal-value",
    ]
    assert by_id["factual-handoff"]["forbidden"] == [
        "diagnosis", "treatment", "prognosis",
    ]

    cases = load("cases.toml")
    birds = cases["bird"]
    assert len(birds) == 8
    assert len({bird["id"] for bird in birds}) == 8
    assert all(set(bird) == {"id", "name"} for bird in birds)

    capture, missing = cases["capture_case"]
    assert sum(capture["step_seconds"]) <= pack["daily_capture"]["budget_seconds"]
    assert capture["expected_forms"] == 1
    assert capture["observation_scope"] == "flock"
    assert missing["expected_state"] == pack["daily_capture"]["missing"] == "unknown"
    assert pack["daily_capture"]["never_expand_normal_to_per_bird"] is True

    deviation = cases["deviation_case"][0]
    assert deviation["bird"] in {bird["id"] for bird in birds}
    assert deviation["observed_facts"] and deviation["source_refs"]
    assert deviation["current_care"] and deviation["current_supplies"]
    assert deviation["expected_attention"] == "human-review-required"
    assert deviation["expected_diagnoses"] == deviation["expected_treatments"] == 0
    assert pack["deviation"]["authority"] == "human-or-professional"

    for case in cases["egg_claim_case"]:
        assert copied_claims(case) == case["expected_kitchen_claims"], case["id"]
    assert pack["kitchen_claim"]["maximum_per_capture"] == 1
    assert pack["kitchen_claim"]["copied"] is True

    forbidden = set(pack["authority"]["forbidden_operations"])
    assert {"productivity", "ranking", "culling", "animal-value", "automated-care"} <= forbidden
    expression = (PACK_ROOT / "daily-care.lisp").read_text()
    for operation in (
        "(score-bird", "(rank-bird", "(value-animal", "(recommend-culling",
        "(diagnose", "(select-treatment", "(automate-care",
    ):
        assert operation not in expression
    print("calm flock care validated: one 18-second capture; no bird scoring or care automation")


if __name__ == "__main__":
    main()

