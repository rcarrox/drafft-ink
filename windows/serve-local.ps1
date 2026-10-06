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
