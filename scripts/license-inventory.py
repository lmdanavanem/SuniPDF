"""Generate and verify distribution notices from locked sources and actual payloads."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import struct
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import unittest
import zipfile

INVENTORY = 'licenses/dependency-inventory.json'
TEXTS = 'licenses/dependency-licenses.txt'
LEGAL_NAME = re.compile(r'(?i)^(licen[cs]e|copying|copyright|notice|unlicense)([._-]|$)')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_plain_text(path):
    data = Path(path).read_bytes()
    encoding = 'utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig'
    return data.decode(encoding).replace('\r\n', '\n').replace('\r', '\n')


def read_json(path):
    return json.loads(read_plain_text(path))


def legal_text(path):
    data = path.read_bytes()
    if not data or len(data) > 16 * 1024 * 1024:
        raise ValueError(f'Empty or oversized legal text: {path.name}')
    return data.decode('utf-8-sig').replace('\r\n', '\n').replace('\r', '\n')


def inside(path, directory):
    if path.is_symlink():
        raise ValueError('Linked legal resource is not allowed')
    path = path.resolve()
    if not path.is_relative_to(directory.resolve()):
        raise ValueError('Legal resource escapes its source directory')
    return path


def windows_ids(metadata):
    nodes = {node['id']: node for node in metadata['resolve']['nodes']}
    pending = list(metadata['workspace_members'])
    selected = set()
    while pending:
        current = pending.pop()
        if current in selected:
            continue
        selected.add(current)
        for dep in nodes[current]['deps']:
            if any(kind['kind'] != 'dev' for kind in dep['dep_kinds']):
                pending.append(dep['pkg'])
    return selected


def rust_inventory(root, metadata, target_metadata):
    locked = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))['package']
    by_key = {(p['name'], p['version'], p.get('source') or ''): p for p in metadata['packages']}
    overrides = {(p['name'], p['version']): p for p in read_json(root / 'assets/licenses/upstream-sources.json')['packages']}
    active = {arch: windows_ids(data) for arch, data in target_metadata.items()}
    texts = {}
    packages = []

    def add_text(path, source):
        text = legal_text(path)
        text_id = digest(text.encode('utf-8'))
        texts.setdefault(text_id, {'text': text, 'sources': set()})['sources'].add(source)
        return {'textSha256': text_id, 'source': source}

    for lock in sorted(locked, key=lambda p: (p['name'], p['version'], p.get('source', ''))):
        key = (lock['name'], lock['version'], lock.get('source', ''))
        if key not in by_key:
            raise ValueError(f'Metadata does not match Cargo.lock: {key[0]} {key[1]}')
        package = by_key[key]
        directory = Path(package['manifest_path']).parent
        license_expression = package.get('license')
        if not license_expression and not package.get('license_file'):
            raise ValueError(f'Missing declared license: {key[0]} {key[1]}')
        prefix = f"{lock.get('source', 'workspace')}::{key[0]}@{key[1]}"
        candidates = {p for p in directory.rglob('*') if p.is_file() and LEGAL_NAME.match(p.name)}
        if package.get('license_file'):
            candidates.add(inside(Path(package['license_file']), directory))
        if not lock.get('source'):
            candidates.update([root / 'LICENSE-MIT', root / 'LICENSE-APACHE'])
        if package['name'] == 'epaint_default_fonts':
            candidates.update((directory / 'fonts').glob('*.txt'))
        authors = directory / 'AUTHORS'
        if authors.is_file():
            candidates.add(authors)
        archive = None
        git_root = None
        git_revision = None
        if lock.get('checksum'):
            archive_path = directory.parents[2] / 'cache' / directory.parent.name / f'{key[0]}-{key[1]}.crate'
            if digest(archive_path.read_bytes()) != lock['checksum']:
                raise ValueError(f'Registry archive differs from Cargo.lock: {key[0]}')
            archive = tarfile.open(archive_path, 'r:gz')
            archived_manifest = archive.extractfile(f'{key[0]}-{key[1]}/Cargo.toml')
            if not archived_manifest or archived_manifest.read() != (directory / 'Cargo.toml').read_bytes():
                raise ValueError(f'Cached manifest differs from the locked archive: {key[0]}')
        elif lock.get('source', '').startswith('git+'):
            git_revision = lock['source'].rsplit('#', 1)[-1]
            if not re.fullmatch('[0-9a-f]{40}', git_revision):
                raise ValueError('An exact locked Git revision is required')
            git_root = Path(subprocess.check_output(['git', '-C', str(directory), 'rev-parse', '--show-toplevel'], stderr=subprocess.PIPE).decode().strip())
            manifest_name = inside(directory / 'Cargo.toml', git_root).relative_to(git_root).as_posix()
            original = subprocess.check_output(['git', '-C', str(git_root), 'show', git_revision + ':' + manifest_name], stderr=subprocess.PIPE)
            if original != (directory / 'Cargo.toml').read_bytes():
                raise ValueError('Git manifest differs from its locked revision')
        evidence = []
        for path in sorted(candidates, key=lambda p: p.as_posix()):
            allowed_root = root if not lock.get('source') else directory
            path = inside(path, allowed_root)
            relative = path.relative_to(allowed_root).as_posix()
            if archive:
                original = archive.extractfile(f'{key[0]}-{key[1]}/{relative}')
                if not original or original.read() != path.read_bytes():
                    raise ValueError(f'Changed cached license text: {key[0]}/{relative}')
            if git_root:
                git_name = inside(path, git_root).relative_to(git_root).as_posix()
                original = subprocess.check_output(['git', '-C', str(git_root), 'show', git_revision + ':' + git_name], stderr=subprocess.PIPE)
                if original != path.read_bytes():
                    raise ValueError('Git legal resource differs from its locked revision: ' + git_name)
            evidence.append(add_text(path, prefix + '/' + relative))
        if archive:
            archive.close()
        override = overrides.get((key[0], key[1]))
        if override:
            if override['license'] != license_expression:
                raise ValueError(f'Pinned upstream license declaration changed: {key[0]}')
            vcs = read_json(directory / '.cargo_vcs_info.json')
            if override['revision'] != vcs['git']['sha1'] or override['path'] != vcs.get('path_in_vcs', ''):
                raise ValueError('Upstream legal source differs from the published crate revision: ' + key[0])
            for entry in override['texts']:
                path = inside(root / entry['file'], root / 'assets/licenses')
                if digest(path.read_bytes()) != entry['sha256']:
                    raise ValueError(f'Pinned upstream legal text changed: {key[0]}')
                reference = add_text(path, entry['source'])
                if entry.get('linked_by'):
                    reference['linkedBy'] = entry['linked_by']
                evidence.append(reference)
        selected_targets = sorted(arch for arch, ids in active.items() if package['id'] in ids)
        unavailable = override.get('unavailable') if override else None
        if (not evidence or unavailable) and selected_targets:
            raise ValueError(f'Full license text unavailable for Windows distribution: {key[0]} {key[1]}')
        packages.append({
            'name': key[0], 'version': key[1], 'source': lock.get('source', 'workspace'),
            'archiveSha256': lock.get('checksum'), 'repository': package.get('repository'),
            'declaredLicense': license_expression, 'licenseFile': Path(package['license_file']).name if package.get('license_file') else None,
            'windowsNormalOrBuildTargets': selected_targets,
            'texts': sorted(evidence, key=lambda item: (item['source'], item['textSha256'])),
            'unavailable': unavailable if unavailable else (None if evidence else 'No full license text in the resolved source'),
            'distributionChoice': override.get('distribution_choice') if override else None,
        })
    return packages, texts


def machine(path):
    with path.open('rb') as stream:
        if stream.read(2) != b'MZ':
            raise ValueError(f'Not a Windows binary: {path.name}')
        stream.seek(0x3c)
        offset = struct.unpack('<I', stream.read(4))[0]
        stream.seek(offset)
        if stream.read(4) != b'PE\0\0':
            raise ValueError(f'Invalid Windows binary: {path.name}')
        return struct.unpack('<H', stream.read(2))[0]


def payload_map(stage):
    output = {}
    for path in sorted(stage.rglob('*')):
        if path.is_symlink():
            raise ValueError('Linked package resource is not allowed')
        if path.is_file():
            relative = path.relative_to(stage).as_posix()
            if relative != INVENTORY:
                output[relative] = {'sha256': digest(path.read_bytes()), 'bytes': path.stat().st_size}
    return output


def bundled_inventory(root, stage, arch, texts):
    native = read_json(root / 'assets/licenses/native-components.json')
    pdfium = native['pdfium']
    pdfium_root = root / 'third_party/pdfium' / ('win-' + arch)
    ocr_root = root / 'third_party/ocr' / ('win-' + arch)
    if (pdfium_root / 'VERSION').read_text().replace('\r', '').splitlines() != ['MAJOR=151', 'MINOR=0', 'BUILD=7881', 'PATCH=0']:
        raise ValueError('PDFium version is not covered by the pinned license inventory')
    pdfium_expected = {'x64': '79d4676b656cfb1abcea88f9ade3b4b0826c5200382db5f4ec72a636c598c118', 'arm64': '267a6f08a9c854d9949754a53b7630f23c0b67de5c7ca273b6abf178b49158c2'}
    if digest((stage / 'pdfium.dll').read_bytes()) != pdfium_expected[arch]:
        raise ValueError('PDFium binary does not match the pinned upstream distribution')
    for path in ('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll', 'licenses/tesseract.exe'):
        if machine(stage / path) != {'x64': 0x8664, 'arm64': 0xaa64}[arch]:
            raise ValueError(f'Package architecture mismatch: {path}')
    for line in (ocr_root / 'SHA256SUMS').read_text().splitlines():
        expected, relative = line.split('  ', 1)
        original = inside(ocr_root / relative, ocr_root)
        if digest(original.read_bytes()) != expected.lower():
            raise ValueError('OCR source payload checksum mismatch')
        if relative in ('tesseract.exe', 'tessdata/eng.traineddata', 'tessdata/fra.traineddata'):
            target = stage / 'licenses' / Path(relative).name
        elif relative.startswith('licenses/'):
            target = stage / 'licenses' / ('ocr-' + Path(relative).name)
        elif relative == 'NOTICE.txt':
            target = stage / 'licenses/ocr-NOTICE.txt'
        else:
            raise ValueError('Uninventoried OCR resource: ' + relative)
        if target.read_bytes() != original.read_bytes():
            raise ValueError('Shipped OCR resource differs from its verified source: ' + relative)
    ocr_version = (ocr_root / 'VERSION').read_text().strip()
    if not ocr_version.startswith('TESSERACT=5.5.3;LEPTONICA=1.87.0;TESSDATA=87416418657359cb625c412a48b6e1d6d41c29bd;') or not ocr_version.endswith('ARCH=' + arch) or ';COMPILER_PROVENANCE=1;' not in ocr_version:
        raise ValueError('OCR version is not covered by the pinned license inventory')
    if (stage / 'licenses/ocr-VERSION.txt').read_text().strip() != ocr_version:
        raise ValueError('Shipped OCR version marker differs')
    originals = {p.name: p for p in (pdfium_root / 'licenses').iterdir() if p.is_file()}
    originals['pdfium-binaries-MIT.txt'] = pdfium_root / 'LICENSE'
    expected_names = {name for component in pdfium['components'] for name in component['files']}
    if set(originals) != expected_names:
        raise ValueError('PDFium license file set differs from the reviewed native inventory')
    components = []
    for component in pdfium['components']:
        entry = dict(component)
        entry['scope'] = 'pdfium.dll'
        entry['source'] = pdfium['source'] + '/' + component.get('sourcePath', '')
        entry['texts'] = []
        for name in component['files']:
            path = stage / 'licenses' / name
            if path.read_bytes() != originals[name].read_bytes():
                raise ValueError('Shipped PDFium notice differs from upstream: ' + name)
            content = legal_text(path)
            text_id = digest(content.encode())
            texts.setdefault(text_id, {'text': content, 'sources': set()})['sources'].add('licenses/' + name)
            entry['texts'].append({'path': 'licenses/' + name, 'textSha256': text_id})
        components.append(entry)
    for name, component in native['ocr'].items():
        entry = dict(component, name=name, scope='licenses/tesseract.exe' if name != 'tessdata_fast' else 'licenses/eng.traineddata and fra.traineddata')
        entry['texts'] = []
        for filename in component['files']:
            notice = stage / 'licenses' / filename
            if component.get('licenseSha256') and digest(notice.read_bytes()) != component['licenseSha256']:
                raise ValueError('Pinned model license differs from its actual source')
            content = legal_text(notice)
            text_id = digest(content.encode())
            texts.setdefault(text_id, {'text': content, 'sources': set()})['sources'].add('licenses/' + filename)
            entry['texts'].append({'path': 'licenses/' + filename, 'textSha256': text_id})
        components.append(entry)
    wix = dict(native['packaging']['wix'])
    source = root / wix['licenseFile']
    target = stage / 'licenses/wix3-MS-RL.txt'
    if target.read_bytes() != source.read_bytes():
        raise ValueError('WiX build-tool notice differs from its reviewed upstream text')
    wix['licenseReferenceVersion'] = wix.pop('version')
    wix['actualSelectedVersion'] = None
    components.append(dict(wix, name='WiX Toolset', shippedCompiler=False))
    return components


def compiler_provenance(data, arch):
    if data.get('schema') != 1 or data.get('architecture') != arch or len(data.get('compilers', [])) != 2:
        raise ValueError('OCR compiler provenance is incomplete')
    if {item.get('language') for item in data['compilers']} != {'C', 'CXX'}:
        raise ValueError('OCR C/C++ compiler evidence is incomplete')
    if not re.fullmatch(r'\d+\.\d+\.\d+', data.get('cmakeVersion', '')) or data.get('cmakeLicenseSource') not in ['https://raw.githubusercontent.com/Kitware/CMake/v' + data['cmakeVersion'] + '/' + name for name in ('LICENSE.rst', 'Copyright.txt')]:
        raise ValueError('Exact CMake source version is unavailable')
    expected = 'x64' if arch == 'x64' else 'ARM64'
    for entry in data['compilers']:
        if entry.get('architecture') != expected or entry.get('visualStudioMajor') not in ('17', '18'):
            raise ValueError('OCR compiler architecture or license edition differs')
        if any(not re.fullmatch(r'\d+(?:\.\d+){2,3}', entry.get(key, '')) for key in ('compilerVersion', 'toolsetVersion', 'windowsSdkVersion')):
            raise ValueError('Exact OCR compiler, toolset or SDK version is unavailable')
        hashes = [entry.get('compilerSha256'), entry.get('windowsSdkLicenseSha256'), entry.get('universalCrtLibrary', {}).get('sha256')]
        libraries = entry.get('toolsetRuntimeLibraries', [])
        if {item['name'] for item in libraries} != {'libcmt.lib', 'libvcruntime.lib', 'libcpmt.lib'}:
            raise ValueError('OCR static runtime provenance is incomplete')
        hashes.extend(item.get('sha256') for item in libraries)
        if any(not isinstance(value, str) or not re.fullmatch('[0-9a-f]{64}', value) for value in hashes):
            raise ValueError('OCR compiler/runtime hash is unavailable')
    return data


def toolchain_inventory(root, stage, arch, texts, sysroot, rust_version, cargo_version):
    components = []

    def include(source, filename, component):
        target = stage / 'licenses' / filename
        target.write_bytes(source.read_bytes())
        content = legal_text(target)
        text_id = digest(content.encode())
        texts.setdefault(text_id, {'text': content, 'sources': set()})['sources'].add('licenses/' + filename)
        component['texts'] = [{'path': 'licenses/' + filename, 'textSha256': text_id}]
        components.append(component)

    version = read_plain_text(rust_version).strip()
    revision = re.search(r'^commit-hash: ([0-9a-f]{40})$', version, re.M)
    release = re.search(r'^release: (.+)$', version, re.M)
    if not revision or not release:
        raise ValueError('The actual rustc -Vv toolchain revision is unavailable')
    include(sysroot / 'share/doc/rust/COPYRIGHT-library.html', 'rust-standard-library.html', {'name': 'Rust standard library', 'version': release[1], 'revision': revision[1], 'source': 'https://github.com/rust-lang/rust/tree/' + revision[1] + '/library', 'license': 'MIT OR Apache-2.0 plus the actual bundled component notices', 'scope': 'Precompiled standard library supplied by the selected Rust toolchain; component-specific full terms retained'})
    include(sysroot / 'share/doc/rust/COPYRIGHT.html', 'rust-build-toolchain.html', {'name': 'Rust compiler', 'version': release[1], 'revision': revision[1], 'source': 'https://github.com/rust-lang/rust/tree/' + revision[1], 'license': 'MIT OR Apache-2.0 plus component-specific full terms', 'scope': 'Build tool only; compiler executables are not distributed'})
    (stage / 'licenses/rust-toolchain.txt').write_text(version + '\n', encoding='utf-8', newline='\n')
    cargo = read_plain_text(cargo_version).strip()
    cargo_revision = re.search(r'^commit-hash: ([0-9a-f]{40})$', cargo, re.M)
    cargo_release = re.search(r'^release: (.+)$', cargo, re.M)
    if not cargo_revision or not cargo_release:
        raise ValueError('The actual cargo -Vv revision is unavailable')
    (stage / 'licenses/cargo-toolchain.txt').write_text(cargo + '\n', encoding='utf-8', newline='\n')
    for name in ('LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE-THIRD-PARTY'):
        include(sysroot / 'share/doc/cargo' / name, 'cargo-' + name + '.txt', {'name': 'Cargo ' + name, 'version': cargo_release[1], 'revision': cargo_revision[1], 'source': 'https://github.com/rust-lang/cargo/tree/' + cargo_revision[1], 'license': 'MIT OR Apache-2.0 with component-specific third-party notices', 'scope': 'Build tool only; Cargo executables are not distributed'})
    include(Path(sys.base_prefix) / 'LICENSE.txt', 'python-build-tool.txt', {'name': 'Python', 'version': platform.python_version(), 'source': 'https://www.python.org/downloads/release/python-' + platform.python_version().replace('.', '') + '/', 'license': 'PSF-2.0 and bundled component terms in the full installed license', 'scope': 'Inventory generator build tool only; Python is not shipped'})
    provenance = compiler_provenance(read_json(stage / 'licenses/ocr-compiler-provenance.json'), arch)
    cmake_license = stage / 'licenses/ocr-cmake.txt'
    if digest(cmake_license.read_bytes()) != provenance.get('cmakeLicenseSha256'):
        raise ValueError('Selected CMake license checksum differs')
    include(cmake_license, 'cmake-build-tool.txt', {'name': 'CMake', 'version': provenance['cmakeVersion'], 'source': provenance['cmakeLicenseSource'], 'license': 'BSD-3-Clause', 'scope': 'OCR build tool only; CMake executable is not distributed'})
    sources = {entry['edition']: entry for entry in read_json(root / 'assets/licenses/microsoft-build-tools-sources.json')}
    for major in sorted({item['visualStudioMajor'] for item in provenance['compilers']}):
        edition = {'17': '2022', '18': '2026'}[major]
        evidence = sources[edition]
        source = root / evidence['licenseFile']
        if digest(source.read_bytes()) != evidence['textSha256']:
            raise ValueError('Reviewed Microsoft build-tool license text differs')
        original = root / evidence['originalDocumentFile']
        if digest(original.read_bytes()) != evidence['documentSha256']:
            raise ValueError('Original Microsoft license document differs')
        document_name = 'microsoft-build-tools-' + edition + '.docx'
        (stage / 'licenses' / document_name).write_bytes(original.read_bytes())
        include(source, 'microsoft-build-tools-' + edition + '.txt', dict(evidence, name='Microsoft Visual Studio Build Tools ' + edition, license='Microsoft proprietary software license terms', scope='Selected OCR compiler and static MSVC runtime; compiler executables are not distributed', compilerProvenance='licenses/ocr-compiler-provenance.json', originalDocument='licenses/' + document_name))
    sdk_license = stage / 'licenses/ocr-windows-sdk-license.rtf'
    for entry in provenance['compilers']:
        if digest(sdk_license.read_bytes()) != entry['windowsSdkLicenseSha256']:
            raise ValueError('Selected Windows SDK license checksum differs')
    include(sdk_license, 'windows-sdk-license.rtf', {'name': 'Windows SDK / Universal CRT', 'versions': sorted({item['windowsSdkVersion'] for item in provenance['compilers']}), 'source': 'https://developer.microsoft.com/windows/downloads/windows-sdk/', 'license': 'Microsoft Windows SDK software license terms', 'scope': 'Selected static Universal CRT for OCR; full license copied from that installed SDK', 'compilerProvenance': 'licenses/ocr-compiler-provenance.json'})
    return components


def generate(root, stage, arch, metadata, target_metadata, sysroot, rust_version, cargo_version):
    packages, texts = rust_inventory(root, metadata, target_metadata)
    components = bundled_inventory(root, stage, arch, texts)
    components.extend(toolchain_inventory(root, stage, arch, texts, sysroot, rust_version, cargo_version))
    fonts = []
    font_licenses = {'Ubuntu-Light.ttf': ('Ubuntu-font-1.0', 'UFL.txt'), 'Hack-Regular.ttf': ('MIT AND Bitstream-Vera', 'Hack-Regular.txt'), 'NotoEmoji-Regular.ttf': ('OFL-1.1', 'OFL.txt'), 'emoji-icon-font.ttf': ('MIT', 'emoji-icon-font-mit-license.txt')}
    for package in metadata['packages']:
        if package['name'] != 'epaint_default_fonts':
            continue
        directory = Path(package['manifest_path']).parent / 'fonts'
        if {path.name for path in directory.glob('*.ttf')} != set(font_licenses):
            raise ValueError('Default font source set differs from the reviewed inventory')
        for filename, (license_expression, notice) in sorted(font_licenses.items()):
            fonts.append({'name': filename, 'version': 'epaint_default_fonts ' + package['version'], 'sha256': digest((directory / filename).read_bytes()), 'license': license_expression, 'noticeSource': 'epaint_default_fonts@' + package['version'] + '/fonts/' + notice, 'scope': 'Desktop default-font source; notices retained even if a font is optimized out of the executable'})
    header = 'SuniPDF complete locked-source license and notice texts\n\nOR denotes alternatives; AND denotes cumulative terms. Legacy slash expressions are preserved exactly as declared.\nThis inventory includes other-platform and development/build dependencies; it is not a claim that every locked crate is linked into a Windows executable.\nRetain applicable copyright, license and NOTICE texts, attribution, font naming and modification notices required by the full terms below. An OR alternative does not impose every alternative.\n\n'
    body = header + '\n'.join(f"{p['name']} {p['version']} | {p['declaredLicense']} | {p['source']}" for p in packages) + '\n'
    for text_id, entry in sorted(texts.items()):
        body += '\n' + '=' * 78 + '\n' + 'SHA256 (LF-normalized text): ' + text_id + '\n' + '\n'.join(sorted(entry['sources'])) + '\n\n' + entry['text'].rstrip('\n') + '\n'
    (stage / TEXTS).write_text(body, encoding='utf-8', newline='\n')
    inventory = {'schema': 1, 'architecture': arch, 'cargoLockSha256': digest((root / 'Cargo.lock').read_bytes()), 'coverage': 'Every Cargo.lock package, with Windows normal/build dependency membership from target-filtered Cargo metadata; all source license expressions retained verbatim', 'licenseRelationships': 'OR = alternative; AND = cumulative; / = legacy declaration retained verbatim. Full terms are included; SPDX identifiers alone are not license evidence.', 'rustPackages': packages, 'nativeAndPackagingComponents': components, 'embeddedFontSources': fonts, 'texts': [{'sha256': key, 'sources': sorted(value['sources'])} for key, value in sorted(texts.items())], 'payloadFiles': payload_map(stage)}
    (stage / INVENTORY).write_text(json.dumps(inventory, indent=2, ensure_ascii=False) + '\n', encoding='utf-8', newline='\n')
    verify(stage)
    return inventory


def verify(stage, archive=None, msi=False):
    if archive:
        with zipfile.ZipFile(archive) as bundle:
            entries = [(name, name.replace('\\', '/')) for name in bundle.namelist() if not name.endswith('/')]
            names = [name for _, name in entries]
            if len(set(name.casefold() for name in names)) != len(names) or any(
                name.startswith('/') or re.search(r'[\x00-\x1f<>:"|?*]', name)
                or any(part in ('', '.', '..') or part.endswith(('.', ' ')) or re.match(r'(?i)^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)', part) for part in name.split('/'))
                for name in names
            ):
                raise ValueError('Unsafe or duplicate package entry')
            data = {normalized: bundle.read(original) for original, normalized in entries}
    else:
        if any(p.is_symlink() for p in stage.rglob('*')):
            raise ValueError('Linked package resource is not allowed')
        data = {p.relative_to(stage).as_posix(): p.read_bytes() for p in stage.rglob('*') if p.is_file()}
    inventory = json.loads(data.pop(INVENTORY).decode('utf-8'))
    expected = inventory['payloadFiles']
    if msi:
        expected = {name: value for name, value in expected.items() if name.startswith('licenses/') or name in ('sunipdf.exe', 'sunipdf-cli.exe', 'pdfium.dll')}
        data.pop('README.txt', None)
    if set(data) != set(expected):
        raise ValueError('Package file set differs from the generated inventory')
    for name, entry in expected.items():
        if len(data[name]) != entry['bytes'] or digest(data[name]) != entry['sha256']:
            raise ValueError('Package payload or notice checksum differs: ' + name)
    return inventory


def record_wix(stage, provenance):
    inventory = verify(stage)
    tools = provenance.get('tools', [])
    if provenance.get('schema') != 1 or {item.get('name') for item in tools} != {'candle.exe', 'light.exe'}:
        raise ValueError('Selected WiX tool evidence is incomplete')
    for tool in tools:
        if not str(tool.get('version', '')).startswith('3.') or not re.fullmatch('[0-9a-f]{64}', tool.get('sha256', '')):
            raise ValueError('Selected WiX version or binary hash is unavailable')
    provenance['scope'] = 'MSI build only; WiX executables are not included'
    provenance['origin'] = 'Actual selected local executable versions and hashes; a hash alone does not attest upstream download origin'
    (stage / 'licenses/wix-build-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n', encoding='utf-8', newline='\n')
    for entry in inventory['nativeAndPackagingComponents']:
        if entry['name'] == 'WiX Toolset':
            entry['actualSelectedVersion'] = sorted({tool['version'] for tool in tools})
            entry['provenance'] = 'licenses/wix-build-provenance.json'
    inventory['payloadFiles'] = payload_map(stage)
    (stage / INVENTORY).write_text(json.dumps(inventory, indent=2, ensure_ascii=False) + '\n', encoding='utf-8', newline='\n')
    return verify(stage)


class InventoryTests(unittest.TestCase):
    def test_platform_membership_excludes_dev_edges(self):
        data = {'workspace_members': ['app'], 'resolve': {'nodes': [{'id': 'app', 'deps': [{'pkg': 'runtime', 'dep_kinds': [{'kind': None}]}, {'pkg': 'dev', 'dep_kinds': [{'kind': 'dev'}]}]}, {'id': 'runtime', 'deps': []}]}}
        self.assertEqual(windows_ids(data), {'app', 'runtime'})

    def test_utf16_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'metadata.json'
            path.write_text('{"name":"日本語"}', encoding='utf-16')
            self.assertEqual(read_json(path)['name'], '日本語')

    def test_payload_verification_rejects_modified_missing_and_added_notices(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            (stage / 'licenses').mkdir()
            path = stage / TEXTS
            path.write_text('Original license text', encoding='utf-8')
            (stage / INVENTORY).write_text(json.dumps({'payloadFiles': payload_map(stage)}))
            verify(stage)
            path.write_text('Changed license text', encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                verify(stage)
            path.unlink()
            with self.assertRaisesRegex(ValueError, 'file set'):
                verify(stage)
            path.write_text('Original license text', encoding='utf-8')
            (stage / 'unexpected.dll').write_bytes(b'extra')
            with self.assertRaisesRegex(ValueError, 'file set'):
                verify(stage)

    def test_zip_duplicate_and_traversal_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ('../escape.txt', 'C:/escape.txt', 'licenses/./license.txt', 'licenses/license.txt ', 'licenses/CON.txt'):
                archive = Path(directory) / 'unsafe.zip'
                with zipfile.ZipFile(archive, 'w') as bundle:
                    bundle.writestr(name, 'unsafe')
                with self.assertRaisesRegex(ValueError, 'Unsafe'):
                    verify(None, archive)
            with zipfile.ZipFile(archive, 'w') as bundle:
                bundle.writestr('licenses/LICENSE.txt', 'original')
                bundle.writestr('licenses/license.txt', 'ambiguous on Windows')
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                verify(None, archive)
            with zipfile.ZipFile(archive, 'w') as bundle:
                bundle.writestr('licenses/license.txt', 'original')
                bundle.writestr('licenses\\license.txt', 'alternate separator duplicate')
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                verify(None, archive)

    def test_full_terms_and_alternative_or_cumulative_declarations_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'pkg').mkdir()
            (root / 'assets/licenses').mkdir(parents=True)
            (root / 'assets/licenses/upstream-sources.json').write_text('{"packages":[]}')
            (root / 'Cargo.lock').write_text('version = 4\n[[package]]\nname = "fixture"\nversion = "1.0.0"\n')
            actual_root = Path(__file__).resolve().parents[1]
            for filename in ('LICENSE-MIT', 'LICENSE-APACHE'):
                (root / filename).write_bytes((actual_root / filename).read_bytes())
            targets = {'workspace_members': ['fixture'], 'resolve': {'nodes': [{'id': 'fixture', 'deps': []}]}}
            package = {'id': 'fixture', 'name': 'fixture', 'version': '1.0.0', 'manifest_path': str(root / 'pkg/Cargo.toml')}
            for expression in ('MIT OR Apache-2.0', 'MIT AND Apache-2.0'):
                package['license'] = expression
                packages, texts = rust_inventory(root, {'packages': [package]}, {'x64': targets, 'arm64': targets})
                self.assertEqual(packages[0]['declaredLicense'], expression)
                self.assertEqual(len(packages[0]['texts']), 2)
                self.assertTrue(any('Permission is hereby granted' in value['text'] for value in texts.values()))
                self.assertTrue(any('TERMS AND CONDITIONS' in value['text'] for value in texts.values()))
            (root / 'LICENSE-MIT').unlink()
            with self.assertRaises(OSError):
                rust_inventory(root, {'packages': [package]}, {'x64': targets, 'arm64': targets})

    def test_missing_or_wrong_architecture_compiler_provenance_rejected(self):
        with self.assertRaisesRegex(ValueError, 'incomplete'):
            compiler_provenance({'schema': 1, 'architecture': 'x64', 'compilers': []}, 'x64')
        with self.assertRaisesRegex(ValueError, 'incomplete'):
            compiler_provenance({'schema': 1, 'architecture': 'arm64', 'compilers': [{}, {}]}, 'x64')

    def test_msi_payload_and_selected_wix_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory)
            (stage / 'licenses').mkdir()
            (stage / 'licenses/full-license.txt').write_text('License retained')
            (stage / 'sunipdf.exe').write_bytes(b'application fixture')
            (stage / 'README.txt').write_text('Portable README')
            (stage / 'USER_GUIDE.en.md').write_text('Portable guide')
            inventory = {'nativeAndPackagingComponents': [{'name': 'WiX Toolset'}], 'payloadFiles': payload_map(stage)}
            (stage / INVENTORY).write_text(json.dumps(inventory))
            tools = {'schema': 1, 'tools': [{'name': name, 'version': '3.14.1.8722', 'sha256': 'a' * 64} for name in ('candle.exe', 'light.exe')]}
            result = record_wix(stage, tools)
            self.assertEqual(result['nativeAndPackagingComponents'][0]['actualSelectedVersion'], ['3.14.1.8722'])
            (stage / 'USER_GUIDE.en.md').unlink()
            (stage / 'README.txt').write_text('Installed MSI README')
            verify(stage, msi=True)
            (stage / 'licenses/full-license.txt').write_text('Changed notice')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                verify(stage, msi=True)

    def test_legal_resource_cannot_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, 'escapes'):
                inside(Path(directory).parent / 'outside.txt', Path(directory))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--stage', type=Path)
    parser.add_argument('--arch', choices=['x64', 'arm64'])
    parser.add_argument('--metadata', type=Path)
    parser.add_argument('--x64-metadata', type=Path)
    parser.add_argument('--arm64-metadata', type=Path)
    parser.add_argument('--rust-sysroot', type=Path)
    parser.add_argument('--rust-version-file', type=Path)
    parser.add_argument('--cargo-version-file', type=Path)
    parser.add_argument('--record-wix', type=Path, help='Add selected WiX evidence to a copy of the verified portable stage for MSI')
    parser.add_argument('--verify', action='store_true')
    parser.add_argument('--archive', type=Path)
    parser.add_argument('--msi', action='store_true', help='Verify an extracted MSI payload; its separate README is excluded')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(InventoryTests))
        return 0 if result.wasSuccessful() else 1
    if args.record_wix:
        if not args.stage:
            parser.error('--record-wix requires --stage')
        result = record_wix(args.stage, read_json(args.record_wix))
    elif args.verify:
        if not args.stage and not args.archive:
            parser.error('--verify requires --stage or --archive')
        result = verify(args.stage, args.archive, args.msi)
    else:
        if not all((args.stage, args.arch, args.metadata, args.x64_metadata, args.arm64_metadata, args.rust_sysroot, args.rust_version_file, args.cargo_version_file)):
            parser.error('Generation requires --stage, --arch, all three metadata inputs, --rust-sysroot, --rust-version-file and --cargo-version-file')
        result = generate(args.root.resolve(), args.stage.resolve(), args.arch, read_json(args.metadata), {'x64': read_json(args.x64_metadata), 'arm64': read_json(args.arm64_metadata)}, args.rust_sysroot, args.rust_version_file, args.cargo_version_file)
    print(f"Verified license inventory: {len(result['rustPackages'])} locked packages, {len(result['texts'])} full texts, {len(result['payloadFiles'])} payload files ({result['architecture']})")
    return 0


if __name__ == '__main__':
    try:
        sys.exit(main())
    except (ValueError, KeyError, OSError, UnicodeError, struct.error, zipfile.BadZipFile, subprocess.CalledProcessError) as error:
        print('License inventory failed: ' + str(error), file=sys.stderr)
        sys.exit(1)
