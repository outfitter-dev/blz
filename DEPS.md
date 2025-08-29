# Core Dependencies

- tantivy: search engine powering indexing/search
- tree-sitter + tree-sitter-md: markdown parsing with precise ranges
- tokio: async runtime for I/O-bound tasks (features = ["full"])
- reqwest: HTTP client (default-features = false; features = ["rustls-tls", "gzip", "brotli", "json", "stream", "http2"])
- criterion: micro-benchmarking and regression tracking
- pprof: CPU profiling and flamegraph generation (features = ["flamegraph", "protobuf-codec"])
- sysinfo: lightweight process/machine metrics

## Links
- [Tantivy](https://github.com/quickwit-oss/tantivy)
- [Tree-sitter](https://tree-sitter.github.io/)
- [tree-sitter-markdown](https://github.com/MDeiml/tree-sitter-markdown)
- [Tokio](https://tokio.rs/)
- [Reqwest](https://docs.rs/reqwest/)
- [Criterion](https://bheisler.github.io/criterion.rs/book/)
- [pprof](https://docs.rs/pprof/)
- [Sysinfo](https://docs.rs/sysinfo/)

