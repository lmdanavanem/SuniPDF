# Edit annotation fields in a batch

The annotation register can apply one reviewed field patch to explicitly selected annotations. This edits business status, discipline and custom text fields. It does not change count groups, PDF review states, typed quantity columns, annotation appearance or page content.

## Desktop workflow

1. Open **Annotation register** and select the required rows with their checkboxes. Selection is explicit and limited to 256 annotations. Changing filters or refreshing the source clears selection; sorting preserves the selected identities.
2. Choose **Edit selected**. For each field, choose **Leave unchanged**, **Set value** or **Clear**. Custom fields are merged by key: keys omitted from the patch keep their existing values.
3. Choose **Preview changes** and inspect the before/after values for every selected row. Rows whose values already match remain part of validation but are not counted as changed.
4. Confirm that you reviewed the changes, then apply the batch. A batch with no actual changes cannot be applied.
5. Save the PDF. The complete batch has one Undo/Redo step.

Changing the patch invalidates its preview. Intervening document edits or navigation invalidate a prepared operation. Cancellation keeps the draft without changing the document. If any selected row is stale, locked or unsupported, the whole batch is refused before modifying an annotation.

Business statuses are separate from standard PDF review states. A custom field set to empty text is also distinct from clearing that key. The existing single-row **Edit fields** operation retains its replacement semantics; batch editing uses explicit per-key changes.

## CLI workflow

Inspect a saved PDF to obtain its source hash and exact row fingerprints:

```sh
sunipdf-cli annotation-fields-batch inspect plan.pdf
sunipdf-cli annotation-fields-batch inspect plan.pdf --page 2
```

Inspection does not select or modify rows. Build a version-1 JSON plan using the returned `source_file_sha256` and `expected_row_sha256` values. Pages are physical, starting at 1; annotation indexes start at 0. The following hash placeholders must be replaced with actual inspection values:

```json
{
  "version": 1,
  "expected_source_sha256": "<source_file_sha256 from inspection>",
  "targets": [
    { "page": 2, "index": 0, "expected_row_sha256": "<row fingerprint>" }
  ],
  "patch": {
    "business_status": { "operation": "set", "value": "Ready for site" },
    "discipline": { "operation": "keep" },
    "custom": {
      "Package": { "operation": "set", "value": "A" },
      "Obsolete": { "operation": "clear" }
    }
  }
}
```

Preview the full patch, then save a new PDF copy:

```sh
sunipdf-cli annotation-fields-batch preview plan.pdf --plan fields.json
sunipdf-cli annotation-fields-batch apply plan.pdf --plan fields.json --out reviewed.pdf
```

Apply revalidates the saved source and every selected row. Existing output files are refused, including with `--overwrite`. Source files remain unchanged. The plan binds saved-file bytes; the report also identifies the internal document snapshot used for validation.

Protected inputs use the existing global `--password-env` or `--password-stdin` options. Passwords do not belong in the plan. Copy and annotation permissions are checked; supported encrypted outputs retain encryption.

## Bounds and limits

- A plan is at most 1 MiB and contains 1–256 distinct row targets. Unknown fields, duplicate custom keys and invalid fingerprints are refused.
- Register text metadata supports at most 16 custom keys, with keys up to 64 UTF-8 bytes, values up to 1,024 bytes and business status/discipline up to 512 bytes. The resulting fields on every annotation must fit these bounds.
- Saved source and edited document snapshots are bounded to 128 MiB. The existing register limits of 1,000 pages and 10,000 rows still apply.
- Signed documents, unsupported annotation types and ambiguous annotation ownership are refused. The operation does not repair malformed metadata or bypass document permissions.

Qualification is pending for the 0.86 candidate. Automated fixtures, native desktop interaction and broader independent-reader interoperability are separate evidence scopes.

[Annotation register](MARKUP_REGISTER.md) · [Quantity tables](QUANTITY_TABLE.md)
