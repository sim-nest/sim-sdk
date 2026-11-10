// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Descriptive factory evidence the child session checks.
//!
//! A host implements these traits over its retained factory. This crate
//! records the observed composition. It does not load a factory.

use sim_kernel::{ContentId, Datum};
use sim_run_core::CliError;

/// One owned projection object the child can reobserve.
pub trait ProjectionOwnedEvidence {
    /// Canonical value of this object.
    ///
    /// # Errors
    ///
    /// Returns an error when the object is not current.
    fn canonical(&self) -> Result<Datum, CliError>;

    /// Confirms this object is still the observed owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the object is no longer current.
    fn reobserve_current(&self) -> Result<(), CliError>;

    /// Runs `action` only while this object stays current.
    ///
    /// # Errors
    ///
    /// Returns an error when the object is not current.
    fn while_current(&self, action: &mut dyn FnMut()) -> Result<(), CliError>;
}

/// Configuration Shape evidence for one loaded factory.
pub trait ProjectionConfigurationShapeEvidence: ProjectionOwnedEvidence {
    /// Confirms `configuration` is the Shape this object still describes.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration was substituted.
    fn verify_configuration(&self, configuration: &Datum) -> Result<(), CliError>;
}

/// Collector for one complete child-factory observation.
pub trait ProjectionFactoryEvidenceVisitor {
    /// Records the configuration Shape.
    ///
    /// # Errors
    ///
    /// Returns an error when the Shape is duplicated or not current.
    fn configuration_shape(
        &mut self,
        configuration: &Datum,
        shape: &dyn ProjectionConfigurationShapeEvidence,
    ) -> Result<(), CliError>;

    /// Records one provider owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider is duplicated or not current.
    fn provider_registration(
        &mut self,
        provider: &dyn ProjectionOwnedEvidence,
    ) -> Result<(), CliError>;

    /// Records one selected policy owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the policy is duplicated or not current.
    fn selected_policy(&mut self, policy: &dyn ProjectionOwnedEvidence) -> Result<(), CliError>;

    /// Records the baseline membership owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the baseline is duplicated or not current.
    fn baseline_membership(
        &mut self,
        membership: &dyn ProjectionOwnedEvidence,
    ) -> Result<(), CliError>;

    /// Records one graph owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the graph owner is duplicated or not current.
    fn graph_owner(&mut self, owner: &dyn ProjectionOwnedEvidence) -> Result<(), CliError>;

    /// Records the federated closure owner.
    ///
    /// # Errors
    ///
    /// Returns an error when the closure is duplicated or not current.
    fn federated_closure(&mut self, closure: &dyn ProjectionOwnedEvidence) -> Result<(), CliError>;

    /// Finishes the observation.
    ///
    /// # Errors
    ///
    /// Returns an error when the collected composition is incomplete or changed.
    fn complete(&mut self) -> Result<(), CliError>;
}

/// Retained child factory a host exposes to the admission session.
pub trait ProjectionFactoryCapture {
    /// Capture identity of this retained factory.
    fn projection_capture(&self) -> &ContentId;

    /// Visits the factory's current evidence once.
    ///
    /// # Errors
    ///
    /// Returns an error when the evidence is incomplete or not current.
    fn visit_projection_evidence<V: ProjectionFactoryEvidenceVisitor + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<(), CliError>;

    /// Visits the evidence and runs `action` only while that evidence stays current.
    ///
    /// # Errors
    ///
    /// Returns an error when the evidence changes or is not current.
    fn with_projection_evidence_current<T, V: ProjectionFactoryEvidenceVisitor + ?Sized>(
        &self,
        visitor: &mut V,
        action: impl FnOnce() -> T,
    ) -> Result<T, CliError>;
}
