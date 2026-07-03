#![cfg(all(
    feature = "citizen",
    feature = "numbers-cas",
    feature = "numbers-complex",
    feature = "numbers-func",
    feature = "numbers-rational",
    feature = "numbers-tensor",
    feature = "pitch-shapes",
    feature = "midi-shapes",
    feature = "music-shapes",
    feature = "sound-shapes",
    feature = "rank",
    feature = "shape",
    feature = "standard-core",
    feature = "audio-dsp",
    feature = "audio-graph-core",
    feature = "music-synth",
    feature = "daw-session",
    feature = "plugin-core",
    feature = "stream-audio",
    feature = "stream-bridge",
    feature = "stream-clock",
    feature = "stream-core",
    feature = "list-cell",
    feature = "list-lazy",
    feature = "table-db",
    feature = "table-fs",
    feature = "table-hash",
    feature = "table-lazy",
    feature = "table-override",
    feature = "table-remote",
    feature = "femm-codec",
    feature = "discrete-runtime",
    feature = "discrete-rank",
    feature = "agent-runner-core",
    feature = "codec-mcp",
    feature = "mcp-cassette",
    feature = "openai-server",
    feature = "server",
    feature = "skill-mcp",
    feature = "topology-core",
    feature = "scene",
    feature = "intent",
    feature = "view",
    feature = "view-doc",
    feature = "web-layout",
))]

use std::fs;

use sim::kernel::{Cx, DefaultFactory, NoopEvalPolicy};

#[test]
fn workspace_citizen_conformance_covers_existing_families() {
    link_workspace_citizen_crates();
    let mut cx = cx();
    sim::citizen::run_registered_conformance(&mut cx).unwrap();

    let symbols = sim::citizen::registered_citizens()
        .map(|info| info.symbol)
        .collect::<std::collections::BTreeSet<_>>();
    for expected in [
        "OverrideTable",
        "numbers/Rational",
        "numbers/Complex",
        "numbers/Cas",
        "numbers/Func",
        "numbers/Tensor",
        "pitch/Pitch",
        "pitch/Interval",
        "pitch/PitchClassMask",
        "pitch/Scale",
        "pitch/Chord",
        "midi/MidiEvent",
        "midi/ChannelMessage",
        "midi/MetaEvent",
        "midi/SmfTrack",
        "midi/SmfFile",
        "music/Note",
        "music/Seq",
        "music/Par",
        "music/Chord",
        "music/Melody",
        "music/Score",
        "sound/Tone",
        "sound/Partial",
        "sound/Envelope",
        "sound/Spectrum",
        "sound/Timbre",
        "sound/TuningDescriptor",
        "rank/Space",
        "rank/Node",
        "rank/Coord",
        "shape/AcceptOnNoDiagnosticsHook",
        "shape/And",
        "shape/Any",
        "shape/Class",
        "shape/DiscardOnDiagnosticPrefixHook",
        "shape/ExactExpr",
        "shape/ExprKind",
        "shape/Hooked",
        "shape/List",
        "shape/Not",
        "shape/Or",
        "shape/Repeat",
        "shape/ScoreFloorHook",
        "shape/Table",
        "shape/TraceMarkHook",
        "shape/Venn",
        "list/ConsList",
        "list/LazyConsList",
        "list/LazyIterList",
        "standard/Profile",
        "standard/FidelityBadge",
        "stream/Metadata",
        "stream/Packet",
        "stream/Clock",
        "stream/PcmFormat",
        "stream-bridge/RenderOptions",
        "stream-bridge/LiftMidiOptions",
        "audio-graph/NodeConfig",
        "audio-graph/Patch",
        "audio-dsp/Config",
        "audio-synth/Preset",
        "plugin-core/PluginDescriptor",
        "daw-session/DawSession",
        "table/DbDir",
        "table/FsDir",
        "table/HashTable",
        "table/LazyTable",
        "table/RemoteDir",
        "femm/Field",
        "femm/Geometry",
        "femm/Material",
        "femm/Mesh",
        "femm/Space",
        "femm/Physics",
        "femm/Solve",
        "femm/Solution",
        "femm/Post",
        "femm/Function",
        "femm/FuncPayload",
        "femm/Model",
        "femm/Sensitivity",
        "femm/Tape",
        "femm/Ode",
        "discrete/Matrix",
        "discrete/SparseMatrix",
        "discrete/Graph",
        "discrete/Edge",
        "discrete/Combination",
        "discrete/Permutation",
        "discrete/FwhtSignal",
        "discrete/MstCertificate",
        "discrete/BitVectorSpace",
        "discrete/SubsetSpace",
        "discrete/CombinationSpace",
        "discrete/PermutationSpace",
        "discrete/BoundedIntVectorSpace",
        "discrete/SimpleGraphSpace",
        "discrete/FwhtSignalSpace",
        "agent-runner/ModelCard",
        "agent-runner/ModelBid",
        "agent-runner/ModelRequest",
        "agent-runner/ModelResponse",
        "agent-runner/ModelUsage",
        "mcp/Request",
        "mcp/Notification",
        "mcp/Response",
        "mcp/ErrorEnvelope",
        "mcp/Error",
        "mcp/CassetteEntry",
        "mcp/AuditEntry",
        "mcp/Cassette",
        "server/Address",
        "server/Frame",
        "openai/GatewayRequest",
        "openai/GatewayResponse",
        "openai/GatewayRun",
        "openai/GatewayEvent",
        "openai/Plan",
        "openai/GatewayKey",
        "skill/Card",
        "skill/McpToolDescriptor",
        "skill/McpCallParams",
        "skill/McpToolResult",
        "skill/McpPromptDescriptor",
        "skill/McpPromptArgument",
        "skill/McpPromptGetParams",
        "skill/McpResourceDescriptor",
        "skill/McpResourceReadParams",
        "topology/Package",
        "topology/Node",
        "topology/Edge",
        "scene/Scene",
        "intent/Intent",
        "view/LensDescriptor",
        "doc/Article",
        "web/Workspace",
    ] {
        assert!(symbols.contains(expected), "missing citizen {expected}");
    }
}

