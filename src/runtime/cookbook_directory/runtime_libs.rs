macro_rules! cookbook_directory_runtime_libs {
    ($m:ident) => {
        $m!(
            "discrete",
            "Discrete math",
            "discrete-runtime",
            Some(sim_lib_discrete::RECIPES),
            || Box::new(sim_lib_discrete::DiscreteLib)
        );
        $m!(
            "organ/binding",
            "Binding organ",
            "standard-binding",
            Some(crate::lib_binding::RECIPES),
            || Box::new(crate::lib_binding::BindingLib)
        );
        $m!(
            "organ/control",
            "Control organ",
            "control",
            Some(crate::lib_control::RECIPES),
            || Box::new(crate::lib_control::ControlLib)
        );
        $m!(
            "organ/sequence",
            "Sequence organ",
            "standard-sequence",
            Some(crate::lib_sequence::RECIPES),
            || Box::new(crate::lib_sequence::SequenceLib)
        );
        $m!(
            "organ/pattern",
            "Pattern organ",
            "standard-pattern",
            Some(crate::lib_pattern::RECIPES),
            || Box::new(crate::lib_pattern::PatternLib)
        );
        $m!(
            "logic",
            "Logic runtime",
            "logic-core",
            Some(crate::lib_logic::RECIPES),
            || Box::new(crate::lib_logic::LogicLib)
        );
        $m!(
            "rank",
            "Rank runtime",
            "rank",
            Some(crate::lib_rank::RECIPES),
            || Box::new(crate::lib_rank::RankLib)
        );

        $m!(
            "agent",
            "Agent runtime",
            "agent",
            Some(crate::lib_agent::RECIPES),
            || Box::new(crate::lib_agent::AgentLib)
        );
        $m!(
            "bridge",
            "Bridge runtime",
            "bridge",
            Some(crate::lib_bridge::RECIPES),
            || Box::new(crate::lib_bridge::BridgeLib)
        );
        $m!(
            "mcp",
            "MCP runtime",
            "mcp",
            Some(crate::lib_mcp::RECIPES),
            || Box::new(crate::lib_mcp::McpLib)
        );
        $m!(
            "openai-gateway",
            "OpenAI gateway",
            "openai-server",
            Some(crate::lib_openai_server::RECIPES),
            || Box::new(crate::lib_openai_server::OpenAiGatewayLib)
        );
        $m!(
            "server",
            "Server runtime",
            "server",
            Some(crate::lib_server::RECIPES),
            || Box::new(crate::lib_server::ServerLib)
        );
        $m!(
            "skill",
            "Skill runtime",
            "skill",
            Some(crate::lib_skill::RECIPES),
            || Box::new(crate::lib_skill::SkillLib)
        );
    };
}
