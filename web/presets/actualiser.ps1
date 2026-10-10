$ErrorActionPreference = 'Stop'
$presetFiles = @(Get-ChildItem -LiteralPath $PSScriptRoot -File | Where-Object { $_.Extension -ieq '.png' } | Sort-Object Name | ForEach-Object { $_.Name })
$presetJson = @{ files = $presetFiles } | ConvertTo-Json -Depth 3
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'manifest.json'), $presetJson, [Text.UTF8Encoding]::new($false))
