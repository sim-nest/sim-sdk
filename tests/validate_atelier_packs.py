#!/usr/bin/env python3
"""Validate the data-authored Atelier room pack closure contract."""

from pathlib import Path
import re
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACKS = ROOT / "packs" / "atelier"
REQUIRED = {
    "version", "content", "title", "input_shape", "output_shape", "route",
    "capabilities", "surface", "ceiling", "degradation", "fallback",
    "specimen", "success_claims", "refusal_claims", "forbidden",
}
CONTENT_ID = re.compile(r"sha256:[0-9a-f]{64}\Z")


def load_packs():
    paths = sorted(PACKS.glob("*.toml"))
    assert len(paths) == 6, f"expected six independent packs, found {len(paths)}"
    packs = {}
    content_ids = set()
    for path in paths:
        with path.open("rb") as source:
            pack = tomllib.load(source)
        missing = REQUIRED - pack.keys()
        assert not missing, f"{path.name}: missing {sorted(missing)}"
        assert pack["version"] == 1
        assert CONTENT_ID.fullmatch(pack["content"]), path
        assert pack["content"] not in content_ids, path
        content_ids.add(pack["content"])
        assert pack["input_shape"].startswith("shape/"), path
        assert pack["output_shape"].startswith("shape/"), path
        assert pack["route"].startswith("route/"), path
        assert pack["specimen"].startswith("specimen/"), path
        assert pack["capabilities"], path
        assert set(pack["capabilities"]) <= set(pack["ceiling"]), path
        assert pack["success_claims"] and pack["refusal_claims"], path
        assert pack["degradation"].strip() and pack["fallback"].strip(), path
        assert pack["forbidden"], path
        packs[pack["title"]] = pack
    return packs


def main():
    packs = load_packs()
    assert set(packs) == {
        "Music atlas", "Physics-edge lens", "Model portfolio", "Bible weave",
        "Mutual porch", "Toy color room",
    }

    music = packs["Music atlas"]
    assert music["stations"] == [
        "exact-transform", "harmonization", "counterpoint-stretto",
        "synthesis-render",
    ]
    assert "implement-algorithm" in music["forbidden"]

    physics = packs["Physics-edge lens"]
    assert "method-cannot-close" in physics["success_claims"]
    assert {"edit-external-problem", "close-external-problem"} <= set(physics["forbidden"])
    assert not any(
        word in capability
        for capability in physics["capabilities"]
        for word in ("close", "edit", "reinterpret", "write")
    )

    models = packs["Model portfolio"]
    assert models["input_shape"] == "shape/frozen-model-pick-evidence"
    assert {"judge-answer", "rank-model", "select-model"} <= set(models["forbidden"])

    bible = packs["Bible weave"]
    assert bible["success_claims"] == [
        "source-text", "textual-witness", "context", "interpretation",
        "reflection", "response",
    ]
    assert "produce-doctrine" in bible["forbidden"]

    porch = packs["Mutual porch"]
    assert "decline-succeeded" in porch["success_claims"]
    assert porch["surface"] == "mutual-minimum"

    toy = packs["Toy color room"]
    assert toy["capabilities"] == ["data-read"]
    assert toy["route"] == "route/identity-expression"
    print("six independent Atelier room closures validated")


if __name__ == "__main__":
    main()
