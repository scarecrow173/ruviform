//! Consumer 側の Typed Port: Definition store と Workflow plan 検証。
//!
//! 混成 Workflow 参照は `ScriptArtifact` の `references` による
//! 型付き digest 参照のみで表現し、VM ポインタ・ハンドル・coroutine の
//! 直渡しは `ref_type` 語彙で拒否する（契約設計 C3'/Fail-Closed F-09〜F-12）。

use std::collections::{BTreeMap, BTreeSet};

use crate::canonical;
use crate::contract::ScriptBackendId;
use crate::value::{ArtifactKind, DefinitionDigest, ScriptArtifact, ScriptValue};

/// `references[]` の参照種別（PoC 規約）。`"definition_ref"` のみ受理。
const REF_TYPE_DEFINITION: &str = "definition_ref";

/// plan 構築の失敗（Fail-Closed 語彙に対応）。
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanError {
    /// entry artifact の kind が WorkflowDefinition でない。
    NotAWorkflow,
    /// payload の canonical digest が `artifact.digest` と不一致
    /// （artifact 改竄・stale digest — F-11）。
    DigestMismatch,
    /// `schema_version` が PoC の受理版と不一致。
    UnsupportedSchemaVersion { found: u32 },
    /// `ref_type` が `definition_ref` 以外（vm_reference/handle 等 — F-09）。
    DirectReferenceRejected { found: String },
    /// 参照先 digest が store に存在しない（F-10 unknown reference）。
    UnknownReference { digest: DefinitionDigest },
    /// 参照が要求する revision と store 上の revision が不一致（F-08 stale）。
    StaleRevision { digest: DefinitionDigest },
    /// 参照グラフに循環がある（F-12）。
    CycleDetected { digest: DefinitionDigest },
    /// 参照先 artifact の kind が参照側の要求と不一致。
    KindMismatch { expected: String, found: String },
    /// artifact 内部構造が不正（references が配列でない等）。
    InvalidStructure { detail: String },
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAWorkflow => write!(f, "entry artifact is not a workflow_definition"),
            Self::DigestMismatch => write!(f, "artifact digest does not match payload"),
            Self::UnsupportedSchemaVersion { found } => {
                write!(f, "unsupported artifact schema_version {found}")
            }
            Self::DirectReferenceRejected { found } => {
                write!(f, "direct/vm reference rejected: {found}")
            }
            Self::UnknownReference { digest } => write!(f, "unknown reference: {digest}"),
            Self::StaleRevision { digest } => write!(f, "stale revision for: {digest}"),
            Self::CycleDetected { digest } => write!(f, "reference cycle at: {digest}"),
            Self::KindMismatch { expected, found } => {
                write!(f, "kind mismatch: expected {expected}, found {found}")
            }
            Self::InvalidStructure { detail } => write!(f, "invalid artifact structure: {detail}"),
        }
    }
}

impl std::error::Error for PlanError {}

/// `references[]` の 1 要素（typed definition 参照）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionRef {
    pub digest: DefinitionDigest,
    pub kind: ArtifactKind,
    pub revision: u64,
}

/// 検証済みの 1 artifact（store から取得・digest 検証済み）。
#[derive(Clone, Debug)]
pub struct ResolvedArtifact {
    pub digest: DefinitionDigest,
    pub kind: ArtifactKind,
    pub source_revision: u64,
    pub backend_id: ScriptBackendId,
    pub references: Vec<DefinitionRef>,
}

/// 検証済み Workflow plan。`nodes` は digest 昇順のトポロジカル順
/// （循環がないことが検証済みなので、DFS 後順が決定的順序を与える）。
pub struct WorkflowPlan {
    pub entry: DefinitionDigest,
    pub nodes: Vec<ResolvedArtifact>,
}

/// Consumer 側の artifact 保管庫。digest を key にした内容アドレス
/// ストア。revision は artifact.source.source_revision から読む。
#[derive(Default)]
pub struct DefinitionStore {
    artifacts: BTreeMap<DefinitionDigest, ScriptArtifact>,
}

impl DefinitionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// artifact を登録する。digest 再計算で改竄を検出（F-11）。
    pub fn insert(&mut self, artifact: ScriptArtifact) -> Result<(), PlanError> {
        verify_artifact(&artifact)?;
        self.artifacts.insert(artifact.digest.clone(), artifact);
        Ok(())
    }

    pub fn get(&self, digest: &DefinitionDigest) -> Option<&ScriptArtifact> {
        self.artifacts.get(digest)
    }

    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

/// artifact の digest・schema・kind を検証する（store insert と plan
/// 構築の両方から使う単一の正本）。
pub fn verify_artifact(artifact: &ScriptArtifact) -> Result<(), PlanError> {
    if artifact.schema_version != 1 {
        return Err(PlanError::UnsupportedSchemaVersion {
            found: artifact.schema_version,
        });
    }
    let recomputed = canonical::definition_digest(&artifact.payload)
        .map_err(|e| PlanError::InvalidStructure { detail: e })?;
    if recomputed != artifact.digest {
        return Err(PlanError::DigestMismatch);
    }
    Ok(())
}

