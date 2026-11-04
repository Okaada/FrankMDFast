# Authenticode-sign files with the PFX in WINDOWS_CERTIFICATE (base64) and
# WINDOWS_CERTIFICATE_PASSWORD. Without them, prints a notice and does nothing.
#   ./packaging/windows/sign.ps1 file1.exe [file2.exe ...]
param([Parameter(Mandatory, ValueFromRemainingArguments)][string[]]$Files)
$ErrorActionPreference = 'Stop'

if (-not $env:WINDOWS_CERTIFICATE) {
    Write-Host "::notice::WINDOWS_CERTIFICATE is not set; $($Files -join ', ') left unsigned."
    exit 0
}

$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" |
    Sort-Object FullName | Select-Object -Last 1
if (-not $signtool) { throw 'signtool.exe not found (Windows SDK missing)' }

$pfx = Join-Path $env:RUNNER_TEMP 'frankmd-signing.pfx'
try {
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERTIFICATE))
    foreach ($file in $Files) {
        & $signtool.FullName sign /f $pfx /p $env:WINDOWS_CERTIFICATE_PASSWORD `
            /fd sha256 /tr http://timestamp.digicert.com /td sha256 $file
        if ($LASTEXITCODE -ne 0) { throw "signtool failed for $file" }
    }
} finally {
    Remove-Item -Force -ErrorAction SilentlyContinue $pfx
}
