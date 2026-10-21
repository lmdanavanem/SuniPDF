# Free local SuniPDF and future enterprise cloud

[Français](CLOUD_STRATEGY.fr.md) · [AEC backlog](AEC_BACKLOG.md)

## Owner decision, 21 October 2026

Cloud and collaboration are deferred. Implement and qualify the local PDF and construction workflows first. No cloud service, desktop connector, billing or public deployment is included in 0.40.0. This document is a future product contract, not an implementation claim.

## Product contract

The local application stays free and open source under the existing MIT OR Apache-2.0 license. Local editing, OCR, forms, signatures, recovery and file operations remain available without a cloud account or subscription.

The future commercial offer is optional managed enterprise hosting and collaboration: company workspaces, shared PDFs, access management, immutable document versions, storage and operations. Pricing and billing terms will be decided later. No prices, entitlements or payment collection are assumed. Revisit service licensing before a commercial launch; the existing repository license is unchanged.

## Deferred implementation sequence

1. Company isolation, viewer/editor/admin roles, immutable PDF versions and guarded edit reservations. Version conflicts must be refused while retaining prior revisions and local source files.
2. An optional desktop connector with explicit selected-file transfers, new-copy downloads, SHA256 checks and protected credentials. Local tools must operate independently.
3. A named deployment with HTTPS, OIDC/SSO, secret provisioning, storage quotas, audit, monitoring, tested backup/restore and an incident procedure. Company documents remain exportable.
4. Live annotation sessions on immutable base documents, authenticated events, per-annotation versions, idempotency, reconnect recovery and tested concurrent-client permissions. Reservations alone are not simultaneous annotation.
5. Billing only after prices, taxes, entitlements and lifecycle rules are defined. Authenticate provider callbacks and process them idempotently. Never disable local PDF tools or silently delete company documents.

AEC-020 and AEC-021 remain deferred in the [26-card backlog](AEC_BACKLOG.md). GitHub Projects edits remain deferred; the repository backlog is the planning record.
