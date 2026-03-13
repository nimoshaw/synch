# Security Policy

## Reporting a Vulnerability

We take the security of Synch seriously. If you believe you have found a security vulnerability, please report it to us by following these steps:

1. **Do NOT open a public issue.**
2. Send an email to `security@synch.protocol` (placeholder) with a detailed description of the vulnerability.
3. Include steps to reproduce the issue and any potential impact.

We will acknowledge your report within 48 hours and provide a timeline for resolution if the vulnerability is confirmed.

## Supported Versions

Only the latest release is actively supported for security updates.

| Version | Supported          |
| ------- | ------------------ |
| v0.1.x  | ✅ Yes            |
| < v0.1  | ❌ No             |

## Cryptography
Synch relies on well-vetted libraries (e.g., `dalek-cryptography`, `aes-gcm`). Any changes to the core cryptographic modules require a rigorous peer review.
