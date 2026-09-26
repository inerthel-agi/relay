# Security policy

Relay handles Discord credentials, private local output URLs, user-submitted media, and moderation decisions. Security and privacy reports must remain private until a maintainer has assessed them.

## Supported versions

Report issues against the latest Relay release or the current `main` branch. Older releases may receive guidance to update before the issue can be reproduced.

## Reporting a vulnerability

1. Use GitHub's private vulnerability reporting flow when the repository exposes **Report a vulnerability**.
2. If private reporting is unavailable, email `contact@inerthel.com` with the subject prefix `[Relay security]`.
3. Do not open a public issue, discussion, pull request, or Discord message containing exploit details before a fix is available.

Include the affected Relay version, Windows version, a minimal reproduction using fictional data, expected and actual behavior, and any mitigation already tested.

## In scope

- Discord credential storage, local Relay secret handling, and private Browser Source authorization
- Local HTTP or WebSocket authorization, origin validation, and localhost binding
- Updater source, signature, digest, or installer verification
- Media download trust boundaries, redirects, resource limits, and unsafe decoding behavior
- Privacy scanner bypasses, accidental publication of blocked content, or sensitive data appearing in logs
- Discord command permissions, confirmation flow, mention handling, and role hierarchy checks

## Report safely

- Do not include a Discord token, local output URL, personal address, phone number, image, OCR text, EXIF value, or user-protected string.
- Do not access another person's Discord data, Relay installation, local service, or account.
- Do not perform denial-of-service testing against public Discord, GitHub, or another service.
- Prefer a minimal local fixture and redact every value that is not needed to demonstrate the issue.

## Disclosure

The maintainer will assess reproducibility and impact, coordinate a fix where appropriate, and agree on a disclosure timeline with the reporter. Relay will not request public disclosure before a safe remediation path is available.
