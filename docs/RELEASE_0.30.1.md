# SuniPDF 0.30.1

[Français](RELEASE_0.30.1.fr.md)

This security patch strengthens document cleanup, protection changes, certificate verification and
command-line automation. Publication and executed validation are recorded in [Project status](PROJECT_STATUS.md).

- Sanitization removes XFA packets and Screen multimedia annotations, then checks that they are absent.
  Redaction's default hidden-data cleanup also removes XFA and replacement accessibility text from
  retained content/resources. Ambiguous content that cannot be rewritten safely is refused.
- Replacing an encrypted document's protection requires its current owner password. Existing protection
  remains in place until the operation succeeds.
- PDF signature integrity accepts detached CMS with SHA-256, SHA-384 or SHA-512. Weak/unsupported digests
  and attached CMS content are refused. Identity trust also requires valid current integrity and usable
  Windows chain/revocation evidence.
- Signature verification uses the exact loaded or saved bytes. A changed, missing or unsaved source
  must be reloaded or saved first. Signer declarations are bounded, scrubbed and visually separated
  from verifier results; certificate subject/issuer names are more complete. Ambiguous multi-key PFX
  files are refused.
- The CLI accepts PDF passwords through environment variables or piped UTF-8 stdin. Legacy password
  arguments remain available with explicit warnings. Text output strips terminal controls; JSON escapes
  C1 controls without changing decoded values. `signatures` prints its complete report and exits with
  code 2 when verification fails. Signing time remains explicitly signer-declared.
- Local Office input guards refuse inline binary payloads and conditional expressions in supported
  macro-free ODF files. Release builds ignore development-only native-library, OCR, signature-storage
  and file-picker environment overrides. Windows desktop startup restricts DLL search directories.
- Workflow actions use verified commit pins. The official WiX fallback archive is SHA256-checked before
  extraction; previous public 0.30.0 ZIP/MSI packages are included in upgrade regressions.

The updater still depends on GitHub HTTPS and checksums delivered through GitHub. These checks detect
corruption; they do not independently authenticate a publisher against a compromised distribution
account. No independent publication key is available, so signed update metadata remains separate work.
Windows packages also remain unsigned. ARM64 hardware, physical printers and exhaustive hostile-input
testing remain separate qualification. PDF cleanup does not certify PDF/A or PDF/UA conformance.

The supplied private review is not distributed with this release. Supported behavior and limits:
[document cleanup](SECURITY_MODEL.md), [signatures](SIGNATURE_SECURITY.md),
[redaction](REDACTION.md) and [Office conversion](OFFICE_CONVERSION.md).

## Validation and distribution

Published as [stable Latest](https://github.com/lmdanavanem/SuniPDF/releases/tag/v0.30.1) under `lmdanavanem`, release `417149160`, immutable source `697a963c756b316e1eb1e678298886bc08521da1`.

[Release workflow](https://github.com/lmdanavanem/SuniPDF/actions/runs/39235567730) and [main CI](https://github.com/lmdanavanem/SuniPDF/actions/runs/39234732417) passed on this source: **575 tests**, zero failures, 9 default ignored; **1060 EN/FR keys**, **84 qpdf corpus checks** plus **9 independent feature cases**, **4 font-reader cases**, DX12 software rendering, **14 Office round trips**, 4 licensed PDFs/744 pages, **22 portable scenarios**, **16 MSI baselines/80 transaction logs**. CodeQL Rust, Python and Actions passed with zero open alerts.

The six public files and `dist/` copies match the inspected packages and SHA256 digests. Eight security scenarios also pass with the packaged CLI. The production updater verified the **27,029,229-byte** x64 ZIP in **1.21 s**, without installation on this host, using the previously built harness and unchanged updater module.

[Detailed evidence](qa/RELEASE_0.30.1.json).