#[test]
fn workspace_citizen_census_is_current() {
    link_workspace_citizen_crates();
    let generated = sim::citizen::citizen_census_markdown();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/generated/citizens.md");
    // Bless: `SIM_BLESS_CITIZENS=1 cargo test --all-features citizen` regenerates
    // the committed census. The census is only complete when every citizen crate
    // is linked, so it must be regenerated under --all-features.
    if std::env::var_os("SIM_BLESS_CITIZENS").is_some() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, &generated).unwrap();
        return;
    }
    let committed = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    });
    assert_eq!(
        committed,
        generated,
        "{} is stale; rerun SIM_BLESS_CITIZENS=1 cargo test --all-features citizen and commit the result",
        path.display()
    );
}

fn link_workspace_citizen_crates() {
    let _ = sim::list_cell::cons_list_class_symbol();
    let _ = sim::list_lazy::lazy_cons_list_class_symbol();
    let _ = sim::list_lazy::lazy_iter_list_class_symbol();
    let _ = sim::table_db::db_dir_class_symbol();
    let _ = sim::table_fs::fs_dir_class_symbol();
    let _ = sim::table_hash::hash_table_class_symbol();
    let _ = sim::table_lazy::lazy_table_class_symbol();
    let _ =
        sim::table_override::install_override_table_lib as fn(&mut Cx) -> sim::kernel::Result<()>;
    let _ = sim::table_remote::remote_dir_class_symbol();
    let _ = sim::femm_codec::femm_field_class_symbol();
    let _ = sim::femm_codec::femm_geometry_class_symbol();
    let _ = sim::femm_codec::femm_material_class_symbol();
    let _ = sim::femm_codec::femm_mesh_class_symbol();
    let _ = sim::femm_codec::femm_space_class_symbol();
    let _ = sim::femm_codec::femm_physics_class_symbol();
    let _ = sim::femm_codec::femm_solve_class_symbol();
    let _ = sim::femm_codec::femm_solution_class_symbol();
    let _ = sim::femm_codec::femm_post_class_symbol();
    let _ = sim::femm_codec::femm_function_class_symbol();
    let _ = sim::femm_codec::femm_func_payload_class_symbol();
    let _ = sim::femm_codec::femm_model_class_symbol();
    let _ = sim::femm_codec::femm_sensitivity_class_symbol();
    let _ = sim::femm_codec::femm_tape_class_symbol();
    let _ = sim::femm_codec::femm_ode_class_symbol();
    let _ = sim::numbers_rational::rational_value_class_symbol();
    let _ = sim::numbers_complex::complex_value_class_symbol();
    let _ = sim::numbers_cas::cas_value_class_symbol();
    let _ = sim::numbers_func::func_class_symbol();
    let _ = sim::numbers_tensor::tensor_value_class_symbol();
    let _ = sim::lib_rank::rank_space_class_symbol();
    let _ = sim::shape::any_shape_class_symbol();
    let _ = sim::shape::expr_kind_shape_class_symbol();
    let _ = sim::shape::trace_mark_hook_class_symbol();
    let _ = sim::shape::venn_shape_set_class_symbol();
    let _ = sim::lib_standard_core::language_profile_class_symbol();
    let _ = sim::lib_stream_core::stream_metadata_class_symbol();
    let _ = sim::lib_stream_core::stream_packet_class_symbol();
    let _ = sim::lib_stream_clock::stream_clock_class_symbol();
    let _ = sim::lib_stream_audio::pcm_format_class_symbol();
    let _ = sim::lib_stream_bridge::stream_bridge_render_options_class_symbol();
    let _ = sim::lib_stream_bridge::stream_bridge_lift_midi_options_class_symbol();
    let _ = sim::lib_audio_graph_core::audio_graph_node_config_class_symbol();
    let _ = sim::lib_audio_graph_core::audio_graph_patch_class_symbol();
    let _ = sim::lib_audio_dsp::dsp_config_class_symbol();
    let _ = sim::lib_music_synth::synth_preset_class_symbol();
    let _ = sim::lib_plugin_core::plugin_descriptor_class_symbol();
    let _ = sim::lib_daw_session::daw_session_class_symbol();
    let _ = sim::lib_pitch_shapes::pitch_class_symbol();
    let _ = sim::lib_pitch_shapes::pitch_interval_class_symbol();
    let _ = sim::lib_pitch_shapes::pitch_class_mask_class_symbol();
    let _ = sim::lib_pitch_shapes::pitch_scale_class_symbol();
    let _ = sim::lib_pitch_shapes::pitch_chord_class_symbol();
    let _ = sim::lib_midi_shapes::midi_event_class_symbol();
    let _ = sim::lib_midi_shapes::midi_channel_message_class_symbol();
    let _ = sim::lib_midi_shapes::midi_meta_event_class_symbol();
    let _ = sim::lib_midi_shapes::midi_smf_track_class_symbol();
    let _ = sim::lib_midi_shapes::midi_smf_file_class_symbol();
    let _ = sim::lib_music_shapes::music_note_class_symbol();
    let _ = sim::lib_music_shapes::music_seq_class_symbol();
    let _ = sim::lib_music_shapes::music_par_class_symbol();
    let _ = sim::lib_music_shapes::music_chord_class_symbol();
    let _ = sim::lib_music_shapes::music_melody_class_symbol();
    let _ = sim::lib_music_shapes::music_score_class_symbol();
    let _ = sim::lib_sound_shapes::sound_tone_class_symbol();
    let _ = sim::lib_sound_shapes::sound_partial_class_symbol();
    let _ = sim::lib_sound_shapes::sound_envelope_class_symbol();
    let _ = sim::lib_sound_shapes::sound_spectrum_class_symbol();
    let _ = sim::lib_sound_shapes::sound_timbre_class_symbol();
    let _ = sim::lib_sound_shapes::sound_tuning_descriptor_class_symbol();
    let _ = sim_lib_discrete::discrete_matrix_class_symbol();
    let _ = sim_lib_discrete::discrete_sparse_matrix_class_symbol();
    let _ = sim_lib_discrete::discrete_graph_class_symbol();
    let _ = sim_lib_discrete::discrete_edge_class_symbol();
    let _ = sim_lib_discrete::discrete_combination_class_symbol();
    let _ = sim_lib_discrete::discrete_permutation_class_symbol();
    let _ = sim_lib_discrete::discrete_fwht_signal_class_symbol();
    let _ = sim_lib_discrete::discrete_mst_certificate_class_symbol();
    let _ = sim_lib_discrete::discrete_bit_vector_space_class_symbol();
    let _ = sim_lib_discrete::discrete_subset_space_class_symbol();
    let _ = sim_lib_discrete::discrete_combination_space_class_symbol();
    let _ = sim_lib_discrete::discrete_permutation_space_class_symbol();
    let _ = sim_lib_discrete::discrete_bounded_int_vector_space_class_symbol();
    let _ = sim_lib_discrete::discrete_simple_graph_space_class_symbol();
    let _ = sim_lib_discrete::discrete_fwht_signal_space_class_symbol();
    let _ = sim::lib_agent_runner_core::model_card_class_symbol();
    let _ = sim::lib_agent_runner_core::model_bid_class_symbol();
    let _ = sim::lib_agent_runner_core::model_request_class_symbol();
    let _ = sim::lib_agent_runner_core::model_response_class_symbol();
    let _ = sim::lib_agent_runner_core::model_usage_class_symbol();
    let _ = sim::codec_mcp::mcp_request_class_symbol();
    let _ = sim::codec_mcp::mcp_notification_class_symbol();
    let _ = sim::codec_mcp::mcp_response_class_symbol();
    let _ = sim::codec_mcp::mcp_error_envelope_class_symbol();
    let _ = sim::codec_mcp::mcp_error_class_symbol();
    let _ = sim::lib_mcp::mcp_cassette_entry_class_symbol();
    let _ = sim::lib_mcp::mcp_audit_entry_class_symbol();
    let _ = sim::lib_mcp::mcp_cassette_class_symbol();
    let _ = sim::lib_server::server_address_class_symbol();
    let _ = sim::lib_server::server_frame_class_symbol();
    let _ = sim::lib_openai_server::citizen::openai_gateway_request_class_symbol();
    let _ = sim::lib_openai_server::citizen::openai_gateway_response_class_symbol();
    let _ = sim::lib_openai_server::citizen::openai_gateway_run_class_symbol();
    let _ = sim::lib_openai_server::citizen::openai_gateway_event_class_symbol();
    let _ = sim::lib_openai_server::citizen::openai_plan_class_symbol();
    let _ = sim::lib_openai_server::citizen::openai_gateway_key_class_symbol();
    let _ = sim::lib_skill::skill_card_descriptor_class_symbol();
    let _ = sim::lib_skill::mcp_tool_descriptor_class_symbol();
    let _ = sim::lib_skill::mcp_call_params_class_symbol();
    let _ = sim::lib_skill::mcp_tool_result_class_symbol();
    let _ = sim::lib_skill::mcp_prompt_descriptor_class_symbol();
    let _ = sim::lib_skill::mcp_prompt_argument_class_symbol();
    let _ = sim::lib_skill::mcp_prompt_get_params_class_symbol();
    let _ = sim::lib_skill::mcp_resource_descriptor_class_symbol();
    let _ = sim::lib_skill::mcp_resource_read_params_class_symbol();
    let _ = sim::lib_topology::topology_package_class_symbol();
    let _ = sim::lib_topology::topology_node_class_symbol();
    let _ = sim::lib_topology::topology_edge_class_symbol();
    let _ = sim::lib_scene::scene_descriptor_class_symbol();
    let _ = sim::lib_intent::intent_descriptor_class_symbol();
    let _ = sim::lib_view::view_lens_descriptor_class_symbol();
    let _ = sim::lib_view_doc::doc_article_class_symbol();
    let _ = sim::lib_web_layout::workspace_descriptor_class_symbol();
}

fn cx() -> Cx {
    Cx::new(
        std::sync::Arc::new(NoopEvalPolicy),
        std::sync::Arc::new(DefaultFactory),
    )
}
