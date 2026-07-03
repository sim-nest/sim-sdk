use sim::{
    kernel::Symbol,
    lib_stream_core::{BridgeLatency, ClockDomain, LatencyClass, RateContract, TransportProfile},
    lib_stream_fabric::{server_buffered_preview_profile, server_render_return_profile},
    lib_topology::{
        Edge, Graph, Node, PlacementNodeProfile, PlacementRefusalReason, PlacementReport, PortRef,
        SiteMap, SiteProfile, place,
    },
};

use crate::support::{CONFORMANCE_CONTRACT, cx};

pub(crate) const MATRIX_PATH: &str = "crates/sim-conformance/tests/spec/placement.rs";

const SINGLE_SITE_REPORT_HASH: &str = "d923b4bcc2cd8f09";
const SINGLE_SITE_AUDIO_HASH: &str = "babd989bc82bdde5";
const MULTI_THREAD_REPORT_HASH: &str = "2715c2b4b8218e73";
const MULTI_THREAD_AUDIO_HASH: &str = "adc664e0e4c68349";
const MULTI_PROCESS_REPORT_HASH: &str = "b2f112526fab5449";
const MULTI_PROCESS_AUDIO_HASH: &str = "3beff72b6648ed2d";

#[derive(Clone, Copy)]
struct PlacementCase {
    name: &'static str,
    site_map: fn() -> SiteMap,
    expect: PlacementExpect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlacementExpect {
    Deterministic {
        report_hash: &'static str,
        audio_hash: &'static str,
    },
    WithinLatency(LatencyClass),
    Diagnosed(&'static str),
}

#[test]
fn placement_matrix_records_path_site_maps_and_expectations() {
    assert_eq!(
        MATRIX_PATH,
        "crates/sim-conformance/tests/spec/placement.rs"
    );
    assert!(CONFORMANCE_CONTRACT.contains(MATRIX_PATH));
    assert_eq!(
        placement_cases().map(|case| case.name),
        [
            "single-site",
            "multi-thread",
            "multi-process",
            "server-preview",
            "lan-peer",
            "browser-clock-crossing",
        ]
    );
    assert_eq!(
        placement_cases()
            .into_iter()
            .filter(|case| matches!(case.expect, PlacementExpect::Deterministic { .. }))
            .count(),
        3
    );
    assert_eq!(
        placement_cases()
            .into_iter()
            .filter(|case| matches!(case.expect, PlacementExpect::WithinLatency(_)))
            .count(),
        2
    );
    assert_eq!(
        placement_cases()
            .into_iter()
            .filter(|case| matches!(case.expect, PlacementExpect::Diagnosed(_)))
            .count(),
        1
    );
}

#[test]
fn placement_deterministic_sites_match_golden_report_and_audio_hashes() {
    let mut mismatches = Vec::new();
    for case in placement_cases() {
        let PlacementExpect::Deterministic {
            report_hash,
            audio_hash,
        } = case.expect
        else {
            continue;
        };
        let report = report_for(case);
        assert!(
            report.is_accepted(),
            "{} must be a deterministic accepted placement: {:?}",
            case.name,
            report.refusals
        );
        let actual_report_hash = hash_text(&canonical_report(&report));
        let actual_audio_hash = hash_audio(&audio_fixture(case.name, &report));
        if actual_report_hash != report_hash || actual_audio_hash != audio_hash {
            mismatches.push(format!(
                "{} report={} audio={}",
                case.name, actual_report_hash, actual_audio_hash
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "golden placement hashes changed:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn placement_network_cases_match_declared_latency_classes() {
    for case in placement_cases() {
        let PlacementExpect::WithinLatency(expected) = case.expect else {
            continue;
        };
        let report = report_for(case);
        assert!(
            report.is_accepted(),
            "{} must be accepted within its declared latency class: {:?}",
            case.name,
            report.refusals
        );
        assert_ne!(
            expected,
            LatencyClass::SampleExact,
            "{} must not claim sample-exact network behavior",
            case.name
        );
        assert!(
            report
                .placed
                .iter()
                .any(|node| node.latency_class == expected),
            "{} must carry {} latency in the placement report",
            case.name,
            expected.wire_label()
        );
    }
    assert_eq!(
        server_buffered_preview_profile().latency_class(),
        LatencyClass::BufferedPreview
    );
    assert_eq!(
        server_render_return_profile().latency_class(),
        LatencyClass::OfflineRender
    );
    assert_eq!(
        TransportProfile::lan_midi_control().latency_class(),
        LatencyClass::Interactive
    );
}

#[test]
fn placement_nondeterministic_crossings_are_diagnosed() {
    for case in placement_cases() {
        let PlacementExpect::Diagnosed(expected) = case.expect else {
            continue;
        };
        let report = report_for(case);
        let diagnostics = report
            .bridges
            .iter()
            .flat_map(|bridge| bridge.descriptor.diagnostics())
            .map(Symbol::as_qualified_str)
            .collect::<Vec<_>>();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.as_str() == expected),
            "{} must record {expected}, got {diagnostics:?}",
            case.name
        );
    }
}

fn placement_cases() -> [PlacementCase; 6] {
    [
        PlacementCase {
            name: "single-site",
            site_map: single_site_map,
            expect: PlacementExpect::Deterministic {
                report_hash: SINGLE_SITE_REPORT_HASH,
                audio_hash: SINGLE_SITE_AUDIO_HASH,
            },
        },
        PlacementCase {
            name: "multi-thread",
            site_map: multi_thread_map,
            expect: PlacementExpect::Deterministic {
                report_hash: MULTI_THREAD_REPORT_HASH,
                audio_hash: MULTI_THREAD_AUDIO_HASH,
            },
        },
        PlacementCase {
            name: "multi-process",
            site_map: multi_process_map,
            expect: PlacementExpect::Deterministic {
                report_hash: MULTI_PROCESS_REPORT_HASH,
                audio_hash: MULTI_PROCESS_AUDIO_HASH,
            },
        },
        PlacementCase {
            name: "server-preview",
            site_map: server_preview_map,
            expect: PlacementExpect::WithinLatency(LatencyClass::BufferedPreview),
        },
        PlacementCase {
            name: "lan-peer",
            site_map: lan_peer_map,
            expect: PlacementExpect::WithinLatency(LatencyClass::Interactive),
        },
        PlacementCase {
            name: "browser-clock-crossing",
            site_map: browser_clock_crossing_map,
            expect: PlacementExpect::Diagnosed("stream/bridge-diagnostic/jitter-buffer"),
        },
    ]
}

fn report_for(case: PlacementCase) -> PlacementReport {
    let mut cx = cx();
    let graph = placement_graph();
    place(&mut cx, &graph, &(case.site_map)()).expect("placement graph compiles")
}

fn placement_graph() -> Graph {
    let mut graph = Graph::minimal("placement-conformance");
    let mut fx = Node::named("fx", "call");
    fx.target = Some(sim::kernel::Expr::Symbol(Symbol::qualified(
        "conformance",
        "gain",
    )));
    graph.nodes = vec![Node::named("in", "in"), fx, Node::named("out", "out")];
    graph.edges = vec![
        Edge::new(0, PortRef::output("in"), PortRef::input("fx")),
        Edge::new(1, PortRef::output("fx"), PortRef::input("out")),
    ];
    graph
}

fn single_site_map() -> SiteMap {
    SiteMap::new(SiteProfile::audio_clock("audio"))
        .with_node_profile("in", block_profile())
        .with_node_profile(
            "fx",
            block_profile().with_latency(BridgeLatency::frames(16)),
        )
        .with_node_profile("out", block_profile())
}

fn multi_thread_map() -> SiteMap {
    SiteMap::new(SiteProfile::audio_clock("audio"))
        .with_site(SiteProfile::local_worker("thread"))
        .assign_node("fx", "thread")
        .with_node_profile("in", block_profile())
        .with_node_profile(
            "fx",
            block_profile().with_latency(BridgeLatency::frames(32)),
        )
        .with_node_profile("out", block_profile())
}

fn multi_process_map() -> SiteMap {
    SiteMap::new(SiteProfile::audio_clock("audio"))
        .with_site(SiteProfile::local_worker("process"))
        .assign_node("fx", "process")
        .with_node_profile("in", block_profile())
        .with_node_profile(
            "fx",
            block_profile().with_latency(BridgeLatency::frames(64)),
        )
        .with_node_profile("out", block_profile())
}

fn server_preview_map() -> SiteMap {
    SiteMap::new(SiteProfile::audio_clock("audio"))
        .with_site(SiteProfile::buffered_remote("server"))
        .assign_node("fx", "server")
        .with_node_profile("in", block_profile())
        .with_node_profile(
            "fx",
            PlacementNodeProfile::new(
                RateContract::new(
                    ClockDomain::ServerFrame,
                    LatencyClass::BufferedPreview,
                    None,
                ),
                false,
            )
            .with_latency(BridgeLatency::packets(2)),
        )
        .with_node_profile("out", block_profile())
}

fn lan_peer_map() -> SiteMap {
    SiteMap::new(SiteProfile::audio_clock("audio"))
        .with_site(SiteProfile::local_worker("lan-peer"))
        .assign_node("fx", "lan-peer")
        .with_node_profile("in", block_profile())
        .with_node_profile(
            "fx",
            PlacementNodeProfile::new(RateContract::control(), false)
                .with_latency(BridgeLatency::packets(1)),
        )
        .with_node_profile("out", block_profile())
}

fn browser_clock_crossing_map() -> SiteMap {
    SiteMap::new(SiteProfile::audio_clock("audio"))
        .with_site(SiteProfile::buffered_remote("browser"))
        .assign_node("fx", "browser")
        .with_node_profile("in", block_profile())
        .with_node_profile(
            "fx",
            PlacementNodeProfile::new(
                RateContract::new(ClockDomain::Wall, LatencyClass::BufferedPreview, None),
                false,
            ),
        )
        .with_node_profile("out", block_profile())
}

fn block_profile() -> PlacementNodeProfile {
    PlacementNodeProfile::block_local()
}

fn canonical_report(report: &PlacementReport) -> String {
    let mut text = String::new();
    text.push_str("placed:");
    for node in &report.placed {
        text.push_str(&format!(
            "{}@{}:{}:{}:{};",
            node.node.as_symbol().as_qualified_str(),
            node.site.as_symbol().as_qualified_str(),
            node.clock_domain.wire_label(),
            node.latency_class.wire_label(),
            node.realtime_pin
        ));
    }
    text.push_str("|bridges:");
    for bridge in &report.bridges {
        text.push_str(&format!(
            "{}:{}>{}:{}>{}:{}:{};",
            bridge.edge.0,
            bridge.from.as_symbol().as_qualified_str(),
            bridge.to.as_symbol().as_qualified_str(),
            bridge.from_site.as_symbol().as_qualified_str(),
            bridge.to_site.as_symbol().as_qualified_str(),
            bridge.descriptor.name(),
            bridge
                .descriptor
                .diagnostics()
                .iter()
                .map(Symbol::as_qualified_str)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    text.push_str("|latency:");
    for latency in &report.latency {
        text.push_str(&format!(
            "{}@{}:{}f:{}p:{};",
            latency.node.as_symbol().as_qualified_str(),
            latency.site.as_symbol().as_qualified_str(),
            latency.latency.frame_count(),
            latency.latency.packet_count(),
            latency.latency_class.wire_label()
        ));
    }
    text.push_str("|refusals:");
    for refusal in &report.refusals {
        text.push_str(&format!(
            "{}@{}:{};",
            refusal.node.as_symbol().as_qualified_str(),
            refusal.site.as_symbol().as_qualified_str(),
            refusal_reason(&refusal.reason)
        ));
    }
    text
}

fn refusal_reason(reason: &PlacementRefusalReason) -> &'static str {
    match reason {
        PlacementRefusalReason::UnknownSite => "unknown-site",
        PlacementRefusalReason::RealtimePinViolation => "realtime-pin-violation",
        PlacementRefusalReason::UnsupportedLatencyClass => "unsupported-latency-class",
    }
}

fn audio_fixture(case_name: &str, report: &PlacementReport) -> Vec<i16> {
    let report = canonical_report(report);
    let mut seed_text = String::with_capacity(case_name.len() + report.len() + 1);
    seed_text.push_str(case_name);
    seed_text.push('|');
    seed_text.push_str(&report);
    let seed = hash_u64(seed_text.as_bytes());
    (0..8)
        .map(|index| {
            let shift = (index % 4) * 16;
            let sample = ((seed >> shift) & 0xffff) as u16;
            i16::from_le_bytes(sample.to_le_bytes())
        })
        .collect()
}

fn hash_text(text: &str) -> String {
    format!("{:016x}", hash_u64(text.as_bytes()))
}

fn hash_audio(samples: &[i16]) -> String {
    let bytes = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect::<Vec<_>>();
    format!("{:016x}", hash_u64(&bytes))
}

fn hash_u64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}
