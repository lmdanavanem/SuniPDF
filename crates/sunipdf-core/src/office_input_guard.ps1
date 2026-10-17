param([Parameter(Mandatory=$true)][string]$InputPath, [Parameter(Mandatory=$true)][ValidateSet('docx','xlsx','pptx','odt','ods','odp')][string]$Kind)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
$stream = [IO.File]::OpenRead($InputPath)
$archive = $null
try {
    if ($stream.Length -gt 64MB) { throw 'Office input exceeds 64 MiB' }
    $archive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Read, $false)
    if ($archive.Entries.Count -lt 1 -or $archive.Entries.Count -gt 4096) { throw 'Office archive entry limit' }
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    [long]$expanded = 0
    [long]$xmlBytes = 0
    foreach ($entry in $archive.Entries) {
        $name = $entry.FullName
        if ($name.Length -gt 240 -or $name -match '[\\:\x00-\x1f<>|?*"%]' -or $name.StartsWith('/') -or $name -match '(^|/)(\.|\.\.)(/|$)' -or $name.Contains('//') -or -not $names.Add($name)) { throw 'Unsafe or duplicate Office archive path' }
        # Converters must never resolve archive entries as filesystem links.
        if ((($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) { throw 'Office archive links are unsupported' }
        $expanded += $entry.Length
        if ($entry.Length -gt 64MB -or $expanded -gt 256MB -or ($entry.Length -gt 1MB -and $entry.Length -gt [Math]::Max(1, $entry.CompressedLength) * 500)) { throw 'Office archive expansion limit' }
        if ($name -match '(?i)(^|/)(Basic|Scripts|embeddings|externalLinks|queryTables|activeX|macrosheets|dialogs|ObjectReplacements|Objects)(/|$)|vbaProject|\.bin$|(^|/)(connections|vbaData)\.xml$') { throw 'Macros, embedded objects or external data are unsupported' }
        if ($name.EndsWith('/')) { continue }
        if ($name -match '(?i)\.(xml|rels|rdf)$' -or $name -eq 'mimetype') {
            if ($entry.Length -gt 16MB) { throw 'Office XML part exceeds 16 MiB' }
            $xmlBytes += $entry.Length
            if ($xmlBytes -gt 64MB) { throw 'Office XML total exceeds 64 MiB' }
        } elseif ($name -notmatch '(?i)\.(png|jpe?g)$') { throw 'Unsupported Office package part; only XML and PNG/JPEG images are accepted' }
    }
    $odf = $Kind -in @('odt','ods','odp')
    $main = switch ($Kind) { 'docx' {'word/document.xml'} 'xlsx' {'xl/workbook.xml'} 'pptx' {'ppt/presentation.xml'} default {'content.xml'} }
    if (-not $names.Contains($main) -or (-not $odf -and -not $names.Contains('[Content_Types].xml'))) { throw 'Office package does not match its extension' }
    if ($odf) {
        $entry = $archive.GetEntry('mimetype')
        if ($null -eq $entry -or $entry.Length -gt 128) { throw 'Missing ODF media type' }
        $reader = [IO.StreamReader]::new($entry.Open())
        try { $mime = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $wanted = switch ($Kind) { 'odt' {'text'} 'ods' {'spreadsheet'} 'odp' {'presentation'} }
        if ($mime -cne "application/vnd.oasis.opendocument.$wanted") { throw 'ODF media type differs from its extension' }
    }
    foreach ($entry in $archive.Entries) {
        if ($entry.FullName -match '(?i)\.(png|jpe?g)$') {
            $image = $entry.Open()
            try {
                $header = [byte[]]::new(8)
                $count = $image.Read($header, 0, 8)
                $png = $count -eq 8 -and [BitConverter]::ToString($header) -eq '89-50-4E-47-0D-0A-1A-0A'
                $jpeg = $count -ge 3 -and $header[0] -eq 255 -and $header[1] -eq 216 -and $header[2] -eq 255
                if (-not $png -and -not $jpeg) { throw 'Unsupported or disguised Office image' }
                [long]$read = $count
                $buffer = [byte[]]::new(65536)
                while (($count = $image.Read($buffer,0,$buffer.Length)) -gt 0) { $read += $count; if ($read -gt 64MB) { throw 'Office image expansion limit' } }
                if ($read -ne $entry.Length) { throw 'Office image size mismatch' }
            } finally { $image.Dispose() }
            continue
        }
        if ($entry.FullName -notmatch '(?i)\.(xml|rels|rdf)$') { continue }
        $settings = [Xml.XmlReaderSettings]::new()
        $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
        $settings.XmlResolver = $null
        $settings.MaxCharactersInDocument = 16MB
        $settings.CloseInput = $true
        $reader = [Xml.XmlReader]::Create($entry.Open(), $settings)
        try {
            while ($reader.Read()) {
                if ($reader.NodeType -ne [Xml.XmlNodeType]::Element) { continue }
                $local = $reader.LocalName
                $ns = $reader.NamespaceURI
                if ($local -match '^(instrText|fldSimple|fldChar|altChunk|oleObject|control|externalData|attachedTemplate|dataBinding|DDE|ddeLink|dde-links|dde-connection|event-listener|script|scripts|object|object-ole|f|formula|annotation|annotation-end|encryption-data)$' -or $ns -match 'script|xml-events') { throw 'Active fields, formulas, scripts, encryption or embedded objects are unsupported' }
                if ($local -eq 'Relationship') {
                    $target = $reader.GetAttribute('Target')
                    $type = $reader.GetAttribute('Type')
                    if ($reader.GetAttribute('TargetMode') -eq 'External' -or $type -match '(?i)macro|vba|oleObject|externalLink|attachedTemplate|aFChunk|control|connection|hyperlink') { throw 'External or active Office relationship is unsupported' }
                    if ([string]::IsNullOrEmpty($target) -or $target -match '[\\:\x00-\x1f%?#]' -or $target.StartsWith('/')) { throw 'Unsafe Office relationship target' }
                    $base = ''
                    if ($entry.FullName -ne '_rels/.rels') {
                        if ($entry.FullName -notmatch '^(.*/)?_rels/([^/]+)\.rels$') { throw 'Invalid Office relationship location' }
                        $base = $Matches[1]
                    }
                    $segments = [Collections.Generic.List[string]]::new()
                    foreach ($segment in (($base + $target) -split '/')) {
                        if ($segment -eq '..') { if ($segments.Count -eq 0) { throw 'Relationship escapes Office package' }; $segments.RemoveAt($segments.Count - 1) }
                        elseif ($segment -ne '.' -and $segment -ne '') { $segments.Add($segment) }
                    }
                    if (-not $names.Contains([string]::Join('/', $segments))) { throw 'Missing Office relationship target' }
                }
                if ($reader.HasAttributes) {
                    while ($reader.MoveToNextAttribute()) {
                        if ($reader.LocalName -in @('formula','update-mode','code','macro-name') -or $reader.NamespaceURI -match 'script|xml-events') { throw 'Active Office attribute is unsupported' }
                        if ($reader.LocalName -eq 'ContentType' -and $reader.Value -match '(?i)macro|vba|activeX|oleObject') { throw 'Active Office content type is unsupported' }
                        if ($reader.LocalName -eq 'href') {
                            $href = $reader.Value
                            if ($href.StartsWith('#')) { continue }
                            if ($href -notmatch '^Pictures/[A-Za-z0-9_. -]+\.(png|jpe?g)$' -or -not $names.Contains($href)) { throw 'External or unsupported ODF link' }
                        }
                        if ($reader.LocalName -in @('src','url','onload','onclick')) { throw 'External or active Office attribute is unsupported' }
                    }
                    [void]$reader.MoveToElement()
                }
            }
        } finally { $reader.Dispose() }
    }
    [Console]::Out.Write('SAFE')
} catch {
    [Console]::Error.Write($_.Exception.Message)
    exit 1
} finally {
    if ($null -ne $archive) { $archive.Dispose() }
    $stream.Dispose()
}