/// artifact payload の `kind` 文字列を `ArtifactKind` に写像する。
/// `references` を抽出する。
fn inspect_artifact(
    artifact: &ScriptArtifact,
) -> Result<(ArtifactKind, Vec<DefinitionRef>), PlanError> {
    let rec = artifact
        .payload
        .as_record()
        .ok_or_else(|| PlanError::InvalidStructure {
            detail: "payload is not a record".to_owned(),
        })?;
    let kind_str = match rec.get("kind") {
        Some(ScriptValue::String(s)) => s.clone(),
        _ => {
            return Err(PlanError::InvalidStructure {
                detail: "payload.kind missing or not a string".to_owned(),
            });
        }
    };
    let kind =
        ArtifactKind::from_script_str(&kind_str).ok_or_else(|| PlanError::InvalidStructure {
            detail: format!("unknown artifact kind {kind_str:?}"),
        })?;
    if kind != artifact.kind {
        return Err(PlanError::KindMismatch {
            expected: artifact.kind.as_script_str(),
            found: kind_str,
        });
    }
    let mut refs = Vec::new();
    if let Some(raw_refs) = rec.get("references") {
        let items = match raw_refs {
            ScriptValue::Array(a) => a,
            ScriptValue::Absent | ScriptValue::Null => &Vec::new(),
            _ => {
                return Err(PlanError::InvalidStructure {
                    detail: "payload.references is not an array".to_owned(),
                });
            }
        };
        for item in items {
            let r = item
                .as_record()
                .ok_or_else(|| PlanError::InvalidStructure {
                    detail: "reference entry is not a record".to_owned(),
                })?;
            let ref_type = match r.get("ref_type") {
                Some(ScriptValue::String(s)) => s.as_str(),
                _ => {
                    return Err(PlanError::InvalidStructure {
                        detail: "reference.ref_type missing".to_owned(),
                    });
                }
            };
            if ref_type != REF_TYPE_DEFINITION {
                return Err(PlanError::DirectReferenceRejected {
                    found: ref_type.to_owned(),
                });
            }
            let digest_s = match r.get("digest") {
                Some(ScriptValue::String(s)) => s.clone(),
                _ => {
                    return Err(PlanError::InvalidStructure {
                        detail: "reference.digest missing".to_owned(),
                    });
                }
            };
            let kind_s = match r.get("kind") {
                Some(ScriptValue::String(s)) => s.clone(),
                _ => {
                    return Err(PlanError::InvalidStructure {
                        detail: "reference.kind missing".to_owned(),
                    });
                }
            };
            let ref_kind = ArtifactKind::from_script_str(&kind_s).ok_or_else(|| {
                PlanError::InvalidStructure {
                    detail: format!("unknown reference kind {kind_s:?}"),
                }
            })?;
            let revision = match r.get("revision") {
                Some(ScriptValue::Integer(i)) => *i as u64,
                Some(ScriptValue::Number(f)) if f.fract() == 0.0 => *f as u64,
                _ => {
                    return Err(PlanError::InvalidStructure {
                        detail: "reference.revision missing or not integer".to_owned(),
                    });
                }
            };
            refs.push(DefinitionRef {
                digest: DefinitionDigest(digest_s),
                kind: ref_kind,
                revision,
            });
        }
    }
    Ok((kind, refs))
}

/// entry workflow artifact から検証済み plan を構築する。
/// 参照は推移的に解決し、未知参照・stale revision・循環・直接参照を
/// Fail-Closed で拒否する。
pub fn build_workflow_plan(
    entry: &ScriptArtifact,
    store: &DefinitionStore,
) -> Result<WorkflowPlan, PlanError> {
    verify_artifact(entry)?;
    let (entry_kind, _) = inspect_artifact(entry)?;
    if entry_kind != ArtifactKind::WorkflowDefinition {
        return Err(PlanError::NotAWorkflow);
    }

    let mut nodes: Vec<ResolvedArtifact> = Vec::new();
    let mut visiting: BTreeSet<DefinitionDigest> = BTreeSet::new();
    let mut visited: BTreeSet<DefinitionDigest> = BTreeSet::new();

    fn visit(
        digest: &DefinitionDigest,
        store: &DefinitionStore,
        nodes: &mut Vec<ResolvedArtifact>,
        visiting: &mut BTreeSet<DefinitionDigest>,
        visited: &mut BTreeSet<DefinitionDigest>,
    ) -> Result<(), PlanError> {
        if visited.contains(digest) {
            return Ok(());
        }
        if !visiting.insert(digest.clone()) {
            return Err(PlanError::CycleDetected {
                digest: digest.clone(),
            });
        }
        let artifact = store
            .get(digest)
            .ok_or_else(|| PlanError::UnknownReference {
                digest: digest.clone(),
            })?;
        verify_artifact(artifact)?;
        let (kind, refs) = inspect_artifact(artifact)?;
        for r in &refs {
            // kind 要求の照合は参照側が kind を明示するため、ここでは
            // store 上の artifact kind が参照要求と一致するかを検査する。
            let target = store
                .get(&r.digest)
                .ok_or_else(|| PlanError::UnknownReference {
                    digest: r.digest.clone(),
                })?;
            verify_artifact(target)?;
            let (target_kind, _) = inspect_artifact(target)?;
            if target_kind != r.kind {
                return Err(PlanError::KindMismatch {
                    expected: r.kind.as_script_str(),
                    found: target_kind.as_script_str(),
                });
            }
            if target.source.source_revision.0 != r.revision {
                return Err(PlanError::StaleRevision {
                    digest: r.digest.clone(),
                });
            }
            visit(&r.digest, store, nodes, visiting, visited)?;
        }
        visiting.remove(digest);
        visited.insert(digest.clone());
        nodes.push(ResolvedArtifact {
            digest: digest.clone(),
            kind,
            source_revision: artifact.source.source_revision.0,
            backend_id: artifact.produced_by.backend_id.clone(),
            references: refs,
        });
        Ok(())
    }

    // entry artifact も store に登録済みであること（consumer は生成物を
    // すべて DefinitionStore に置く前提）。visit が entry を含めて
    // トポロジカル順に nodes を構築する。
    visit(
        &entry.digest.clone(),
        store,
        &mut nodes,
        &mut visiting,
        &mut visited,
    )?;
    Ok(WorkflowPlan {
        entry: entry.digest.clone(),
        nodes,
    })
}
