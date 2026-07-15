use crate::codec::{Input, decode_with_codec, encode_with_codec};
use crate::codec_bridge::{
    BridgeBook, BridgeCodecLib, BridgeFramePayload, BridgeHeader, BridgePacket, BridgePart,
    BridgePatchPayload, BridgeProvenance, BridgeReviewPayload, BridgeScore, BridgeVotePayload,
    BridgeWarrantPolicy, BridgeWeavePayload, BridgeWeaveRow, ask_profile_symbol,
    brief_profile_symbol, collab_profile_symbol, decode_bridge_text, encode_bridge_text,
    expr_to_packet, loom_profile_symbol, packet_to_expr, stamp_packet_cid,
};
use crate::kernel::{Cx, EncodeOptions, Expr, Symbol};
use crate::lib_bridge::{MergePolicy, ask_packet, bridge_brief, merge_bridge_replies, prepare_packet, rx_check};

fn cx() -> Cx {
    let mut cx = super::support::cx();
    let bridge = BridgeCodecLib::new(cx.registry_mut().fresh_codec_id());
    cx.load_lib(&bridge).unwrap();
    cx
}

fn codec_symbol() -> Symbol {
    Symbol::qualified("codec", "bridge")
}

fn core_map() -> Expr {
    Expr::Symbol(Symbol::qualified("core", "Map"))
}

fn core_string() -> Expr {
    Expr::Symbol(Symbol::qualified("core", "String"))
}

fn entry(name: &str, value: Expr) -> (Expr, Expr) {
    (Expr::Symbol(Symbol::new(name)), value)
}

fn answer_map(text: &str) -> Expr {
    Expr::Map(vec![entry("answer", Expr::String(text.to_owned()))])
}

fn frame_payload(frame: &str) -> Expr {
    let payload = BridgeFramePayload::new(Symbol::qualified("bridge", frame));
    if frame == "produce-artifact" {
        return payload
            .with_slot(
                Symbol::new("what"),
                Expr::Symbol(Symbol::qualified("bridge", "proposal")),
            )
            .with_slot(
                Symbol::new("target"),
                Expr::Symbol(Symbol::qualified("bridge", "sdk-fixture")),
            )
            .to_expr();
    }
    payload.to_expr()
}

fn brief_request() -> BridgePacket {
    bridge_brief(
        "model:drafter",
        BridgeFramePayload::new(Symbol::qualified("bridge", "produce-artifact"))
            .with_slot(
                Symbol::new("what"),
                Expr::Symbol(Symbol::qualified("bridge", "proposal")),
            )
            .with_slot(
                Symbol::new("target"),
                Expr::Symbol(Symbol::qualified("bridge", "sdk-fixture")),
            ),
        core_map(),
    )
    .unwrap()
}

fn reply_to_request(parent: &BridgePacket, from: &str, payload: Expr) -> BridgePacket {
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("reply"),
            from: from.to_owned(),
            to: vec![parent.header.from.clone()],
            role: Symbol::new("implementer"),
            parents: vec![parent.header.cid.clone().unwrap()],
            task: Symbol::new("T2"),
            output: Symbol::new("O2"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body: vec![
            BridgePart {
                id: Symbol::new("T2"),
                kind: Symbol::qualified("bridge", "Frame"),
                payload: frame_payload("answer"),
            },
            BridgePart {
                id: Symbol::new("O2"),
                kind: Symbol::qualified("bridge", "Return"),
                payload,
            },
        ],
        warrant: None,
    }
}

fn return_reply(parent: &BridgePacket, from: &str, payload: Expr) -> BridgePacket {
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("reply"),
            from: from.to_owned(),
            to: vec![parent.header.from.clone()],
            role: Symbol::new("implementer"),
            parents: vec![parent.header.cid.clone().unwrap()],
            task: Symbol::new("A1"),
            output: Symbol::new("A1"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body: vec![BridgePart {
            id: Symbol::new("A1"),
            kind: Symbol::qualified("bridge", "Return"),
            payload,
        }],
        warrant: None,
    }
}

