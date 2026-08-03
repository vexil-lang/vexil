# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| Latest pre-release | Yes |

## Report a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's
[private vulnerability reporting](https://github.com/vexil-lang/vexil/security/advisories/new)
so the report can be investigated before disclosure.

Please include:

- the affected component and version;
- a minimal schema, input, or reproduction procedure;
- the expected and observed behavior;
- the potential impact;
- any mitigation you have already identified.

The maintainer aims to acknowledge reports within 48 hours, provide a status
update within seven days, and coordinate disclosure after a fix is available,
typically within 90 days. Complex or disputed reports may require a different
timeline; material changes will be communicated through the private report.

## Scope

In scope:

- remote code execution through a malicious `.vexil` schema;
- memory-safety issues in the parser or compiler library;
- data exposure or corruption in generated code.

Out of scope:

- denial of service through intentional resource exhaustion, such as deeply
  nested schemas;
- issues in upstream dependencies, which should be reported to the relevant
  project.

No independent security audit has been completed. Review the current
[limitations](docs/limitations-and-gaps.md) when assessing deployment risk.
