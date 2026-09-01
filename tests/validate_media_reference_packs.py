#!/usr/bin/env python3
"""Offline conformance proof for chosen media and current references."""

from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[1]
MEDIA = ROOT / "packs" / "stewardship" / "media-memory"
REFERENCES = ROOT / "packs" / "stewardship" / "current-reference"


def load(root, name):
    with (root / name).open("rb") as source:
        return tomllib.load(source)


def main():
    media_shapes = {row["id"]: row for row in load(MEDIA, "shapes.toml")["shape"]}
    assert set(media_shapes) == {
        "source-ref", "import-receipt", "episode-position", "book-position",
        "saved-item", "authored-note", "citation-link", "removal", "replacement",
    }
    assert media_shapes["authored-note"]["authority"] == "mia"
    assert media_shapes["replacement"]["constraint"].startswith("explicit-choice")

    media_pack = load(MEDIA, "pack.toml")
    predicates = media_pack["predicates"]
    assert predicates["distinct"] == [
        "imported", "queued", "played", "positioned", "saved", "noted",
        "understood", "endorsed", "finished",
    ]
    assert predicates["mia_authored"] == ["understood", "endorsed", "finished"]
    assert predicates["no_inference"] == "one-predicate-never-implies-another"
    assert set(media_pack["absent_lanes"]) == {
        "spotify", "podcast-republic", "model", "network",
    }
    assert media_pack["privacy"] == {"profile": "none", "inferred_taste": False}

    cases = load(MEDIA, "cases.toml")
    accepted = []
    for case in cases["import_case"]:
        result = "imported" if (
            case["selected"] and case["authorized"]
            and case["origin"] in {"local-export", "user-reference"}
        ) else "refused"
        assert result == case["expected"], case["id"]
        if result == "imported":
            accepted.append(case["id"])
    assert accepted == ["selected-export-entry"]

    traces = {trace["id"]: trace for trace in cases["trace"]}
    assert traces["episode-7"]["source_policy"] == "quotation-permitted"
    assert traces["book-3"]["excerpt"] == ""
    for removal in cases["removal_case"]:
        retained = sorted(set(traces) - {removal["target"]})
        assert retained == removal["expected_search_ids"], removal["id"]
        assert removal["expected_replacement"] == "none"
        assert removal["expected_citation"] in {"broken-reference", "policy-tombstone"}

    reference_shapes = {
        row["id"]: row for row in load(REFERENCES, "shapes.toml")["shape"]
    }
    assert set(reference_shapes) == {
        "current-reference", "replacement-reference", "broken-reference",
        "policy-tombstone",
    }
    reference_pack = load(REFERENCES, "pack.toml")
    assert set(reference_pack["reference"]["required_fields"]) == {
        "source", "jurisdiction", "edition", "checked-at", "expires-at",
    }
    assert reference_pack["removal"]["silent_substitution"] is False
    references = load(REFERENCES, "cases.toml")["reference"]
    assert {row["domain"] for row in references} == {
        "garden", "flock", "food", "product",
    }
    for row in references:
        assert all(row[field] for field in (
            "source", "jurisdiction", "edition", "checked_at", "expires_at",
        ))

    expression = (MEDIA / "chosen-traces.lisp").read_text()
    recipe = (ROOT / "recipes/stewardship/chosen-media-traces/input.lisp").read_text()
    for text in (expression, recipe):
        for forbidden in ("spotify", "podcast-republic", "remote-scrape", "infer-taste"):
            assert forbidden not in text.lower()
    print("media/reference packs validated: selected, local, removable, and citation-honest")


if __name__ == "__main__":
    main()
