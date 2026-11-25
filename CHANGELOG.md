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

## [0.16.0] - 2026-11-25

Bundles **Tusk PHP language server 0.16.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/releases/tag/0.16.0).
Thanks to [@stubbedev](https://github.com/stubbedev) and [@AJenbo](https://github.com/AJenbo), who contributed to this server release.

- **Type hierarchy**: browse a class's parents, interfaces and traits, and the classes that extend or implement it.
- **Go to declaration** opens the interface, parent class or trait that first declares a method, property or constant, while go to definition keeps opening the implementation.
- **Ignore a diagnostic** with a `// @tusk-ignore` comment (or the new "Ignore … on this line" quick fix), across files with `diagnosticIgnore` rules in `.tusk-php.json`, and through the `@phpstan-ignore` comments a project already has.
- **Clickable links** on `require` and `include` paths, Laravel path helpers such as `base_path('routes/web.php')`, and URLs in comments.
- **New diagnostics**: `unknown-named-argument` for a named argument the callee has no parameter for, and `unused-private-constant` for a private constant nothing reads.
- **Right answers from the first request**: while a project is still being indexed, a class the editor asks about is loaded straight away, so definition, hover and completion work immediately.
- **Fewer false warnings** on real projects, and Symfony translation keys and console command names now complete and link.

## [0.15.0] - 2026-11-22

Bundles **Tusk PHP language server 0.15.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/releases/tag/0.15.0).

- **Blade views, in full**: hover, completion, go to definition, references and rename work in the PHP inside Blade views. Directives, view names, sections, stacks and components (including Flux and Filament) complete and link, variables passed to a view are typed, and unclosed directives and unknown components are reported.
- **PHP 8.6**: projects on PHP 8.6 get its new syntax (partial function application, parameter doc comments, readonly property defaults), its new functions and classes, and its deprecations.
- **Eloquent static calls** such as `Category::where()` complete and hover with the query builder's methods and keep the model's type, and local scopes (`scopeActive()` or `#[Scope]`) complete on models and their queries.
- **Fewer false warnings** on real projects: about 1,000 fewer across Laravel, Symfony, WordPress and Magento 2, from classes in PSR-0 and optional Composer packages, plain PHP views, and private members used by traits.

## [0.14.0] - 2026-11-18

Bundles **Tusk PHP language server 0.14.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/releases/tag/0.14.0).

- **Typing no longer waits for diagnostics**: they're worked out once you pause, so completion and hover answer straight away, even in files thousands of lines long.
- **Fewer false warnings**: a variable assigned in different branches, checked with `instanceof`, caught in `catch (A|B $e)` or guarded by `method_exists()` now has the type PHP gives it, in hover and completion too.
- **Too few arguments** to a function or method are reported, since PHP throws on them. Calls inside a version check (`version_compare()`, `PHP_VERSION_ID`) are left alone.
- **Class completion no longer adds a clashing import**: when the short name is already taken, the class is imported under an alias (`use Illuminate\Foundation\Auth\User as UserAuth;`).

## [0.13.0] - 2026-11-09

Bundles **Tusk PHP language server 0.13.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/releases/tag/0.13.0).

- **Generics**: types written in PHPDoc the way PHPStan reads them (`Collection<int, User>`, `@template`, `@extends`, `class-string<T>`, closures, conditional return types, `@phpstan-assert`) now carry through hover, completion and diagnostics, so Laravel's collections and query builders, Symfony's messenger and PHPUnit assertions are typed.
- **Diagnostics are 3–4× faster** on large files, and report fewer false warnings.

## [0.12.0] - 2026-11-09

Bundles **Tusk PHP language server 0.12.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/releases/tag/0.12.0).

- **Blade views**: with the Blade extension installed, `.blade.php` files are sent to the language server, which checks the PHP in them.

## [0.11.1] - 2026-11-02

First release through Zed's extension registry.

Bundles **Tusk PHP language server 0.11.1** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/releases/tag/0.11.1).

- The extension ID is now `tusk-php-lsp`, as Zed requires for extensions that only provide
  a language server. Your `lsp.tusk-php` settings keep working unchanged.
- **Removed** the `/tusk-copy-namespace` and `/tusk-namespace-for-path` slash commands,
  which Zed no longer accepts in published extensions. Copy and move namespace come
  back as code actions with Tusk PHP language server 0.9.0.
- The language server now starts for **PHP** files again; it had been attached to
  `composer.json` only.
- `lsp.tusk-php.binary.path` is now used whenever that file exists on the machine the
  project is on, instead of being silently ignored in favour of the downloaded release
  binary. Over **Remote SSH**, a path that only exists locally is skipped and the
  extension falls back to `tusk-php` on the remote `PATH` or the downloaded release.
- Starting the server over **Remote SSH** no longer tries to run a binary from your local
  machine on the remote host.

## [0.9.0]

Bundles **Tusk PHP language server 0.8.0** —
[what's new in the server](https://github.com/Tusk-PHP/lsp/blob/main/CHANGELOG.md).

- **Namespace slash commands** — `/tusk-copy-namespace` shows a PHP file's declared
  namespace or FQN, and `/tusk-namespace-for-path` shows the namespace a path *should*
  have from your `composer.json` PSR-4 mappings.
- PHP and `composer.json` are routed to the language server for intelligence across both.
- **Zero-setup server** — the matching language-server binary is downloaded on first use
  and verified against the pinned SHA-256 sums before it runs.
