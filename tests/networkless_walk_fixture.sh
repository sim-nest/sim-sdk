#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
thought="$root/recipes/continuity/expedition-thought/product.toml"
cards="$root/recipes/continuity/quiet-stewardship-cards/product.toml"
rust_test="$root/tests/continuity_exports.rs"

for product in "$thought" "$cards"; do
    sed '/\[deleted_before_boot\]/,$d' "$product" > "${TMPDIR:-/tmp}/sim-networkless-minimal-$$"
    minimal="${TMPDIR:-/tmp}/sim-networkless-minimal-$$"
    for required in continuity-organ android-capsule phone-surface journal turn phone-scene; do
        grep -q "$required" "$minimal"
    done
    for forbidden in audio-role watch-role halo-role speech-role model-role audio-scene watch-scene halo-scene speech-cache model-cache; do
        ! grep -q "$forbidden" "$minimal"
        grep -q "$forbidden" "$product"
    done
    grep -q 'root = "phone"' "$product"
    grep -q 'network = "denied"' "$product"
    rm -f "$minimal"
done

grep -q 'continuation = "by-content-id"' "$thought"
grep -q 'result = "pending-honestly"' "$thought"
grep -q 'card_count = "0..3"' "$cards"
grep -q 'acknowledgement = "optional-one"' "$cards"
grep -q 'refused_on_worn_or_audible = \["private-note", "finance"\]' "$cards"

for case in no-accessories modeled-audio modeled-watch modeled-halo missing-speech-model route-loss endpoint-removal process-death network-denied manual-continuation; do
    grep -q "\"$case\"" "$rust_test"
done
grep -q 'journal and turn identity diverged' "$rust_test"
grep -q 'turn and phone Scene identity diverged' "$rust_test"
grep -q 'continuity_production_has_no_product_or_person_policy_branch' "$rust_test"

echo "networkless walk fixture conformance: OK"
