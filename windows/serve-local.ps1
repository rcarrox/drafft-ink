param(
    [string]$Root,
    [int]$Port = 8765
)

$ErrorActionPreference = 'Stop'

if (-not $Root) {
    $Root = Join-Path (Split-Path -Parent $PSScriptRoot) 'web'
}
$Root = [System.IO.Path]::GetFullPath($Root)
if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
    throw "Web root not found: $Root"
}

# Refresh the PNG list before serving this installation.
$presetScript = Join-Path $Root 'presets/actualiser.ps1'
if (Test-Path -LiteralPath $presetScript) { & $presetScript }

$pidFile = Join-Path (Split-Path -Parent $PSScriptRoot) '.drafftink-server.pid'
[System.IO.File]::WriteAllText($pidFile, [string]$PID, [System.Text.Encoding]::ASCII)

function Get-ContentType([string]$Path) {
    switch ([System.IO.Path]::GetExtension($Path).ToLowerInvariant()) {
        '.html' { return 'text/html; charset=utf-8' }
        '.js'   { return 'text/javascript; charset=utf-8' }
        '.mjs'  { return 'text/javascript; charset=utf-8' }
        '.wasm' { return 'application/wasm' }
        '.css'  { return 'text/css; charset=utf-8' }
        '.svg'  { return 'image/svg+xml' }
        '.png'  { return 'image/png' }
        '.jpg'  { return 'image/jpeg' }
        '.jpeg' { return 'image/jpeg' }
        '.webp' { return 'image/webp' }
        '.json' { return 'application/json; charset=utf-8' }
        default { return 'application/octet-stream' }
    }
}

function Write-Response($Stream, [int]$Status, [string]$StatusText, [byte[]]$Body, [string]$ContentType) {
    $header = "HTTP/1.1 $Status $StatusText`r`n" +
              "Content-Type: $ContentType`r`n" +
              "Content-Length: $($Body.Length)`r`n" +
              "Cache-Control: no-cache`r`n" +
              "Connection: close`r`n`r`n"
    $headerBytes = [System.Text.Encoding]::ASCII.GetBytes($header)
    $Stream.Write($headerBytes, 0, $headerBytes.Length)
    if ($Body.Length -gt 0) {
        $Stream.Write($Body, 0, $Body.Length)
    }
    $Stream.Flush()
}

