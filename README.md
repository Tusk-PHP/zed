# Tusk PHP — Zed extension

Zed extension for [Tusk PHP](https://github.com/Tusk-PHP/lsp), a PHP language
server with Laravel and Symfony awareness.

The extension downloads the `tusk-php` language server binary from
[Tusk-PHP/lsp releases](https://github.com/Tusk-PHP/lsp/releases) on first use.

## LSP version

This extension is pinned to a specific, tested LSP version — see
[`tusk-lsp.toml`](./tusk-lsp.toml). The downloaded binary is verified against the
SHA-256 sums declared there before it is used.

## Development

```bash
cargo build --release --target wasm32-wasip2
```

Then install as a [Zed dev extension](https://zed.dev/docs/extensions/developing-extensions)
pointing at this directory.

## Releases

Registered with [`zed-industries/extensions`](https://github.com/zed-industries/extensions).
Each release pins a known-good `Tusk-PHP/lsp` version via `tusk-lsp.toml`.
