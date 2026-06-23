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

## Changelog

This repo keeps a thin, **user-facing** [`CHANGELOG.md`](./CHANGELOG.md) for the extension
itself — slash commands, editor integration, and which language-server version each
release bundles. When a release ships a new server, the entry records *"Bundles Tusk PHP
language server X.Y.Z"* and **links** to the detailed
[LSP changelog](https://github.com/Tusk-PHP/lsp/blob/main/CHANGELOG.md) rather than
restating it (the same convention as the [VS Code extension](https://github.com/Tusk-PHP/vscode)).

Unlike VS Code, **Zed has no in-editor changelog tab or update prompt** — the extension
API can't surface release notes — so this file is only read on GitHub / the repository
page. Keep it accurate anyway: it's the single user-facing record of what each Zed release
changed.
