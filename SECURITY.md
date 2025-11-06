# Security Policy

## Reporting a vulnerability

If you discover a vulnerability in ASM or its replication infrastructure, send
an email to `security@example.com` with a detailed description of the issue,
steps to reproduce, and any suggested mitigations. Please do not disclose the
vulnerability publicly until we have acknowledged receipt and provided a fix or
mitigation.

## Supported versions

Security fixes are applied to the latest release candidate (`v1.0.0-rc*`) and
backported to the most recent stable release when applicable.

## Scope

This policy covers the Rust crates under `crates/`, the `asm-sim` binary, the
replication pack, and the published Docker image. Third-party dependencies are
tracked via `Cargo.lock`; please notify the upstream project directly for
vulnerabilities outside this scope.
