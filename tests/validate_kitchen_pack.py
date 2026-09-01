#!/usr/bin/env python3
"""Conformance proof for the data-only taste-led kitchen pack."""

from fractions import Fraction
from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACK_ROOT = ROOT / "packs" / "stewardship" / "kitchen"


def load(name):
    with (PACK_ROOT / name).open("rb") as source:
        return tomllib.load(source)


def quantity(text):
    amount, unit = text.split(maxsplit=1)
    return Fraction(amount), unit


def project(case):
    needed, need_unit = quantity(case["need"])
    available, available_unit = quantity(case["available"])
    if case["claim_truth"] in {"unknown", "stale"} or not case["claim_fresh"]:
        return 1, 1
    assert case["claim_truth"] == "confirmed"
    assert need_unit == available_unit, "unit labels cannot be converted implicitly"
    return (int(needed > available), 0)


def main():
    shapes = load("shapes.toml")
    expected_shapes = {
        "meal-intention", "occasion", "people", "taste", "effort", "time",
        "pantry-observation", "exact-quantity", "preparation-option", "question",
        "shopping-draft",
    }
    assert {shape["id"] for shape in shapes["shape"]} == expected_shapes
    exact = next(shape for shape in shapes["shape"] if shape["id"] == "exact-quantity")
    assert exact["unit_semantics"] == "uninterpreted-label"

    pack = load("pack.toml")
    assert pack["quantity"]["conversion"] == "explicit-reviewed-rational-rule-only"
    assert pack["availability"] == {
        "reduction_claim": "fresh-confirmed", "unknown": "question",
        "stale": "question", "absent": "explicit-claim-only",
    }
    assert set(pack["copied_claims"]["accepted_kinds"]) == {
        "confirmed-harvest-availability", "confirmed-egg-availability",
    }
    assert set(pack["copied_claims"]["forbidden_fields"]) == {
        "garden-handle", "flock-handle",
    }
    assert pack["cost"]["exposes"] == ["content-key"]
    assert "finance-lane" in pack["cost"]["forbidden"]
    impossible = {"choose-meal", "purchase", "checkout", "payment"}
    assert impossible <= set(pack["forbidden_effects"])
    assert impossible.isdisjoint(pack["effects"])
    assert set(pack["absent_lanes"]) == {"garden", "flock", "finance", "model", "network"}

    cases = load("cases.toml")["case"]
    for case in cases:
        lines, questions = project(case)
        assert lines == case["expected_lines"], case["id"]
        assert questions == case["expected_questions"], case["id"]
    stale = next(case for case in cases if case["id"] == "stale-flour-asks")
    fresh = next(case for case in cases if case["id"] == "fresh-eggs-reduce")
    assert project(stale)[1] > project(fresh)[1]

    expression = (PACK_ROOT / "shopping-draft.lisp").read_text()
    for forbidden in ("purchase", "checkout", "payment", "choose-meal"):
        assert f"({forbidden}" not in expression
    print("taste-led kitchen pack validated: stale stock asks; optional lanes absent")


if __name__ == "__main__":
    main()