# Font bytes are served only by this existing loopback listener. Registry paths
# are restricted to the two Windows font directories, never arbitrary files.
$script:LocalFontFiles = @{}
function Get-LocalFontCatalog {
    $script:LocalFontFiles.Clear()
    $records = New-Object 'System.Collections.Generic.List[object]'
    $roots = @()
    if ($env:WINDIR) { $roots += [System.IO.Path]::GetFullPath((Join-Path $env:WINDIR 'Fonts')) }
    if ($env:LOCALAPPDATA) { $roots += [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\Fonts')) }
    foreach ($registryPath in @('HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts', 'HKCU:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts')) {
        $key = Get-Item -LiteralPath $registryPath -ErrorAction SilentlyContinue
        if (-not $key) { continue }
        foreach ($displayName in $key.GetValueNames()) {
            $registered = $key.GetValue($displayName)
            if ($registered -isnot [string] -or [string]::IsNullOrWhiteSpace($registered)) { continue }
            $candidates = @()
            if ([System.IO.Path]::IsPathRooted($registered)) { $candidates += $registered }
            else { foreach ($fontRoot in $roots) { $candidates += Join-Path $fontRoot $registered } }
            foreach ($candidate in $candidates) {
                try { $fontPath = [System.IO.Path]::GetFullPath($candidate) } catch { continue }
                $allowed = $false
                foreach ($fontRoot in $roots) {
                    $prefix = $fontRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
                    if ($fontPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) { $allowed = $true; break }
                }
                if (-not $allowed -or -not (Test-Path -LiteralPath $fontPath -PathType Leaf)) { continue }
                if ([System.IO.Path]::GetExtension($fontPath).ToLowerInvariant() -notin @('.ttf','.otf','.ttc')) { continue }
                $identifier = [System.IO.Path]::GetFileNameWithoutExtension($fontPath)
                $family = $displayName -replace '\s*\((TrueType|OpenType)\)\s*$', ''
                $script:LocalFontFiles[$identifier] = $fontPath
                $records.Add([pscustomobject][ordered]@{ family = $family; fullName = $family; postscriptName = $identifier; style = ''; source = 'local-server' })
                break
            }
        }
        $key.Close()
    }
    return @($records.ToArray() | Sort-Object family,postscriptName -Unique)
}

$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
try {
    $listener.Start()
    while ($true) {
        $client = $listener.AcceptTcpClient()
        try {
            $client.ReceiveTimeout = 5000
            $client.SendTimeout = 5000
            $stream = $client.GetStream()
            $reader = New-Object System.IO.StreamReader($stream, [System.Text.Encoding]::ASCII, $false, 4096, $true)
            $requestLine = $reader.ReadLine()
            if ([string]::IsNullOrWhiteSpace($requestLine)) {
                continue
            }
            do { $line = $reader.ReadLine() } while ($null -ne $line -and $line.Length -gt 0)

            $parts = $requestLine.Split(' ')
            if ($parts.Length -lt 2 -or ($parts[0] -ne 'GET' -and $parts[0] -ne 'HEAD')) {
                $body = [System.Text.Encoding]::UTF8.GetBytes('Method not allowed')
                Write-Response $stream 405 'Method Not Allowed' $body 'text/plain; charset=utf-8'
                continue
            }

            $target = $parts[1].Split('?')[0]
            if ($target -eq '/local-fonts.json') {
                $catalog = @(Get-LocalFontCatalog)
                $json = ConvertTo-Json -InputObject $catalog -Depth 4 -Compress
                $body = [System.Text.Encoding]::UTF8.GetBytes($json)
                Write-Response $stream 200 'OK' $body 'application/json; charset=utf-8'
                continue
            }
            if ($target.StartsWith('/local-font/', [System.StringComparison]::Ordinal)) {
                $identifier = [System.Uri]::UnescapeDataString($target.Substring('/local-font/'.Length))
                if ($script:LocalFontFiles.Count -eq 0) { $null = Get-LocalFontCatalog }
                if (-not $script:LocalFontFiles.ContainsKey($identifier)) {
                    Write-Response $stream 404 'Not Found' ([System.Text.Encoding]::UTF8.GetBytes('Font not found')) 'text/plain; charset=utf-8'
                    continue
                }
                $fontPath = $script:LocalFontFiles[$identifier]
                $body = if ($parts[0] -eq 'HEAD') { [byte[]]::new(0) } else { [System.IO.File]::ReadAllBytes($fontPath) }
                Write-Response $stream 200 'OK' $body 'application/octet-stream'
                continue
            }

            $relative = [System.Uri]::UnescapeDataString($target).TrimStart('/').Replace('/', [System.IO.Path]::DirectorySeparatorChar)
            if ([string]::IsNullOrWhiteSpace($relative)) { $relative = 'index.html' }

            $fullPath = [System.IO.Path]::GetFullPath((Join-Path $Root $relative))
            $rootPrefix = $Root.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
            if (-not $fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
                $body = [System.Text.Encoding]::UTF8.GetBytes('Forbidden')
                Write-Response $stream 403 'Forbidden' $body 'text/plain; charset=utf-8'
                continue
            }

            if (Test-Path -LiteralPath $fullPath -PathType Container) {
                $fullPath = Join-Path $fullPath 'index.html'
            }
            if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
                $body = [System.Text.Encoding]::UTF8.GetBytes('Not found')
                Write-Response $stream 404 'Not Found' $body 'text/plain; charset=utf-8'
                continue
            }

            if ($parts[0] -eq 'HEAD') {
                $body = [byte[]]::new(0)
            } else {
                $body = [System.IO.File]::ReadAllBytes($fullPath)
            }
            Write-Response $stream 200 'OK' $body (Get-ContentType $fullPath)
        } catch {
            try {
                $body = [System.Text.Encoding]::UTF8.GetBytes('Server error')
                Write-Response $stream 500 'Internal Server Error' $body 'text/plain; charset=utf-8'
            } catch {}
        } finally {
            $client.Close()
        }
    }
} finally {
    $listener.Stop()
    Remove-Item -LiteralPath $pidFile -Force -ErrorAction SilentlyContinue
}

