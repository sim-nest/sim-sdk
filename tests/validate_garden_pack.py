#!/usr/bin/env python3
"""Offline conformance proof for the cited seasonal garden pack."""

from datetime import datetime
from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACK_ROOT = ROOT / "packs" / "stewardship" / "garden"


def load(name):
    with (PACK_ROOT / name).open("rb") as source:
        return tomllib.load(source)


def instant(value):
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def consider(case):
    expired = instant(case["as_of"]) > instant(case["guidance_expires_at"])
    weather_missing = case["forecast"] != "present-fresh"
    if expired or weather_missing:
        return 0, 0, 1
    return 1, 0, 0


def copied_claims(case):
    return int(case["truth"] == "confirmed" and case["fresh"])


def main():
    shapes = load("shapes.toml")["shape"]
    by_id = {shape["id"]: shape for shape in shapes}
    independent = {
        "bed", "crop", "sowing", "planting", "care-action", "observation",
        "weather-reference", "harvest", "season", "next-question",
    }
    assert independent <= by_id.keys()
    assert len(by_id) == len(shapes), "shape ids must be independent"

    predicates = {shape["predicate"] for shape in shapes if "predicate" in shape}
    assert predicates == {"plan", "observation", "forecast", "recommendation"}

    pack = load("pack.toml")
    assert pack["predicates"]["distinct"] == [
        "plan", "observation", "forecast", "recommendation", "completed-work",
    ]
    assert pack["predicates"]["no_promotion"] == (
        "predicate-kind-never-implies-another-kind"
    )
    assert set(pack["guidance"]["required_fields"]) == {
        "source", "jurisdiction", "checked-at", "expires-at",
    }
    assert pack["guidance"]["expired"] == "next-question"
    assert pack["calendar"]["authority"] == "none"
    assert pack["calendar"]["input"] == "chosen-action"
    assert pack["calendar"]["missing"] == (
        "retain-chosen-action-without-projection"
    )
    assert pack["kitchen_claim"]["kind"] == "confirmed-harvest-availability"
    assert pack["kitchen_claim"]["required_truth"] == "confirmed"
    assert pack["kitchen_claim"]["copied"] is True
    assert set(pack["absent_lanes"]) == {"kitchen", "calendar", "model", "network"}

    cases = load("cases.toml")
    for case in cases["case"]:
        result = consider(case)
        assert result == (
            case["expected_recommendations"], case["expected_instructions"],
            case["expected_questions"],
        ), case["id"]
    for case in cases["harvest_case"]:
        assert copied_claims(case) == case["expected_kitchen_claims"], case["id"]

    base, extended = cases["season_fixture"]
    assert extended["beds"][: len(base["beds"])] == base["beds"]
    assert extended["crops"][: len(base["crops"])] == base["crops"]
    assert "herb-bed" in extended["beds"] and "dill" in extended["crops"]
    assert pack["extensibility"] == {
        "season_rebuild": "ordered-input-data", "bed_crop_addition": "data-only",
    }

    expression = (PACK_ROOT / "season-memory.lisp").read_text()
    for forbidden in ("(predict-weather", "(command-work", "(choose-action", "(calendar/mutate"):
        assert forbidden not in expression
    print("cited seasonal garden validated: stale or missing evidence asks; optional lanes absent")


if __name__ == "__main__":
    main()
