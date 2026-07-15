macro_rules! cookbook_directory_codecs {
    ($m:ident) => {
        $m!(
            "codec/json",
            "JSON codec",
            "codec-json",
            Some(crate::codec_json::RECIPES),
            || Box::new(crate::codec_json::JsonCodecLib::new(codec_id(1)))
        );
        $m!(
            "codec/binary",
            "Binary codec",
            "codec-binary",
            Some(crate::codec_binary::RECIPES),
            || Box::new(crate::codec_binary::BinaryCodecLib::new(codec_id(2)))
        );
        $m!(
            "codec/binary-base64",
            "Binary base64 codec",
            "codec-binary-base64",
            Some(crate::codec_binary_base64::RECIPES),
            || Box::new(crate::codec_binary_base64::BinaryBase64CodecLib::new(
                codec_id(3)
            ))
        );
        $m!(
            "codec/bitwise",
            "Bitwise codec",
            "codec-bitwise",
            Some(crate::codec_bitwise::RECIPES),
            || Box::new(crate::codec_bitwise::BitwiseCodecLib::new(codec_id(4)))
        );
        $m!(
            "codec/bitwise-base64",
            "Bitwise base64 codec",
            "codec-bitwise-base64",
            Some(crate::codec_bitwise_base64::RECIPES),
            || Box::new(crate::codec_bitwise_base64::BitwiseBase64CodecLib::new(
                codec_id(5)
            ))
        );
        $m!(
            "codec/chat",
            "Chat codec",
            "codec-chat",
            Some(crate::codec_chat::RECIPES),
            || Box::new(crate::codec_chat::ChatCodecLib::new(codec_id(6)))
        );
        $m!(
            "codec/bridge",
            "Bridge codec",
            "codec-bridge",
            Some(crate::codec_bridge::RECIPES),
            || Box::new(crate::codec_bridge::BridgeCodecLib::new(codec_id(19)))
        );
        $m!(
            "codec/ollama",
            "Ollama chat codec",
            "codec-chat",
            None,
            || Box::new(crate::codec_chat::OllamaCodecLib::new(codec_id(7)))
        );
        $m!(
            "codec/anthropic",
            "Anthropic chat codec",
            "codec-chat",
            None,
            || Box::new(crate::codec_chat::AnthropicCodecLib::new(codec_id(9)))
        );
        $m!(
            "codec/lm-studio",
            "LM Studio chat codec",
            "codec-chat",
            None,
            || Box::new(crate::codec_chat::LmStudioCodecLib::new(codec_id(10)))
        );
        $m!(
            "codec/lemonade",
            "Lemonade chat codec",
            "codec-chat",
            None,
            || Box::new(crate::codec_chat::LemonadeCodecLib::new(codec_id(11)))
        );
        $m!(
            "codec/mcp",
            "MCP codec",
            "codec-mcp",
            Some(crate::codec_mcp::RECIPES),
            || Box::new(crate::codec_mcp::McpCodecLib::new(codec_id(12)))
        );
        $m!(
            "codec/algol",
            "Algol codec",
            "codec-algol",
            Some(crate::codec_algol::RECIPES),
            || Box::new(crate::codec_algol::AlgolCodecLib::new(codec_id(13)))
        );
        $m!(
            "codec/scheme-r7rs-small",
            "Scheme codec",
            "standard-scheme",
            Some(crate::lib_lang_scheme::RECIPES),
            || Box::new(crate::lib_lang_scheme::SchemeCodecLib::new(codec_id(14)))
        );
        $m!(
            "codec/common-lisp-lite",
            "Common Lisp codec",
            "standard-cl",
            Some(crate::lib_lang_cl::RECIPES),
            || Box::new(crate::lib_lang_cl::ClLiteReaderCodecLib::new(codec_id(15)))
        );
        $m!(
            "codec/clojure-edn",
            "Clojure EDN codec",
            "standard-clojure",
            Some(crate::lib_lang_clojure::RECIPES),
            || Box::new(crate::lib_lang_clojure::ClojureEdnCodecLib::new(codec_id(
                16
            )))
        );
        $m!(
            "codec/intent",
            "Intent codec",
            "intent",
            Some(crate::lib_intent::RECIPES),
            || Box::new(crate::lib_intent::IntentCodecLib::new(codec_id(17)))
        );
        $m!(
            "codec/scene",
            "Scene codec",
            "scene",
            Some(crate::lib_scene::RECIPES),
            || Box::new(crate::lib_scene::SceneCodecLib::new(codec_id(18)))
        );
    };
}
