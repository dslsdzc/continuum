use continuum_artifact::ArtifactId;
use continuum_memory::MemoryClaim;
use continuum_trust::{ContextProvenance, ContextTrust};

use crate::{ContextPackage, Reference};

impl ContextPackage {
    pub fn instructions(&self) -> &[String] {
        &self.instructions
    }

    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    pub fn artifacts(&self) -> &[ArtifactId] {
        &self.artifacts
    }

    pub fn memories(&self) -> &[MemoryClaim] {
        &self.memories
    }

    pub fn provenance(&self) -> &[ContextProvenance] {
        &self.provenance
    }

    pub fn trust_labels(&self) -> &[ContextTrust] {
        &self.trust_labels
    }
}
