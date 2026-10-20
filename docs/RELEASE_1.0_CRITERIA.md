# SuniPDF 1.0 acceptance criteria

[Français](RELEASE_1.0_CRITERIA.fr.md)

This is the accepted preparation track after 0.32.0, not a claim that 1.0 is qualified. The current
stable release is now [0.34.0](RELEASE_0.34.0.md). Existing supported operations and conservative refusals
stay in the [feature matrix](FEATURE_MATRIX.md); a 1.0 number does not expand font, Office-export,
signature, XFA or PDF-conformance support.

| Gate | Required evidence | Current qualification gap |
|---|---|---|
| Update authenticity | Verify signed metadata and exact packages against an embedded public key; version/expiry checks, rejected tampering and documented key custody/rotation | GitHub HTTPS and same-channel SHA256 only; no independent publication key configured |
| Windows publisher identity | Verify timestamped Authenticode signatures on distributed EXE/MSI payloads | No production code-signing certificate supplied; test certificates do not qualify |
| Document safety | Pinned licensed samples, corruption/large-input campaigns, atomic failure, source immutability and recorded reopening/rendering checks | Four NIST documents/744 pages and seven PDF Association teaching examples/8 pages are released QA baselines; broader customer, hostile and large-file coverage remains open |
| Independent interoperability | Saved encrypted documents, forms, annotations and signatures checked in the named external readers with exact versions and outcomes | Existing qpdf/MuPDF gates cover bounded cases; Acrobat/Foxit qualification remains open |
| Native Windows experience | Clean installation and upgrade, startup notification/install/cancel/rollback, save/recovery dialogs, keyboard/Narrator, DPI/GPU and physical printing | Scripted/headless/offscreen tests do not qualify native interactions or a physical printer |
| ARM64 support | Run the actual public package on real Windows ARM64 and record OS/device/results | Cross-compilation and PE inspection only; keep runtime support experimental until qualified |
| PDF/A and PDF/UA claims | Name supported profiles, retain independent validator reports, and perform applicable human accessibility checks | Metadata preservation is not conformance validation; no universal PDF/A/PDF/UA claim |

For every gate, an unsupported or unavailable result must stay explicit. A partial report, a successful
build, an ignored test or a signed test fixture must not close an unverified acceptance criterion.

The first preparation lot, released in 0.33.0, adds seven pinned PDF Association teaching examples under
CC BY-SA 4.0, deterministic PDF/CMS mutation smoke tests, per-command measurements and hardware metadata.
The examples cover UTF-8 annotations, incremental saves, a displaced header and colour/output-intent
structures. They are distinct from the NIST research documents and from a real customer/hostile corpus.
All downloaded examples and derived PDF copies remain outside Git and application packages.

The mutation campaign is bounded smoke testing, **not coverage-guided fuzzing**. Its timeout/output guards
have executable negative controls; successful editing outputs must reopen and rejected edits must publish
no output. Reports identify the environment and sampling limits; CI/virtual-machine measurements do not
establish performance on a customer's physical device. Keep both relevant P1 cards open until their full
criteria have independent evidence.

Prepare a release candidate only after required CI/security/package gates pass on the frozen source and
all P1 criteria in the advertised 1.0 scope are met. Freeze features for pilot use, fix regressions,
then qualify the same source and its actual downloaded packages. Retain updater and installer evidence,
English/French guides, license inventories, reproducible source links and recovery instructions.

The eight remaining P3 Suggestions can follow 1.0 unless a concrete user failure makes one necessary.
No release date or hardware-validation result is implied by this checklist.
