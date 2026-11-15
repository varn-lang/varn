use vn_core::StdErrorClass;
use vn_macros::natives;
use vn_types::NativeError;
use vn_types::marshal::VnArray;
use vn_types::{NativeCtx, VmValue};


fn read_host(ctx: &dyn NativeCtx, arr: VnArray) -> Result<Vec<f32>, String> {
    let len = arr.len(ctx);
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let v = arr
            .get(ctx, i)
            .ok_or_else(|| "gpu.upload: index out of bounds".to_string())?;
        if !(v.is_f64() || v.is_int()) {
            return Err(format!("gpu.upload: expected float[], got {v:?}"));
        }
        out.push(v.to_f64() as f32);
    }
    Ok(out)
}

fn read_dims(ctx: &dyn NativeCtx, arr: VnArray) -> Result<Vec<usize>, String> {
    let len = arr.len(ctx);
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let v = arr
            .get(ctx, i)
            .ok_or_else(|| "gpu.reshape: index out of bounds".to_string())?;
        if !v.is_int() || v.as_int() < 0 {
            return Err(format!("gpu.reshape: expected int[] >= 0, got {v:?}"));
        }
        out.push(v.as_int() as usize);
    }
    Ok(out)
}

fn elementwise(op: vn_gpu::Op, a: i64, b: i64) -> Result<i64, String> {
    let pool = vn_gpu::pool();
    let g = pool.lock();
    let (la, lb) = (g.len(a)?, g.len(b)?);
    let (da, db) = (g.dims(a)?, g.dims(b)?);
    if la != lb || da != db {
        return Err(format!("gpu: shape mismatch ({da:?} vs {db:?})"));
    }
    drop(g);
    vn_gpu::pool().lock().lazy(op, vec![a, b], la, da)
}

natives! {
    backend() => vn_gpu::backend_name().to_string(),
    deviceCount() => vn_gpu::device_count(),
    adapterName(i) => {
        if i < 0 {
            return Err(NativeError::range_error("gpu.adapterName: index must be non-negative".to_string()));
        }
        (vn_gpu::adapter_name(i as usize)
            .map(|s| s.to_string())
            .ok_or_else(|| format!("gpu.adapterName: no adapter at index {i}"))).map_err(|e| NativeError::std(StdErrorClass::Std, e))
    },
    engine() => vn_gpu::last_engine().to_string(),
    create(len) => {
        if len < 0 {
            return Err(NativeError::range_error("gpu.create: len must be non-negative".to_string()));
        }
        Ok(vn_gpu::pool().lock().create(len as usize))
    },
    destroy(id) => (vn_gpu::pool().lock().destroy(id)).map_err(|e| NativeError::std(StdErrorClass::Std, e)),
    len(id) => vn_gpu::pool().lock().len(id).map_err(|e| NativeError::std(StdErrorClass::Std, e))? as i64,
    dims(ctx, id) => {
        let _ = ctx;
        Ok(vn_gpu::pool().lock().dims(id).map_err(|e| NativeError::std(StdErrorClass::Std, e))?
            .into_iter()
            .map(|d| VmValue::from_int(d as i64))
            .collect())
    },
    reshape(ctx, id, dims) => {
        let dims = read_dims(ctx, dims).map_err(|e| NativeError::std(StdErrorClass::Std, e))?;
        (vn_gpu::pool().lock().reshape(id, dims)).map_err(|e| NativeError::std(StdErrorClass::Std, e))
    },
    upload(ctx, data) => {
        let vals = read_host(ctx, data).map_err(|e| NativeError::std(StdErrorClass::Std, e))?;
        Ok(vn_gpu::pool().lock().upload(vals))
    },
    download(ctx, id) => {
        let _ = ctx;
        let vals = vn_gpu::pool().lock().data(id).map_err(|e| NativeError::std(StdErrorClass::Std, e))?;
        Ok(vals.into_iter().map(|x| VmValue::from_f64(x as f64)).collect())
    },
    add(a, b) => (elementwise(vn_gpu::Op::Add, a, b)).map_err(|e| NativeError::std(StdErrorClass::Std, e)),
    mul(a, b) => (elementwise(vn_gpu::Op::Mul, a, b)).map_err(|e| NativeError::std(StdErrorClass::Std, e)),
    scale(a, s) => {
        let pool = vn_gpu::pool();
        let (len, dims) = {
            let g = pool.lock();
            (g.len(a).map_err(|e| NativeError::std(StdErrorClass::Std, e))?, g.dims(a).map_err(|e| NativeError::std(StdErrorClass::Std, e))?)
        };
        (vn_gpu::pool().lock().lazy(
            vn_gpu::Op::Scale { factor: s as f32 },
            vec![a],
            len,
            dims,
        )).map_err(|e| NativeError::std(StdErrorClass::Std, e))
    },
    dot(a, b) => {
        let m = elementwise(vn_gpu::Op::Mul, a, b).map_err(|e| NativeError::std(StdErrorClass::Std, e))?;
        let vals = vn_gpu::pool().lock().data(m).map_err(|e| NativeError::std(StdErrorClass::Std, e))?;
        let acc: f64 = vals.iter().map(|x| *x as f64).sum();
        vn_gpu::pool().lock().destroy(m).ok();
        Ok(acc)
    },
    matMul(a, b) => {
        let pool = vn_gpu::pool();
        let (da, db) = {
            let g = pool.lock();
            (g.dims(a).map_err(|e| NativeError::std(StdErrorClass::Std, e))?, g.dims(b).map_err(|e| NativeError::std(StdErrorClass::Std, e))?)
        };
        if da.len() != 2 || db.len() != 2 || da[1] != db[0] {
            return Err(NativeError::range_error(format!("gpu.matMul: shape mismatch ({da:?} vs {db:?})")));
        }
        let (m, k, n) = (da[0], da[1], db[1]);
        (vn_gpu::pool().lock().lazy(
            vn_gpu::Op::MatMul { m, k, n },
            vec![a, b],
            m * n,
            vec![m, n],
        )).map_err(|e| NativeError::std(StdErrorClass::Std, e))
    },
}
