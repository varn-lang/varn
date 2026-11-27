use crate::document::DocumentState;

#[cfg(feature = "machine-view")]
use super::emit_tir;

#[cfg(feature = "machine-view")]
enum MachineView {
    Assembly { with_wrapper: bool },
    Clif,
}

#[cfg(feature = "machine-view")]
pub(super) fn command(
    command: &str,
    arguments: &[serde_json::Value],
    state: &DocumentState,
) -> Result<String, String> {
    let view = match command {
        "vn.showClif" => MachineView::Clif,
        _ => MachineView::Assembly {
            with_wrapper: arguments.get(1).and_then(|v| v.as_bool()).unwrap_or(false),
        },
    };
    compile_machine_code(state, view)
}

#[cfg(feature = "machine-view")]
fn compile_machine_code(state: &DocumentState, view: MachineView) -> Result<String, String> {
    let proto =
        vn_compiler::from_tir::compile_module(&emit_tir(state)?, Vec::new(), &state.source, false)
            .0
            .map_err(|e| format!("Compilation failed: {e:?}"))?;
    let settings = vn_vm::ExecSettings::default();
    let helpers = vn_vm::jit::table_build::build_jit_helpers(&settings);
    let policy = vn_debug::machine::TierPolicy {
        calls_with_loops: settings.tier_loop,
        calls_without_loops: settings.tier_straight,
    };
    let functions = vn_debug::machine::collect(&proto, &helpers, policy);
    if functions.is_empty() {
        return Err("the host has no Cranelift target".to_string());
    }
    Ok(match view {
        MachineView::Assembly { with_wrapper } => {
            vn_debug::machine::render_machine_code(&functions, with_wrapper)
        }
        MachineView::Clif => vn_debug::machine::render_clif(&functions),
    })
}

#[cfg(not(feature = "machine-view"))]
pub(super) fn command(
    _command: &str,
    _arguments: &[serde_json::Value],
    _state: &DocumentState,
) -> Result<String, String> {
    Err("machine code views need vn-lsp built with the `machine-view` feature".to_string())
}
