# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| latest (pre-release) | ✅ |

## Reporting a Vulnerability

**Please do not open a public issue for security vulnerabilities.**

Use GitHub's [private vulnerability reporting](https://github.com/vexil-lang/vexil/security/advisories/new) to submit a report confidentially.

Include in your report:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Any suggested mitigations

You can expect:

- **Acknowledgment** within 48 hours
- **Status update** within 7 days
- **Public disclosure** coordinated after a fix is available (typically 90 days)

## Scope

In scope:

- Remote code execution via malicious `.vexil` schema files
- Memory safety issues in the parser or compiler library
- Data exposure or corruption in generated code

Out of scope:

- Denial of service via intentional resource exhaustion (e.g., deeply nested schemas)
- Issues in upstream dependencies — report those to the relevant project
