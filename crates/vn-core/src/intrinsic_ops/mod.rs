pub mod math;
mod str_probe;
pub use str_probe::StrProbe;

pub fn intrinsic_lookup(module: &str, name: &str) -> Option<u8> {
    (module == math::MathOp::MODULE)
        .then(|| math::MathOp::of(name))
        .flatten()
        .map(math::MathOp::wire)
}
