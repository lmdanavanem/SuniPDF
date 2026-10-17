# Third-party notices

[Français](THIRD_PARTY_NOTICES.fr.md)

Every portable ZIP and MSI includes `licenses/dependency-inventory.json` and
`licenses/dependency-licenses.txt`: exact locked Rust versions, source revisions or
registry archive checksums, declared license expressions and full license, copyright
and NOTICE texts. All Cargo.lock packages are inventoried, including development and
other-platform dependencies. Separate x64/ARM64 normal/build graph membership describes
the Windows closure; it does not claim every source package or font survives linking.

`OR` denotes alternative licenses; `AND` denotes cumulative terms. Legacy slash
expressions are preserved as declared. SPDX identifiers index the supplied full terms.
Retain applicable copyrights, licenses, NOTICE, attribution, font naming restrictions
and modification notices as those terms require. An alternative does not impose all
other alternatives.

Registry archives and their legal files are verified against Cargo.lock. Where a crate
omits a full license, reviewed texts are vendored from its exact upstream revision in
`assets/licenses/upstream-sources.json`. Unavailable texts stay explicitly unavailable
and block generation if selected for either Windows graph. The macOS-only `dispatch`
package has such a record and is excluded from both Windows graphs.

| Component | Evidence in licenses/ |
|---|---|
| Rust crates, including egui/eframe/wgpu, PDF bindings, TLS, codecs and XML | Exact inventory and full deduplicated license/NOTICE texts |
| Ubuntu Light, Hack, Noto Emoji, emoji-icon-font | Source hashes and font notice references; full UFL, MIT/Bitstream Vera, OFL and MIT terms |
| PDFium 151.0.7881.0 and native libraries | Original pdfium.txt, pdfium-binaries-MIT.txt and all upstream native notices, with revisions and binary hashes |
| Tesseract 5.5.3 / Leptonica 1.87.0 / pinned English/French tessdata_fast | ocr-tesseract.txt, ocr-leptonica.txt, ocr-tessdata-fast.txt, AUTHORS, COPYRIGHTS, NOTICE, exact source/model revisions and payload hashes |
| Rust standard library | Actual selected toolchain rust-standard-library.html with full component notices and rust-toolchain.txt revision |
| Selected OCR compiler / static MSVC runtime / Windows SDK Universal CRT | ocr-compiler-provenance.json, compiler and library hashes, selected Microsoft Build Tools terms and installed SDK license |
| Rust compiler/Cargo, CMake, Python build tools | Actual versions and full installed or exact upstream legal texts; tool executables are not shipped |
| WiX 3 MSI build tool | Full wix3-MS-RL.txt; MSI additionally records actual selected candle/light versions and hashes |
| SuniPDF | LICENSE-MIT and LICENSE-APACHE |

OCR provenance records CMake-selected compiler and static library inputs. Those hashes
identify inputs; they are not an independent linker attestation. Legacy OCR caches
without this evidence must be rebuilt. PDFium is a pinned upstream binary: version,
hash, supplied notices and upstream DEPS revisions are checked, without claiming
knowledge of the producer's private compiler installation. Windows system DLLs used
through DX12, certificate APIs and other Windows APIs are not redistributed.

package-portable.ps1 checks every staged and archived file. package-msi.ps1 verifies
its input, adds actual WiX evidence to a separate payload copy, then administratively
extracts the MSI and verifies binaries and licenses without installing it. Changed,
missing, extra, unsafe-path or duplicate archive entries fail verification. The MSI's
separate README is excluded from this license-payload comparison.

The icon uses the author-provided assets/icon/sunipdf-source.png; make_icon.py produces
PNG/ICO sizes. No additional third-party graphic resource is introduced. New resources
and models require review of their actual terms before redistribution.
