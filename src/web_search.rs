/// Neutral HTTP request policy and injectable connector contracts.
pub mod http {
    pub use sim_lib_net_http::{
        Cancellation, Client, Connector, Error, Header, Method, Policy, ProxyPolicy,
        RedirectPolicy, Request, RequestBody, Response, TlsRoots, Url,
    };
}
/// Immutable raw and normalized web evidence records.
pub mod web {
    pub use sim_lib_web_core::{
        DecodeLimits, EvidenceSelector, PolicyDecision, PolicyKind, PolicyReceipt, PolicyVerdict,
        RepresentationMetadata, WebCapture, WebExchange, WebRepresentation,
    };
}
/// Provider-neutral query, observation, citation, and wire contracts.
pub mod records {
    pub use sim_lib_search_core::{
        AliasEvidence, Citation, ProviderClaim, RankContribution, ResearchBundle, SearchNotice,
        SearchObservation, SearchPage, SearchQuery, SearchRun, SearchSite, SearchWireCodec,
    };
}
/// Bounded provider transport host contracts and receipts.
pub mod search_host {
    pub use sim_lib_search_http::{
        CallMode, HttpRequest, HttpResponse, HttpSearchTransport, PrincipalRef, RawResponseCapture,
        SearchHttpClient, SearchHttpError, SearchHttpNotice, SearchHttpReceipt, SearchSiteConfig,
        SecretResolver, SiteLimits,
    };
}
/// Independently authorized fetch host, plans, and receipts.
pub mod fetch_host {
    pub use sim_lib_web_fetch::{
        CaptureDir, EgressPolicy, ExchangeReceipt, FetchError, FetchMode, FetchPlan, FetchReceipt,
        HttpExecutor, MemoryCaptureDir, PolicyReceipt, PublicWebEgress, RepresentationOutcome,
        RobotsReceipt, StoredCapture, StoredRobots, WebFetcher,
    };
}
/// Deterministic federation, ranking, inspection, and replay records.
pub mod research {
    pub use sim_lib_search::{
        AliasCluster, AliasRule, CapturedPage, Judge, JudgeReceipt, JudgeRequest, PageCapturer,
        PlanLimits, PlanOmission, PlanReceipt, PlannedSite, ResearchBundle, RetrieverSite,
        SearchCancellation, SearchFailure, SearchPlan, SearchRun, SiteOutcome, TypedOmission,
        call_judge, cluster_aliases, dispatch, fenced_capture, fenced_claim, fuse, inspect,
        local_corpus_page, plan_search, query, research,
    };
}
/// Office evidence anchors derived only from checked web representations.
pub mod office {
    pub use sim_lib_doc_web::{
        AnchorInput, AnchorKind, CitationFormat, EvidenceAnchor, WebEvidenceError, load_anchor,
        project_document, save_anchor, save_capture, save_representation,
    };
}
/// Inert, offline audit view over canonical records.
pub mod audit_view {
    pub use sim_lib_view_search::{
        AuditError, AuditRecords, CaptureEvidence, Layout, SEARCH_AUDIT_SURFACE_ID, SearchAction,
        ViewState, apply_action, render,
    };
}
