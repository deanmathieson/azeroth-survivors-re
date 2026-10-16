<#
  Best-effort updater for the friends kit.  It is dot-sourced by _launch.cmd, but can also be
  dry-run locally with a saved GitHub release response:
    . .\_update.ps1
    Invoke-AzerothSurvivorsUpdate -KitPath . -ReleaseJsonPath .\release.json -DryRun
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest

function Test-AzerothSurvivorsNewerTag {
    param([string]$Current, [string]$Latest)

    if (-not $Latest -or $Current -eq $Latest) { return $false }
    $currentVersion = [version]::new()
    $latestVersion = [version]::new()
    $hasCurrentVersion = [version]::TryParse($Current.TrimStart('v'), [ref]$currentVersion)
    $hasLatestVersion = [version]::TryParse($Latest.TrimStart('v'), [ref]$latestVersion)
    if ($hasCurrentVersion -and $hasLatestVersion) { return $latestVersion -gt $currentVersion }

    # Development kits contain a short hash, so any tagged release is newer than one of them.
    return $Latest -match '^v\d+'
}

function Invoke-AzerothSurvivorsUpdate {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][string]$KitPath,
        [string]$ReleaseApiUrl = 'https://api.github.com/repos/deanmathieson/azeroth-survivors-re/releases/latest',
        [string]$ReleaseJsonPath,
        [switch]$DryRun
    )

    $ErrorActionPreference = 'Stop'
    try {
        $KitPath = (Resolve-Path -LiteralPath $KitPath -ErrorAction Stop).Path
        $versionPath = Join-Path $KitPath 'version.txt'
        $current = if (Test-Path -LiteralPath $versionPath -PathType Leaf) {
            ([IO.File]::ReadAllText($versionPath)).Trim()
        } else {
            'unknown'
        }

        if ($ReleaseJsonPath) {
            $release = Get-Content -LiteralPath $ReleaseJsonPath -Raw | ConvertFrom-Json
        } else {
            $release = Invoke-RestMethod -Uri $ReleaseApiUrl -Headers @{ 'User-Agent' = 'AzerothSurvivors-FriendsKit' } -TimeoutSec 5
        }
        $latest = [string]$release.tag_name
        if (-not (Test-AzerothSurvivorsNewerTag -Current $current -Latest $latest)) { return $false }

        $asset = @($release.assets | Where-Object { $_.name -eq 'AzerothSurvivors.zip' } | Select-Object -First 1)[0]
        if (-not $asset -or -not $asset.browser_download_url) { throw "release $latest has no AzerothSurvivors.zip" }

        Write-Host "Updating Azeroth Survivors $current -> $latest..."
        if ($DryRun) { return $true }

        $work = Join-Path ([IO.Path]::GetTempPath()) ("AzerothSurvivors-update-" + [guid]::NewGuid().ToString('N'))
        $zip = Join-Path $work 'AzerothSurvivors.zip'
        $unpacked = Join-Path $work 'unpacked'
        try {
            New-Item -ItemType Directory -Force -Path $unpacked | Out-Null
            Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -Headers @{ 'User-Agent' = 'AzerothSurvivors-FriendsKit' } -TimeoutSec 30
            Expand-Archive -LiteralPath $zip -DestinationPath $unpacked -Force
            $source = @(Get-ChildItem -LiteralPath $unpacked -Directory | Where-Object {
                Test-Path -LiteralPath (Join-Path $_.FullName 'warcraft-survivors.exe') -PathType Leaf
            } | Select-Object -First 1)[0]
            if (-not $source) {
                throw 'downloaded archive is not an Azeroth Survivors kit'
            }

            # Do not copy the data path, last host, name files, or benilla-config: those are local state.
            $payload = 'warcraft-survivors.exe', 'README-FRIENDS.txt', 'version.txt', '_update.ps1',
                'Play.bat', 'Host.bat', 'Join.bat', 'Update.bat', '_launch.cmd'
            foreach ($file in $payload) {
                if (-not (Test-Path -LiteralPath (Join-Path $source.FullName $file) -PathType Leaf)) {
                    throw "downloaded archive is missing $file"
                }
            }
            foreach ($file in 'warcraft-survivors.exe', 'README-FRIENDS.txt', 'version.txt', '_update.ps1') {
                Copy-Item -LiteralPath (Join-Path $source.FullName $file) -Destination (Join-Path $KitPath $file) -Force
            }
            # Batch launchers are copied last, so the command file currently running stays valid.
            foreach ($file in 'Play.bat', 'Host.bat', 'Join.bat', 'Update.bat', '_launch.cmd') {
                Copy-Item -LiteralPath (Join-Path $source.FullName $file) -Destination (Join-Path $KitPath $file) -Force
            }
        } finally {
            if (Test-Path -LiteralPath $work) { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue }
        }
        return $true
    } catch {
        Write-Host 'Update check failed; using the current version.'
        return $false
    }
}
