#!/usr/bin/env python3
"""Deterministic conformance proof for the quiet stewardship cadence."""

from pathlib import Path
import tomllib


ROOT = Path(__file__).resolve().parents[1]
CADENCE = ROOT / "packs" / "stewardship" / "cadence"


def load(name):
    with (CADENCE / name).open("rb") as source:
        return tomllib.load(source)


def main():
    pack = load("pack.toml")
    projections = [load(name) for name in pack["projections"]]
    assert [row["id"] for row in projections] == ["daily", "weekly", "monthly", "seasonal"]
    assert all(row["source_truth"] == "none" for row in projections)
    assert all(set(row["reads"]) <= {"copied-claim", "attention", "reconciliation-status", "completed-pack"} for row in projections)
    assert pack["input_contract"] == "declared-copied-claims-only"
    assert pack["composition"]["roles"] == ["finance", "kitchen", "garden", "flock", "private", "media"]
    assert pack["composition"]["replaceable"] is True
    assert pack["composition"]["shared_source_handles"] is False
    assert pack["composition"]["conductor"] == "none"
    assert pack["composition"]["always_on_agent"] is False

    ceilings = {row["id"]: row for row in pack["channel"]}
    for row in ceilings.values():
        assert "private-observation" in row["forbidden"]
    for channel in ("workstation-kitchen", "telegram", "worn", "export"):
        assert "finance-detail" in ceilings[channel]["forbidden"]
    assert ceilings["export"]["requires"] == ["preview", "explicit-confirmation"]

    week = load("week.toml")
    days = week["day"]
    assert len(days) == 7
    assert {day["id"] for day in days} == {
        "monday-burst", "tuesday-stale", "wednesday-refusal",
        "thursday-no-input", "friday-lock-revoke", "saturday-unavailable",
        "sunday-removed-pack",
    }
    for day in days:
        assert pack["budget"]["daily_cards_min"] <= len(day["cards"]) <= pack["budget"]["daily_cards_max"], day["id"]
        assert day["seconds"] <= pack["budget"]["normal_daily_seconds_max"], day["id"]
        assert day["flock_seconds"] <= pack["budget"]["flock_common_path_seconds_max"], day["id"]
        channel = ceilings[day["channel"]]
        assert set(day["fields"]) <= set(channel["allowed"]), day["id"]
        assert not set(day["fields"]) & set(channel["forbidden"]), day["id"]
        if any(value.endswith(":removed") or value.endswith(":locked") for value in day["inputs"]):
            assert not day["fields"], day["id"]
    assert {row["outcome"] for row in days} <= set(pack["successful_outcomes"]["states"])
    reconciliation = {row["id"]: row for row in week["reconciliation_case"]}
    assert reconciliation["closed"]["expected"] == "refused"
    assert reconciliation["explicit-open"]["expected"] == "shown-in-session-only"
    print("quiet cadence validated: 7 days, <=3 cards/day, <=120s/day, channel ceilings intact")


if __name__ == "__main__":
    main()
