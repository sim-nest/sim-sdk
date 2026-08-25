#!/usr/bin/env python3
"""Deterministic proof for generic stewardship adoption and subtraction."""

from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "recipes/stewardship/adopt-subtract-composition/composition.toml"
RECIPE = FIXTURE.with_name("recipe.toml")


def main():
    with FIXTURE.open("rb") as source:
        document = tomllib.load(source)
    with RECIPE.open("rb") as source:
        recipe = tomllib.load(source)

    assert recipe["id"] == "adopt-subtract-composition"
    assert recipe["codec"] == "lisp"
    assert recipe["setup"] == "input.lisp"
    assert recipe["purpose"] == "README.md"
    assert recipe["network"] is False

    assert document["brief_profile"] == "BRIDGE"
    assert document["brief_state"] == "human-reviewed-before-preview"
    assert document["assembly"] == "existing-pack-roots-and-pure-expressions-only"
    assert document["missing_contract_policy"] == "stop-and-record"

    preview = document["preview"]
    assert set(preview) == {
        "closure", "data_access", "key_lanes", "outputs", "channels",
        "attention", "retention", "leakage", "deletion", "manual_fallback",
    }

    adoption = document["adoption"]
    assert adoption["transaction"] == "atomic"
    assert adoption["rollback"] == "restore-prior-root-set"
    assert adoption["rebuild_after_ephemeral_deletion"] is True
    ephemeral = {"chat", "model-cache", "derived-views", "workshop-scratch"}
    assert set(adoption["ephemeral_inputs"]) == ephemeral
    assert not ephemeral & set(adoption["durable_inputs"])

    subtraction = document["subtraction"]
    retired = {subtraction["retired_field"], subtraction["retired_pack"]}
    assert set(subtraction["remove_from"]) == {"closure", "projections", "grants", "recipe-roots"}
    after = document["after_subtraction"]
    assert subtraction["retired_pack"] not in after["closure"]
    assert subtraction["retired_pack"] not in after["recipe_roots"]
    assert all(subtraction["retired_field"] not in value for value in after["projections"] + after["grants"])
    tombstoned = set(subtraction["tombstones"])
    assert retired == tombstoned
    replay_candidates = set(preview["closure"]) | {subtraction["retired_field"]}
    replay_result = replay_candidates - tombstoned
    assert not retired & replay_result
    assert subtraction["retention_action"] == "delete-sealed-content-at-expiry"
    assert subtraction["replay_policy"] == "tombstones-win"

    porch = document["mutual_porch"]
    assert porch == {"invitation": "optional", "decision": "declined", "outcome": "success-no-shared-object"}
    print("adopt/subtract validated: ephemeral rebuild, complete retirement, declined porch success")


if __name__ == "__main__":
    main()
