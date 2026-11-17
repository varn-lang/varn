use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;
use vn_types::{VmValue};


vn_macros::vn_record! {
    contract: "src/modules/runtime/text/text_runtime.vn",
    class: "ShapedRun",
    pub struct ShapedRun;
}

natives! {
    class ShapedRun {}
    fontCount() => vn_text::fonts().len() as i64,
    fonts() => vn_text::fonts(),
    shape(ctx, text, size) => {
        let run = vn_text::shape_run(text, size as f32).map_err(|e| NativeError::std(StdErrorClass::Parse, e))?;
        let chars: Vec<VmValue> = run
            .glyphs
            .iter()
            .map(|g| VmValue::from_int(g.ch as i64))
            .collect();
        let adv: Vec<VmValue> = run
            .glyphs
            .iter()
            .map(|g| VmValue::from_f64(g.advance as f64))
            .collect();
        let clusters: Vec<VmValue> = run
            .glyphs
            .iter()
            .map(|g| VmValue::from_int(g.cluster as i64))
            .collect();
        let glyphs = ctx.alloc_array(chars);
        let advances = ctx.alloc_array(adv);
        let clusters = ctx.alloc_array(clusters);
        Ok(ShapedRun {
            glyphs,
            advances,
            clusters,
            width: run.width as f64,
            count: run.glyphs.len() as i64,
        }
        .alloc(ctx)
        .map_err(NativeError::failure)?)
    },
    ascent(size) => vn_text::font_ascent(size as f32).map_err(|e| NativeError::std(StdErrorClass::Parse, e))? as f64,
    descent(size) => vn_text::font_descent(size as f32).map_err(|e| NativeError::std(StdErrorClass::Parse, e))? as f64,
    wrap(text, size, max) => {
        vn_text::wrap_lines(text, size as f32, max as f32).map_err(|e| NativeError::std(StdErrorClass::Parse, e))
    },
}
