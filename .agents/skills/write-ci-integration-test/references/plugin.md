# integration-plugin

Closes `S-PLUGIN-001`. Builds a minimal example plugin and asserts the host can load + execute it.

## Fixture

A new `examples/plugins/hello-effect/` Rust crate that compiles to `wasm32-wasip2` Component Model:

```toml
[package]
name = "hello-effect"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["cdylib"]
[dependencies]
wit-bindgen = "0.30"
```

Exports a single function the host can call to verify round-trip.

## Job body

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-wasip2
- name: Build example plugin
  run: cargo build -p hello-effect --target wasm32-wasip2 --release
- name: Run plugin host integration
  run: cargo test -p slop-plugin --test load_and_call -- --nocapture
```

## Test

The test should:
1. Construct a `PluginHost`.
2. Build a `PluginManifest` with `abi_version = PluginManifest::CURRENT_ABI`, the right `wasm_sha256` hash, and the `Effects { kinds: ["hello"] }` capability.
3. Load the WASM via `host.load(&manifest, &wasm_path, &granted)`.
4. Invoke the exported function via the linker.
5. Assert the return value matches the expected output.

## Promotion criteria

- 3+ green runs.
- Mark `S-PLUGIN-001` green.
