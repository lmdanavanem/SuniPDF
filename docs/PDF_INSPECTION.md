# Inspect a saved PDF

Open **File → Properties** (`Ctrl+D`), expand **Inspect saved PDF**, then choose **Inspect**.
Save document edits first. The report refers to the exact saved revision whose SHA256 is displayed.
If the file on disk changes, reload it before inspecting again. Metadata drafts in the Properties
dialog are not applied by inspection. Closing, switching tabs or editing invalidates pending results.

The inventory reports declarations of JavaScript, automatic actions, external actions, embedded files,
XFA, multimedia and signatures. Counts refer to matching stored dictionaries, including unreachable
objects; they are not unique attachment, script or signature counts. Payload text and attachment names
are not exposed. Actions are not run and no document copy is written.

```powershell
sunipdf-cli inspect document.pdf
sunipdf-cli inspect protected.pdf --password-env PDF_PASSWORD
```

The CLI writes JSON to stdout. A complete dictionary inventory exits with code `0`, even when it contains
findings. A partial inventory still writes its findings and exits with code `2`; `limitations` identifies
unresolved references, malformed feature names or traversal limits. Missing credentials exit with code `3`,
invalid PDF input with `4`, and file I/O errors with `5`. Other refusals use the normal typed error JSON
on stderr. Environment and piped password sources retain the existing CLI safeguards.

The report includes the input's SHA256, byte length, stored-object count, visited-node count, completeness
and seven feature counts. Input is capped at 128 MiB, 250,000 stored objects, one million visited values
and 32 levels of inline dictionary/array nesting. References are not recursively followed: each stored
object is inspected separately, and bounded name dereferencing handles indirect feature names. These are
inventory bounds after PDF parsing, not a native parser memory or execution-time guarantee.

This is a structural inventory, not a malware scanner, safety verdict, signature-integrity check or
PDF/A/PDF/UA conformance validator. Zero findings do not establish safety. Script, attachment and other
stream payloads are not inspected; parsing may decode PDF object/xref streams. Existing signature
verification and explicit sanitization remain separate operations with their own limits.

The saved-revision guard shared with signature verification now checks file length and hashes disk bytes
in bounded chunks. It refuses changed inputs while operations continue to use the captured revision;
it does not allocate another full-file buffer to compare the saved file.

The independent packaged-CLI gate is `scripts/test-pdf-inspection.py`. It covers fourteen scenarios with
handwritten PDF/xref fixtures, unresolved/cyclic values, opaque stream bodies, typed failures, oversized
files, AES-128/256 and environment credentials. Native UI, Narrator and broad hostile-corpus qualification
remain separate work.
