use std::rc::Rc;
use std::sync::{Arc, Mutex};

use rustc_hash::FxHashMap;
use vn_core::ModuleId;
use vn_modules::loader::ModuleLoader as CanonicalLoader;
use vn_pipeline::stdlib_image::compile_source;
use vn_pipeline::ModuleResolver;
use vn_types::FunctionProto;
use vn_vm::loader::{ModuleError, ModuleLoader};

pub struct PipelineLoader {
    registry: vn_modules::loader::ModuleRegistry,
    protos: Mutex<ProtoBytesCache>,
    session: Arc<ModuleResolver>,
}

type ProtoBytesCache = FxHashMap<String, (u64, Arc<[u8]>)>;

impl PipelineLoader {
    pub fn new(session: Arc<ModuleResolver>) -> Self {
        Self {
            registry: vn_modules::loader::default_registry(),
            protos: Mutex::new(FxHashMap::default()),
            session,
        }
    }

    fn cached_proto(&self, key: &str, fingerprint: u64) -> Option<Rc<FunctionProto>> {
        let bytes = {
            let guard = self.protos.lock().ok()?;
            let (fp, bytes) = guard.get(key)?;
            if *fp != fingerprint {
                return None;
            }
            bytes.clone()
        };
        postcard::from_bytes::<FunctionProto>(&bytes)
            .ok()
            .map(Rc::new)
    }

    fn store_proto(&self, key: &str, fingerprint: u64, proto: &Rc<FunctionProto>) {
        if let Ok(bytes) = postcard::to_allocvec(proto.as_ref())
            && let Ok(mut guard) = self.protos.lock()
        {
            guard.insert(
                key.to_owned(),
                (fingerprint, Arc::from(bytes.into_boxed_slice())),
            );
        }
    }
}

impl ModuleLoader for PipelineLoader {
    fn resolve(&self, specifier: &str, from: &ModuleId) -> Result<ModuleId, ModuleError> {
        self.registry
            .resolve(specifier, from)
            .map_err(|e| ModuleError::new(e.to_string()))
    }

    fn load(&self, id: &ModuleId) -> Result<Option<Rc<FunctionProto>>, ModuleError> {
        match id {
            ModuleId::Local(_) | ModuleId::Std(_) | ModuleId::Core(_) => {}
            ModuleId::Runtime(_) | ModuleId::Package { .. } => return Ok(None),
        }
        let source = CanonicalLoader::source(&self.registry, id)
            .map_err(|e| ModuleError::new(e.to_string()))?;
        if let Some(blob) = source.bytecode.as_ref() {
            return postcard::from_bytes(blob)
                .map(Rc::new)
                .map(Some)
                .map_err(|e| ModuleError::new(format!("corrupt bytecode for {id:?}: {e}")));
        }
        let text = source.text;
        let fingerprint = vn_core::fingerprint::fingerprint(text.as_bytes());
        let key = vn_modules::artifact::module_key(id, fingerprint);
        if let Some(hit) = self.cached_proto(&key, fingerprint) {
            return Ok(Some(hit));
        }
        let path = match id {
            ModuleId::Local(p) => p.as_ref(),
            ModuleId::Std(s) | ModuleId::Core(s) => s.as_ref(),
            ModuleId::Runtime(_) | ModuleId::Package { .. } => return Ok(None),
        };
        let proto = compile_source(text.as_ref(), path, &self.session)
            .map(Rc::new)
            .map_err(|e| ModuleError::new(format!("compile error in '{path}': {e}")))?;
        self.store_proto(&key, fingerprint, &proto);
        Ok(Some(proto))
    }
}
