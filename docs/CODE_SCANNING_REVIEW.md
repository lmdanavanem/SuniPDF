# Code Scanning review — 2026-10-16

[Français](CODE_SCANNING_REVIEW.fr.md) · [Per-alert critical inventory (CSV)](qa/CODE_SCANNING_2026-10-16.csv)

## 0.30.0 code scanning review — 2026-10-17

The [first complete scan](https://github.com/lmdanavanem/SuniPDF/actions/runs/38994022178) of source
`b87beea529196e7f93308908eef1e96be4feff25` passed its Rust, Python and Actions analysis jobs and reported
**27 new critical test-fixture findings** (#129–155) and **five high native-pointer findings** (#124–128).

All 27 criticals were reviewed individually against that exact source and dismissed by `lmdanavanem`
as `used in tests`. They are deterministic encrypted-PDF passwords and synthetic PFX identities in
Cargo integration tests or explicit `cfg(test)` modules. No production credential or application default
was identified. The [per-alert CSV](qa/CODE_SCANNING_0.30.0_2026-10-17.csv) records scope, source, location,
reason, author, timestamp and the verified dismissal comment for every finding.

The five high findings were corrected with explicit guards and checked ownership of native certificate,
chain and timestamp pointers, including null/missing certificate regressions. The [fresh scan](https://github.com/lmdanavanem/SuniPDF/actions/runs/38995917746)
passed its three analysis jobs on source `4896cc277c32621e5d65335b4fd9823760427793`.
GitHub reports #124–128 as **fixed** at `2026-10-17T02:51:43Z`; they were not dismissed.
The unfiltered authenticated API subsequently confirmed **zero open alerts**. Queries, test fixtures
and source coverage remain enabled without exclusions. This checkpoint does not replace the complete
application and package qualification required for publication.

## Verified 0.21.0 remediation

[CodeQL](https://github.com/lmdanavanem/SuniPDF/actions/runs/38957648825) passed all Rust/Actions/Python jobs
on source `f006b7dd027fd5a3084c2bfffc2fceaa1b66be46`. The unfiltered authenticated API confirms **zero open alerts** at `2026-10-16T19:59:54.8976555Z`.
**High #3 fixed at `2026-10-16T19:58:04Z`; medium #119 fixed at `2026-10-16T19:49:12Z`.**
Neither was dismissed: fresh analysis verified the explicit native certificate check and root `contents: read` permissions.
#119 is the reindexed release finding #1; #1 and #2 remain separate historical fixes.

All **115 criticals** were individually dismissed by **`lmdanavanem`** on 2026-10-16 between 19:34:00 and
19:38:10 UTC: **114 `used in tests`**, **#13 `false positive`** (empty authentication probe, no embedded secret).
The [CSV](qa/CODE_SCANNING_2026-10-16.csv) retains every historical field and adds the verified state, reason,
author, timestamp and comment. Fixtures and queries remain enabled; no directory is excluded.
Rust analysis still contains **115 already-triaged findings**, Actions/Python zero; this does not establish a vulnerability-free project.

471 local/CI tests, formatting/Clippy, 912 keys, 75 qpdf checks, 4/744 corpus, helper, 19 portable cases,
13 MSI baselines/65 logs, ARM64, six public packages/sidecars and updater passed. Three new CMS and UTF-8/UTF-16
XMP/XFDF DTD/entity regressions passed; bounded XML 1.0 parsing and accurate license notices are retained.
[0.21.0](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.21.0) was stable Latest at publication. See [Project status](PROJECT_STATUS.md) for evidence.
Exhaustive fuzzing, certificate trust/revocation/timestamps and hardware remain unchecked.

The initial snapshot below is historical: its open states describe the original review rather than the current state.

## Original reviewed snapshot

All **117 open alerts** in the authenticated GitHub API snapshot were reviewed: **115 critical, one high
and one medium**. There is also **one already-fixed medium alert (#2)** and **zero dismissed alerts**.
No other closed critical alert appears in this snapshot. These are GitHub alert states; a source-review
classification does not close an alert.

The scan refers to commit `72dd623576203988a22fdc9662963b7982c7c746`; the reviewed 0.20.0 release source is
`54054b101282a7688dde778b24090af6d6e1e6a6`. The alert-bearing application and workflow files are identical
between these commits; the tracked difference is `.github/FUNDING.yml`. Locations below refer to that source.

## Results

| GitHub severity | Alerts | Reviewed result |
|---|---:|---|
| Critical | 114 | Deterministic test inputs; no production credential identified in these alerts |
| Critical | 1 (#13) | Intentional empty-password PDF authentication attempt; no embedded secret |
| High | 1 (#3) | False positive for the reported initial-null pointer path; high confidence from source and Windows API contracts |
| Medium | 1 (#1) | Missing explicit workflow-level permissions; real declaration gap, currently mitigated by read-only repository defaults |
| **Open total** | **117** | **117 reviewed; zero left unreviewed in this snapshot** |
| Medium, already fixed | 1 (#2) | CI workflow permission declaration; historical fixed state, separate from the open total |

No production secret was confirmed among the 115 critical alerts. This finding does not establish that the
whole repository is free of vulnerabilities. All 117 alerts remain open in the reviewed snapshot; this review
did not dismiss alerts, change scanning configuration or permissions, or modify application/workflow code.

## Critical: hard-coded cryptographic values

All **115/115** use `rust/hard-coded-cryptographic-value`, covering **105 distinct source locations**,
**23 files** and **39 enclosing functions**. Duplicate GitHub alerts at the same location remain separate
CSV rows. The CSV preserves alert links, exact functions, test-scope evidence, classifications, supporting
locations and redacted source anchors; credential values are not copied.

The **114 test-only alerts** consist of **97 integration-test alerts, 16 inside explicit `#[cfg(test)]`
modules and one test-support alert**. Each location and enclosing function was inspected; test attributes,
conditional test modules and helper callers establish the test-only scope. Fixed passwords and
invalid/empty/Unicode inputs exercise PDF protection and failure behavior. Signature tests generate
ephemeral RSA-2048 identities in temporary directories; their fixed PFX test passphrase is not a shipping
private key or account credential. No reviewed alert was classified as a required algorithm constant.

### Production alert #13

[`security::load`](https://github.com/lmdanavanem/SuniPDF/blob/54054b101282a7688dde778b24090af6d6e1e6a6/crates/sunipdf-core/src/security.rs#L162-L199)
tries the empty PDF user password when the caller supplies none. The empty string is not a stored secret.
`load_mem_with_options` still authenticates; invalid credentials map to `PasswordRequired`, and an encrypted
result is rejected. This supports owner-only protection without granting permission to remove restrictions.
`unprotect` separately verifies the owner password. The writer takes caller passwords and obtains AES-256
key material from `getrandom`. Existing owner-only and wrong-password regressions cover this behavior.
[Alert #13](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/13) remains open.

Keep deterministic regression inputs. Randomizing them or concealing literals solely to silence the analyzer
would not repair a vulnerability and could weaken tests. Any later triage should retain the per-alert rationale;
do not exclude entire folders on the basis of this summary.

## High #3: reported invalid pointer access

[`rust/access-invalid-pointer`](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/3) reports
`certificate_native.rs:222`, with a trace starting from `null_mut` at line 185. The reported initial-null path
is a **false positive with high confidence**: the lazy `(!cert.is_null()).then(|| Cert(cert))` construction only creates
the owned certificate wrapper for a non-null pointer; `ok_or(...)?` returns before `cert_info` when it is null.
The API-error branch also exits before that call. This is a source/API-contract conclusion, not a malformed-CMS
fuzzing result.

Windows returns an owned signer certificate context through the output parameter of
[`CryptVerifyDetachedMessageSignature`](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptverifydetachedmessagesignature).
The `Option<Cert>` remains alive while `cert_info` borrows its pointer; `Cert::drop` frees the context afterwards
or on an error return. The only other caller, `Identity::info`, uses a certificate whose enumeration result was
checked for null before construction, consistent with
[`CertEnumCertificatesInStore`](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-certenumcertificatesinstore).
Windows decodes the [`CERT_CONTEXT`](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/ns-wincrypt-cert_context);
neither caller casts arbitrary PDF memory into that structure. No reachable null dereference or premature free
was found through these two callers.

Existing native tests cover signing/verification, tampering, invalid PFX and wrong/empty passwords. They do not
exhaust malformed CMS inputs or force successful API output with a null certificate. Optional clarification:
an explicit null-check branch could express the same invariant more directly without adding a wrapper.

## Medium #1: release workflow permissions

[`actions/missing-workflow-permissions`](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/1)
is a **confirmed explicit least-privilege declaration gap**: `release.yml` has no root/build-job `permissions`.
The separate Publish job's `contents: write` does not constrain the build job.

An authenticated read of repository workflow settings returned `default_workflow_permissions: "read"` and
`can_approve_pull_request_reviews: false`. The build therefore inherits read-only defaults; no excessive write
token was observed. The workflow runs for tags/manual dispatch, not untrusted pull-request events. Broader
defaults enabled later, or copying the workflow elsewhere, would make the omission more consequential.
This alert alone is not evidence of compromise.

Smallest future hardening: declare root-level `permissions: { contents: read }` and retain Publish's explicit
`contents: write`, following [GitHub's workflow/job permission rules](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#permissions).
This review records the finding and does not change those settings or files.

## Historical fixed alert #2

[Alert #2](https://github.com/lmdanavanem/SuniPDF/security/code-scanning/2), another medium
`actions/missing-workflow-permissions` finding in `ci.yml`, is already **fixed** in the API snapshot
(`fixed_at: 2026-10-16T18:51:19Z`). CI now explicitly declares root `contents: read`. This historical fix is
separate from release-workflow alert #1 and was not performed by this review.

## Limits

This is complete static coverage of the snapshot's open alerts, not a whole-project penetration test or a
guarantee of security. The review ran no new Cargo, exploitation or fuzzing tests and changed no GitHub alert
states. Existing release regressions are recorded separately in [Project status](PROJECT_STATUS.md).
