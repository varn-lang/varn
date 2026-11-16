#[derive(Debug, Clone)]
pub(super) struct BenchDef {
    pub(super) name: &'static str,
    pub(super) python: bool,
}

impl BenchDef {
    const fn new(name: &'static str, python: bool) -> Self {
        Self { name, python }
    }

    pub(super) fn vn_file(&self) -> String {
        format!("vn/bench_{}.vn", self.name)
    }

    pub(super) fn ts_file(&self) -> String {
        format!("ts/bench_{}.ts", self.name)
    }

    pub(super) fn py_file(&self) -> Option<String> {
        self.python.then(|| format!("py/{}.py", self.name))
    }
}

pub(super) const ALL_BENCHMARKS: &[BenchDef] = &[
    BenchDef::new("fib", true),
    BenchDef::new("gc_alloc", true),
    BenchDef::new("dto", false),
    BenchDef::new("matrix", true),
    BenchDef::new("str_ops", false),
    BenchDef::new("json_native", false),
    BenchDef::new("json_pure", false),
    BenchDef::new("csv_pipeline", false),
    BenchDef::new("collection_pipeline", false),
    BenchDef::new("http_routing", false),
    BenchDef::new("csv_etl", false),
    BenchDef::new("class_fields", false),
    BenchDef::new("new_retained", false),
    BenchDef::new("json_api_payloads", false),
    BenchDef::new("generator", true),
];

pub(super) fn select_benchmarks(
    only: &Option<Vec<String>>,
) -> Result<Vec<&'static BenchDef>, String> {
    let benchmarks: Vec<&BenchDef> = if let Some(only_list) = only {
        let filtered: Vec<&BenchDef> = ALL_BENCHMARKS
            .iter()
            .filter(|b| only_list.iter().any(|o| o.eq_ignore_ascii_case(b.name)))
            .collect();
        if filtered.is_empty() {
            eprintln!("error: no benchmark matched: {}", only_list.join(", "));
            std::process::exit(1);
        }
        filtered
    } else {
        ALL_BENCHMARKS.iter().collect()
    };
    Ok(benchmarks)
}
