# Changelog

User-facing changes to the **Tusk PHP** Zed extension — the editor integration itself
(slash commands, language-server handling) and which **Tusk PHP language server** version
each release bundles.

For the language server's own changes — diagnostics, completions, hover, and Laravel /
Symfony intelligence — see the
[Tusk PHP LSP changelog](https://github.com/Tusk-PHP/lsp/blob/main/CHANGELOG.md). Each
release below links the server notes for the version it ships.

> Zed has no in-editor changelog view or update prompt, so this file is read on GitHub /
> the repository page rather than inside the editor.

This changelog is written for people installing the extension; it does not follow
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) strictly. The format is based on
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.9.0]

Bundles **Tusk PHP language server 0.8.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/blob/main/CHANGELOG.md).

- **Namespace slash commands** — `/tusk-copy-namespace` shows a PHP file's declared
  namespace or FQN, and `/tusk-namespace-for-path` shows the namespace a path *should*
  have from your `composer.json` PSR-4 mappings.
- PHP and `composer.json` are routed to the language server for intelligence across both.
- **Zero-setup server** — the matching language-server binary is downloaded on first use
  and verified against the pinned SHA-256 sums before it runs.
