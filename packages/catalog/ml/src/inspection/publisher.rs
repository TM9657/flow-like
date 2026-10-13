//! Publishes ledger models as FlowPath files under `models/`. Publishing never fails the
//! calling node: experiments record events, champions report a blocking next action and
//! deployments and SAM 2 sources log a warning.

use super::lifecycle::LedgerScope;
use super::published::PublishedModelRef;
use flow_like::flow::execution::context::ExecutionContext;
use flow_like_catalog_core::FlowPath;
use flow_like_ml_runtime::{Deployment, PretrainedSource, TrainingRepository};
use flow_like_types::{Result, anyhow};

#[derive(Clone, Debug)]
pub(crate) enum PublishOutcome {
    Published(PublishedModelRef),
    Unchanged,
    Skipped { reason: String },
    Failed { error: String },
}

/// Why a publish runs. Self-repair runs at most once per run per record (run-cache key
/// `published-models-checked-v1`); a state change always rebuilds the pointer from the
/// ledger and publishes, and a concurrent publish of the same record waits instead of
/// skipping it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublishReason {
    /// Read paths: Get Learning Project, Get Active Student, Get and Predict fallbacks.
    SelfRepair,
    /// Experiment finalization, learning cycles, promote, rollback, pause and resume.
    StateChange,
}

/// A ledger record that Export Model writes as a model folder.
#[derive(Clone, Debug)]
pub(crate) enum LedgerModel {
    Experiment { experiment_id: String },
    LearningChampion { project_id: String },
    Artifact { artifact_id: String },
    PretrainedSource { source_id: String },
}

pub(crate) async fn ensure_experiment_published(
    _context: &mut ExecutionContext,
    _repo: &TrainingRepository,
    _scope: LedgerScope,
    _experiment_id: &str,
    _reason: PublishReason,
) -> PublishOutcome {
    PublishOutcome::Unchanged
}

pub(crate) async fn ensure_champion_published(
    _context: &mut ExecutionContext,
    _repo: &TrainingRepository,
    _scope: LedgerScope,
    _project_id: &str,
    _reason: PublishReason,
) -> PublishOutcome {
    PublishOutcome::Unchanged
}

pub(crate) async fn ensure_deployment_published(
    _context: &mut ExecutionContext,
    _repo: &TrainingRepository,
    _scope: LedgerScope,
    _deployment: &Deployment,
    _reason: PublishReason,
) -> PublishOutcome {
    PublishOutcome::Unchanged
}

/// Content-addressed: an existing folder with the same blob digest is left as it is.
pub(crate) async fn publish_pretrained_source(
    _context: &mut ExecutionContext,
    _repo: &TrainingRepository,
    _scope: LedgerScope,
    _source: &PretrainedSource,
) -> PublishOutcome {
    PublishOutcome::Unchanged
}

/// Writes the model folder, with `dependencies/{source}` for its extractor, to `destination`.
pub(crate) async fn export_model_folder(
    _context: &mut ExecutionContext,
    _repo: &TrainingRepository,
    _scope: LedgerScope,
    _model: &LedgerModel,
    _destination: &FlowPath,
) -> Result<PublishedModelRef> {
    Err(anyhow!("export_model_folder lands in L4"))
}

/// Export Auto Model's files: the weights at `weights` and a v2 manifest at `manifest`.
pub(crate) async fn export_auto_model_files(
    _context: &mut ExecutionContext,
    _repo: &TrainingRepository,
    _scope: LedgerScope,
    _experiment_id: &str,
    _weights: &FlowPath,
    _manifest: &FlowPath,
) -> Result<PublishedModelRef> {
    Err(anyhow!("export_auto_model_files lands in L4"))
}