fn loom_request() -> BridgePacket {
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("request"),
            from: "sim".to_owned(),
            to: vec!["model:drafter".to_owned()],
            role: Symbol::new("implementer"),
            parents: Vec::new(),
            task: Symbol::new("W1"),
            output: Symbol::new("O1"),
            ceiling: vec![Symbol::qualified("ai", "run")],
            context: vec![Symbol::new("Seed1")],
            provenance: BridgeProvenance::default(),
        },
        body: vec![
            BridgePart {
                id: Symbol::new("Seed1"),
                kind: Symbol::qualified("bridge", "Frame"),
                payload: frame_payload("produce-artifact"),
            },
            BridgePart {
                id: Symbol::new("W1"),
                kind: Symbol::qualified("bridge", "Weave"),
                payload: BridgeWeavePayload::new(vec![BridgeWeaveRow::new(
                    "draft",
                    Symbol::new("reply"),
                    vec![(Symbol::new("input"), Expr::Symbol(Symbol::new("Seed1")))],
                )])
                .to_expr(),
            },
            BridgePart {
                id: Symbol::new("O1"),
                kind: Symbol::qualified("bridge", "Return"),
                payload: Expr::Map(vec![entry(
                    "codec",
                    Expr::Symbol(Symbol::qualified("codec", "bridge")),
                )]),
            },
        ],
        warrant: None,
    }
}

fn review_patch_reply(parent: &BridgePacket, from: &str, replacement: Expr) -> BridgePacket {
    let parent_cid = parent.header.cid.clone().unwrap();
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("patch"),
            from: from.to_owned(),
            to: vec!["sim".to_owned()],
            role: Symbol::new("reviewer"),
            parents: vec![parent_cid.clone()],
            task: Symbol::new("P1"),
            output: Symbol::new("P1"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body: vec![
            BridgePart {
                id: Symbol::new("R1"),
                kind: Symbol::qualified("bridge", "Review"),
                payload: BridgeReviewPayload::new("body/O2/payload", "tighten the packet")
                    .to_expr(),
            },
            BridgePart {
                id: Symbol::new("P1"),
                kind: Symbol::qualified("bridge", "Patch"),
                payload: BridgePatchPayload::new(parent_cid, "body/O2/payload", replacement)
                    .to_expr(),
            },
        ],
        warrant: None,
    }
}

fn vote_reply(parent: &BridgePacket, from: &str) -> BridgePacket {
    BridgePacket {
        header: BridgeHeader {
            cid: None,
            move_kind: Symbol::new("vote"),
            from: from.to_owned(),
            to: vec!["sim".to_owned()],
            role: Symbol::new("judge"),
            parents: vec![parent.header.cid.clone().unwrap()],
            task: Symbol::new("V1"),
            output: Symbol::new("V1"),
            ceiling: Vec::new(),
            context: Vec::new(),
            provenance: BridgeProvenance::default(),
        },
        body: vec![BridgePart {
            id: Symbol::new("V1"),
            kind: Symbol::qualified("bridge", "Vote"),
            payload: BridgeVotePayload::new(
                "body/O2/payload",
                vec![BridgeScore::new(
                    Symbol::new("correctness"),
                    1,
                    "preserves the BRIDGE contract",
                )],
            )
            .to_expr(),
        }],
        warrant: None,
    }
}

fn assert_codec_roundtrip(cx: &mut Cx, packet: &BridgePacket) {
    let expr = packet_to_expr(packet);
    let encoded = encode_with_codec(cx, &codec_symbol(), &expr, EncodeOptions::default())
        .unwrap()
        .into_text()
        .unwrap();
    let decoded = decode_with_codec(cx, &codec_symbol(), Input::Text(encoded), Default::default())
        .unwrap();

    assert_eq!(expr_to_packet(&decoded).unwrap(), *packet);
}

fn assert_line_roundtrip(packet: &BridgePacket) {
    let book = BridgeBook::standard();
    let text = encode_bridge_text(packet, &book).unwrap();
    let decoded = decode_bridge_text(&text, &book).unwrap();

    assert_eq!(decoded, *packet);
}

#[test]
fn codec_bridge_roundtrips_packet_expr_through_sdk_export() {
    let mut cx = cx();
    let book = BridgeBook::standard().with_warrant_policy(BridgeWarrantPolicy::Verify);
    let packet = prepare_packet(&mut cx, &book, &brief_request()).unwrap();

    assert!(packet.warrant.is_some());
    assert!(rx_check(&mut cx, &book, &packet, None).unwrap().accepted());
    assert_codec_roundtrip(&mut cx, &packet);
    assert_line_roundtrip(&packet);
}

