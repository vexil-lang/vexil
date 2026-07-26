# GitHub Actions static analysis — 2026-07-26

Scope: repository workflow definitions collected from `.github/workflows`.

Command:

```text
uvx zizmor@1.26.1 --collect=workflows .
```

Result: `No findings to report. Good job! (9 suppressed)`.

The first run reported a `cache-poisoning` finding in the effects-disabled npm
readiness workflow because `actions/setup-node` enables package-manager
caching by default. The workflow now sets `package-manager-cache: false`; the
same scan then returned no findings.

This is static workflow evidence only. It does not assert live GitHub control,
protected-environment, registry-identity, Manifest, or publication readiness.
