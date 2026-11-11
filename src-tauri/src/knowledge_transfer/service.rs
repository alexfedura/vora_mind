use super::*;
use anyhow::Result;
use rusqlite::Connection;

pub fn workspace(conn: &Connection) -> Result<TransferWorkspaceDto> {
    repository::workspace(conn)
}

pub fn provenance_preview(
    conn: &Connection,
    origin_id: &str,
) -> Result<TransferProvenancePreviewDto> {
    repository::provenance_preview(conn, origin_id)
}

pub fn create_concept_version(
    conn: &mut Connection,
    request: &CreateConceptVersionReq,
) -> Result<String> {
    repository::create_concept_version(conn, request)
}

pub fn create_principle_version(
    conn: &mut Connection,
    request: &CreatePrincipleVersionReq,
) -> Result<String> {
    repository::create_principle_version(conn, request)
}

pub fn review_knowledge_version(
    conn: &mut Connection,
    request: &ReviewKnowledgeVersionReq,
) -> Result<()> {
    repository::review_knowledge_version(conn, request)
}

pub fn create_origin(
    conn: &mut Connection,
    request: &CreateTransferOriginReq,
) -> Result<KnowledgeTransferOriginDto> {
    repository::create_origin(conn, request)
}

pub fn create_eligibility(
    conn: &mut Connection,
    request: &CreateTransferEligibilityReq,
) -> Result<TransferEligibilityDto> {
    repository::create_eligibility(conn, request)
}

pub fn change_eligibility(
    conn: &mut Connection,
    request: &ChangeTransferEligibilityReq,
) -> Result<TransferEligibilityDto> {
    repository::change_eligibility(conn, request)
}

pub fn adopt_knowledge(
    conn: &mut Connection,
    request: &AdoptKnowledgeReq,
) -> Result<KnowledgeTransferDto> {
    repository::adopt_knowledge(conn, request)
}

pub fn create_adaptation(
    conn: &mut Connection,
    request: &CreateTransferAdaptationReq,
) -> Result<TransferAdaptationDto> {
    repository::create_adaptation(conn, request)
}

pub fn record_downstream_usage(
    conn: &mut Connection,
    request: &RecordDownstreamUsageReq,
) -> Result<DownstreamUsageDto> {
    repository::record_downstream_usage(conn, request)
}
