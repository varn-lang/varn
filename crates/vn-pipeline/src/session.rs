use std::sync::Arc;
use vn_modules::project::{Anchor, Project};

use crate::{ModuleResolver, PipelineError};

pub struct Session {
    project: Project,
    resolver: Arc<ModuleResolver>,
}

impl Session {
    pub fn open(anchor: Anchor<'_>) -> Result<Session, PipelineError> {
        let project = Project::discover(anchor).map_err(PipelineError::Project)?;
        Ok(Session::of(project))
    }

    pub fn of(project: Project) -> Session {
        Session {
            project,
            resolver: Arc::new(ModuleResolver::new()),
        }
    }

    pub fn detached() -> Session {
        Session {
            project: Project::detached(),
            resolver: Arc::new(ModuleResolver::new()),
        }
    }

    pub fn isolated(&self) -> Session {
        Session {
            project: self.project.clone(),
            resolver: Arc::new(ModuleResolver::new()),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn resolver(&self) -> &ModuleResolver {
        &self.resolver
    }

    pub fn shared_resolver(&self) -> Arc<ModuleResolver> {
        Arc::clone(&self.resolver)
    }
}
