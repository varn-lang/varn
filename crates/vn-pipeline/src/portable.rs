use crate::PipelineError;
use vn_modules::artifact::{
    read_artifact, write_artifact, write_artifact_file, ArtifactClass, ArtifactKind,
};
use vn_types::ModuleGraphArtifact;

pub fn write_portable(path: &str, artifact: &ModuleGraphArtifact) -> Result<(), PipelineError> {
    let payload = postcard::to_allocvec(artifact)
        .map_err(|e| PipelineError::internal(format!("cannot serialize .vnc: {e}")))?;

    let out = write_artifact(ArtifactKind::ModuleGraph, ArtifactClass::Portable, &payload);

    write_artifact_file(std::path::Path::new(path), &out)
        .map_err(|e| PipelineError::io(format!("cannot write '{}': {e}", path)))
}

pub fn read_portable(path: &str) -> Result<ModuleGraphArtifact, PipelineError> {
    let bytes = std::fs::read(path)
        .map_err(|e| PipelineError::io(format!("cannot read '{}': {e}", path)))?;

    let payload = read_artifact(ArtifactKind::ModuleGraph, &bytes)
        .map_err(|e| PipelineError::internal(format!("'{}' no se puede ejecutar: {}", path, e)))?;

    postcard::from_bytes(payload)
        .map_err(|e| PipelineError::internal(format!("cannot deserialize '{}': {e}", path)))
}

pub fn is_portable(path: &str) -> bool {
    path.ends_with(".vnc")
}
