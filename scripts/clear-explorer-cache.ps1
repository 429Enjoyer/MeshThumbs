param(
    [switch]$NoRestartExplorer
)

$ErrorActionPreference = "SilentlyContinue"

function Remove-CacheFiles {
    param([string[]]$Paths)

    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path) {
            Get-ChildItem -LiteralPath $path -Force |
                Where-Object { $_.Name -match '^(thumbcache|iconcache).*\.db$' } |
                Remove-Item -Force
        }
    }
}

$explorerCacheDir = Join-Path $env:LOCALAPPDATA "Microsoft\Windows\Explorer"
$legacyIconCache = Join-Path $env:LOCALAPPDATA "IconCache.db"
$thumbHandler = "{e357fccd-a995-4576-b01f-234630154e96}"
$providerClsids = @(
    "{4C6F2B8A-5D2E-4C64-9AC7-B6FD046A8241}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00001}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00002}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00003}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00004}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00005}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00006}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00007}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00008}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00009}",
    "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D0000A}"
)

function Remove-ThumbnailKeyIfOurs {
    param([string]$Path)

    $value = (Get-ItemProperty -LiteralPath $Path -ErrorAction SilentlyContinue)."(default)"
    if ($providerClsids -contains $value) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Remove-CurrentUserShellOverrides {
    foreach ($ext in ".obj", ".fbx", ".glb", ".gltf", ".stl", ".dae", ".ply", ".x3d", ".x3db", ".3ds", ".model") {
        Remove-ThumbnailKeyIfOurs "HKCU:\Software\Classes\$ext\shellex\$thumbHandler"
        Remove-ThumbnailKeyIfOurs "HKCU:\Software\Classes\SystemFileAssociations\$ext\shellex\$thumbHandler"

        $userChoice = Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$ext\UserChoice" -ErrorAction SilentlyContinue
        if ($userChoice.ProgId) {
            Remove-ThumbnailKeyIfOurs "HKCU:\Software\Classes\$($userChoice.ProgId)\shellex\$thumbHandler"
        }
    }

    foreach ($clsid in $providerClsids) {
        Remove-Item -LiteralPath "HKCU:\Software\Classes\CLSID\$clsid" -Recurse -Force
    }
}

Write-Host "Stopping Explorer thumbnail hosts..."
Get-Process explorer,dllhost -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800

Write-Host "Removing stale current-user shell overrides..."
Remove-CurrentUserShellOverrides

Write-Host "Clearing Explorer thumbnail and icon cache databases..."
Remove-CacheFiles @($explorerCacheDir)
Remove-Item -LiteralPath $legacyIconCache -Force

$ie4uinit = Join-Path $env:WINDIR "System32\ie4uinit.exe"
if (Test-Path -LiteralPath $ie4uinit) {
    Write-Host "Asking Windows to refresh icon cache..."
    & $ie4uinit -ClearIconCache | Out-Null
    & $ie4uinit -show | Out-Null
}

if (-not $NoRestartExplorer) {
    Write-Host "Starting Explorer..."
    Start-Process explorer.exe
}

Write-Host "Done. Reopen the model folder and switch the view size once if Explorer still shows stale thumbnails."