#[test]
fn sdk_bridge_profiles_and_reply_tree_validate() {
    let mut cx = cx();
    let book = BridgeBook::standard();
    let verify_book = BridgeBook::standard().with_warrant_policy(BridgeWarrantPolicy::Verify);
    let brief = prepare_packet(&mut cx, &verify_book, &brief_request()).unwrap();
    assert_eq!(book.profiles.matching_profiles(&brief), vec![brief_profile_symbol()]);

    let reply = stamp_packet_cid(&reply_to_request(
        &brief,
        "model:drafter",
        answer_map("draft answer"),
    ))
    .unwrap();
    let reply_report = rx_check(&mut cx, &book, &reply, Some(&brief)).unwrap();
    assert!(reply_report.accepted());

    let bad_reply = stamp_packet_cid(&reply_to_request(&brief, "model:drafter", Expr::Bool(false)))
        .unwrap();
    let bad_report = rx_check(&mut cx, &book, &bad_reply, Some(&brief)).unwrap();
    assert!(!bad_report.accepted());
    assert!(bad_report.obligations.iter().any(|obligation| {
        obligation.expected == "parent Return contract"
    }));

    let ask = ask_packet(
        &mut cx,
        "bridge/answer-question",
        vec![("question".to_owned(), Expr::String("status?".to_owned()))],
        core_string(),
        "model:drafter",
    )
    .unwrap();
    let ask = prepare_packet(&mut cx, &verify_book, &ask).unwrap();
    assert_eq!(book.profiles.matching_profiles(&ask), vec![ask_profile_symbol()]);
    let ask_reply = stamp_packet_cid(&return_reply(
        &ask,
        "model:drafter",
        Expr::String("green".to_owned()),
    ))
    .unwrap();
    assert!(rx_check(&mut cx, &book, &ask_reply, Some(&ask)).unwrap().accepted());

    let loom = prepare_packet(&mut cx, &verify_book, &loom_request()).unwrap();
    assert!(
        book.profiles
            .matching_profiles(&loom)
            .contains(&loom_profile_symbol())
    );

    let review = stamp_packet_cid(&review_patch_reply(
        &reply,
        "model:reviewer",
        answer_map("reviewed answer"),
    ))
    .unwrap();
    let vote = stamp_packet_cid(&vote_reply(&reply, "model:judge")).unwrap();
    assert!(
        book.profiles
            .matching_profiles(&review)
            .contains(&collab_profile_symbol())
    );
    assert!(rx_check(&mut cx, &book, &review, Some(&reply)).unwrap().accepted());
    assert!(rx_check(&mut cx, &book, &vote, Some(&reply)).unwrap().accepted());
    assert!(
        book.moves
            .check_move(
                &Symbol::new("vote"),
                &[Symbol::new("request")],
                &[Symbol::qualified("bridge", "Vote")],
            )
            .is_err()
    );
}

#[test]
fn dogfood_multi_model_authoring_round_replays() {
    let mut cx = cx();
    let book = BridgeBook::standard();
    let verify_book = BridgeBook::standard().with_warrant_policy(BridgeWarrantPolicy::Verify);
    let root = prepare_packet(&mut cx, &verify_book, &brief_request()).unwrap();
    let drafter = stamp_packet_cid(&reply_to_request(
        &root,
        "model:drafter",
        answer_map("first construction"),
    ))
    .unwrap();
    let reviewer = stamp_packet_cid(&review_patch_reply(
        &drafter,
        "model:reviewer",
        answer_map("reviewed construction"),
    ))
    .unwrap();
    let judge = stamp_packet_cid(&vote_reply(&drafter, "model:judge")).unwrap();
    let merged = merge_bridge_replies(
        &drafter,
        &[reviewer.clone(), judge.clone()],
        &MergePolicy::SynthesisThenVote {
            synthesizer: "model:reviewer".to_owned(),
            min_votes: 1,
        },
    )
    .unwrap();

    for packet in [&root, &drafter, &reviewer, &judge, &merged] {
        assert_codec_roundtrip(&mut cx, packet);
        assert_line_roundtrip(packet);
    }
    assert!(rx_check(&mut cx, &book, &drafter, Some(&root)).unwrap().accepted());
    assert!(
        rx_check(&mut cx, &book, &reviewer, Some(&drafter))
            .unwrap()
            .accepted()
    );
    assert!(rx_check(&mut cx, &book, &judge, Some(&drafter)).unwrap().accepted());
    assert!(rx_check(&mut cx, &book, &merged, Some(&drafter)).unwrap().accepted());
}
