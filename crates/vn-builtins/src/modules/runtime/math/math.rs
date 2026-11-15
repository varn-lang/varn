use vn_macros::natives;


natives! {
    abs(x) => x.abs(),
    sqrt(x) => x.sqrt(),
    sin(x) => x.sin(),
    cos(x) => x.cos(),
    tan(x) => x.tan(),
    floor(x) => x.floor(),
    ceil(x) => x.ceil(),
    round(x) => x.round(),
    random() => rand::random::<f64>(),
    acos(x) => x.acos(),
    asin(x) => x.asin(),
    atan(x) => x.atan(),
    atan2(y, x) => y.atan2(x),
    exp(x) => x.exp(),
    log(x) => x.ln(),
    log10(x) => x.log10(),
    log2(x) => x.log2(),
    pow(x, y) => x.powf(y),
    cbrt(x) => x.cbrt(),
    hypot(x, y) => x.hypot(y),
    max2(x, y) => vn_core::fmax(x, y),
    min2(x, y) => vn_core::fmin(x, y),
    sign(x) => vn_core::fsign(x),
}
