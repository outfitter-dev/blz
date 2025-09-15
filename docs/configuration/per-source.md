## Per-Source Settings (settings.toml)

Each source can have its own `settings.toml` under the cache directory:

```
<cache_root>/<alias>/settings.toml
```

Example:

```toml
[meta]
name = "react"
display_name = "React"
homepage = "https://react.dev"
repo = "https://github.com/facebook/react"

[fetch]
refresh_hours = 12           # override global default
follow_links = "first_party"
allowlist = ["react.dev", "github.com"]

[index]
max_heading_block_lines = 500
```

Notes:
- Only keys present here are overridden for this source.
- Missing keys inherit from the global config.
