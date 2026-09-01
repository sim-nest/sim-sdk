#!/usr/bin/env python3
"""Offline conformance proof for voluntary private observation."""

from fractions import Fraction
from pathlib import Path
import re
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACK_ROOT = ROOT / "packs" / "stewardship" / "private-observation"


def load(name):
    with (PACK_ROOT / name).open("rb") as source:
        return tomllib.load(source)


def parse_quantity(text):
    amount, unit = text.split(maxsplit=1)
    return Fraction(amount), unit


def totals(events):
    result = {}
    for event in events:
        amount, unit = parse_quantity(event)
        result[unit] = result.get(unit, Fraction()) + amount
    return result


def main():
    pack = load("pack.toml")
    by_id = {shape["id"]: shape for shape in load("shapes.toml")["shape"]}
    assert set(by_id) == {
        "observation", "declared-product", "ingredient", "dose-event",
        "meal-note", "missingness", "concurrent-change", "timeline",
        "professional-review-export", "exact-quantity",
    }
    assert pack["default"] == "absent"
    assert pack["activation"] == "explicit-pack-opt-in"
    assert pack["consent"]["granularity"] == "per-field"
    assert by_id["observation"]["constraint"] == "every-present-optional-field-listed-in-opted-fields"

    for case in load("cases.toml")["sum_case"]:
        actual = totals(case["events"])
        expected = totals(case["expected"])
        assert actual == expected, case["id"]
        if case["missing"]:
            assert pack["quantity"]["missing"] == "missing"
    concurrent = load("cases.toml")["concurrent_case"][0]
    assert concurrent["expected_visible"] == 2
    assert pack["quantity"]["concurrent_changes"] == "retain-both-until-human-review"

    forbidden = {
        "diagnosis", "cause", "treatment", "prescribe", "recommend",
        "adherence", "risk", "target", "restrictive-diet",
    }
    assert forbidden == set(pack["forbidden_effects"])
    expression = (PACK_ROOT / "timeline.lisp").read_text()
    called = set(re.findall(r"\(([a-z][a-z0-9/-]*)", expression))
    assert not forbidden.intersection(called)

    storage = pack["storage"]
    assert storage["lane"] == "private-observation"
    assert storage["payload_scope"] == "all-source-and-derived-payloads"
    for operation in ("lock", "revoke", "destroy"):
        assert "remove-plaintext-from-derived-views-and-pack-access" in storage[operation]
    assert storage["other_packs"] == "unaffected"

    export = pack["export"]
    assert export["selection"] == "explicit-fields-and-events"
    assert export["preview"] == export["confirmation"] == "required"
    assert export["default_encoding"] == "locally-encrypted"
    assert export["plaintext"] == "requires-mia-acknowledged-warning"
    assert export["plaintext_warning"]
    assert pack["network"] is False
    print("private observation validated: exact voluntary records, closed verbs, sealed erasure, reviewed export")


if __name__ == "__main__":
    main()
