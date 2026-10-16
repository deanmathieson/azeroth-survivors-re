<#
.SYNOPSIS
    Packs an already-built warcraft-survivors.exe into a "launch night kit" zip for friends.

.DESCRIPTION
    Builds nothing. Takes target/play/warcraft-survivors.exe (build it first with
    `cargo build --profile play -p warcraft-survivors`) and writes
    dist/AzerothSurvivors-<version>.zip holding:
        AzerothSurvivors-<version>/
            warcraft-survivors.exe
            Play.bat, Host.bat, Join.bat, Update.bat, _launch.cmd, _update.ps1
            README-FRIENDS.txt
            version.txt
            benilla-config/README.txt                   (an empty profile: no saves)
    The .bat files set BENILLA_HOME to the kit's own benilla-config, so the profile stays beside
    the exe wherever the zip is unzipped. No Blizzard data is ever packed: the script refuses to
    finish if any .MPQ (or a Data/WoW folder) would end up in the zip.

.PARAMETER Exe
    The built exe. Default: <repo>/target/play/warcraft-survivors.exe.

.PARAMETER OutDir
    Where the zip goes. Default: <repo>/dist.

.PARAMETER Tag
    Version written into version.txt and used in the versioned zip name. Defaults to the tag at
    HEAD (or GITHUB_REF_NAME for a tag build), then the git short hash.

.PARAMETER Force
    Replace an existing versioned zip. Use this in CI so packaging never prompts.

.EXAMPLE
    pwsh scripts/package-friends.ps1
    pwsh scripts/package-friends.ps1 -Exe "D:\other worktree\target\play\warcraft-survivors.exe"
#>
[CmdletBinding()]
param(
    [string]$Exe,
    [string]$OutDir,
    [string]$Tag,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$KitSrc = Join-Path $PSScriptRoot 'friends-kit'
if (-not $Exe) { $Exe = Join-Path $Root 'target/play/warcraft-survivors.exe' }
if (-not $OutDir) { $OutDir = Join-Path $Root 'dist' }

if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
    throw "No exe at '$Exe'. Build it first: cargo build --profile play -p warcraft-survivors (or pass -Exe)."
}
$Exe = (Resolve-Path -LiteralPath $Exe).Path

$hash = (& git -C $Root rev-parse --short HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or -not $hash) { throw "git rev-parse failed in '$Root'." }
if (-not $Tag -and $env:GITHUB_REF_TYPE -eq 'tag' -and $env:GITHUB_REF_NAME) {
    $Tag = $env:GITHUB_REF_NAME
}
if (-not $Tag) {
    $headTag = @(& git -C $Root tag --points-at HEAD | Where-Object { $_ -match '^v' } | Select-Object -First 1)
    if ($headTag) { $Tag = $headTag[0].Trim() }
}
if (-not $Tag) { $Tag = $hash }
if ($Tag -match '[\\/:*?"<>|]') { throw "Tag '$Tag' cannot be used in a kit file name." }
$dirty = & git -C $Root status --porcelain --untracked-files=no
if ($dirty) {
    Write-Warning "The working tree has uncommitted changes: the zip is named after HEAD ($hash), which may not match what the exe was built from."
}
$exeAge = (Get-Item -LiteralPath $Exe).LastWriteTime
$commitAge = [DateTimeOffset]::FromUnixTimeSeconds([long](& git -C $Root log -1 --format=%ct)).LocalDateTime
if ($exeAge -lt $commitAge) {
    Write-Warning "The exe ($exeAge) is older than HEAD ($commitAge). Rebuild if HEAD changed the game."
}

$name = "AzerothSurvivors-$Tag"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutDir = (Resolve-Path -LiteralPath $OutDir).Path
$zip = Join-Path $OutDir "$name.zip"
$stableZip = Join-Path $OutDir 'AzerothSurvivors.zip'
$stageRoot = Join-Path $OutDir '_stage'
$stage = Join-Path $stageRoot $name
if ((Test-Path -LiteralPath $zip) -and -not $Force) {
    throw "'$zip' already exists. Pass -Force to replace it."
}
if (Test-Path -LiteralPath $stageRoot) { Remove-Item -LiteralPath $stageRoot -Recurse -Force }
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'benilla-config') | Out-Null

# Text files go in with CRLF line endings: cmd.exe mis-parses labels in LF-only batch files.
function Copy-Crlf([string]$From, [string]$To) {
    $text = [IO.File]::ReadAllText($From)
    $text = ($text -replace "`r`n", "`n") -replace "`n", "`r`n"
    [IO.File]::WriteAllText($To, $text, [Text.Encoding]::ASCII)
}

Copy-Item -LiteralPath $Exe -Destination (Join-Path $stage 'warcraft-survivors.exe')
foreach ($f in 'Play.bat', 'Host.bat', 'Join.bat', 'Update.bat', '_launch.cmd', '_update.ps1', 'README-FRIENDS.txt') {
    Copy-Crlf (Join-Path $KitSrc $f) (Join-Path $stage $f)
}
Copy-Crlf (Join-Path $KitSrc 'benilla-config-README.txt') (Join-Path $stage 'benilla-config/README.txt')
[IO.File]::WriteAllText((Join-Path $stage 'version.txt'), "$Tag`r`n", [Text.Encoding]::ASCII)

# Guard: never ship Blizzard data.
$bad = Get-ChildItem -LiteralPath $stage -Recurse -Force |
    Where-Object { $_.Extension -ieq '.mpq' -or ($_.PSIsContainer -and $_.Name -in 'Data', 'WoW') }
if ($bad) { throw "Refusing to pack Blizzard data: $($bad.FullName -join ', ')" }

if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
Add-Type -AssemblyName System.IO.Compression.FileSystem
[IO.Compression.ZipFile]::CreateFromDirectory($stageRoot, $zip, [IO.Compression.CompressionLevel]::Optimal, $false)
Remove-Item -LiteralPath $stageRoot -Recurse -Force

# Check what actually went in.
$archive = [IO.Compression.ZipFile]::OpenRead($zip)
try {
    $entries = @($archive.Entries | ForEach-Object { $_.FullName })
} finally {
    $archive.Dispose()
}
if ($entries | Where-Object { $_ -match '(?i)\.mpq$' }) {
    Remove-Item -LiteralPath $zip -Force
    throw 'An .MPQ ended up in the zip; deleted it.'
}

Copy-Item -LiteralPath $zip -Destination $stableZip -Force

$mb = [math]::Round((Get-Item -LiteralPath $zip).Length / 1MB, 1)
Write-Host "Wrote $zip ($mb MB):"
$entries | ForEach-Object { Write-Host "  $_" }
Write-Host "Wrote $stableZip"
Write-Host 'Everyone in the party needs this exact zip.'
