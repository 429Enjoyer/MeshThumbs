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
    "{A9FFD4C4-3FA9-4EB7-8B47-B89A7F09D059}",
    "{4E5FD91F-C018-4850-9636-3069629C6D3D}",
    "{E859325C-5506-4419-8AC5-6A4B03F3A138}",
    "{B7265976-0DBA-44B5-9303-0B0DAFD034E0}",
    "{A3BAFD17-52CD-4CF6-869E-A4BB020591EF}",
    "{7BF654CD-6B62-4A1C-BE5F-53DF447C2BE6}",
    "{AB2CDE52-5C15-4DAF-B43A-E4C9F1EAAEC0}",
    "{0AD51061-9A3C-4EC3-9757-874ECB89457C}",
    "{35A24A7A-CC90-48B3-9849-548DDAA06B01}",
    "{4C451BA6-CC4C-47FB-8F5F-3D32029E2F45}",
    "{AF3D3DCD-8C60-4E29-A483-46DB39C40FB9}",
    "{098B9E30-DA6D-43B8-9C2D-F82D32B4B070}",
    "{F0C794C3-D2B3-42D4-A861-966CED3C966B}",
    "{61B9231B-1E4B-4A1B-ADFB-812E3AE8A6BB}",
    "{3EC714F0-B74B-477C-B682-8B74AEE7C61A}",
    "{125A17BA-7451-4726-BD0B-738E93ADD02F}",
    "{EEBDC2F2-E564-476C-AD42-C8F35F8997F4}",
    "{991F57D3-64BA-42C9-A65C-19530FF5C176}",
    "{D84B7C81-992B-480A-9108-17321DCF1410}"
)

function Remove-ThumbnailKeyIfOurs {
    param([string]$Path)

    $value = (Get-ItemProperty -LiteralPath $Path -ErrorAction SilentlyContinue)."(default)"
    if ($providerClsids -contains $value) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Remove-CurrentUserShellOverrides {
    foreach ($ext in ".obj", ".fbx", ".glb", ".gltf", ".stl", ".dae", ".ply", ".3ds", ".3mf", ".vrm", ".blend", ".x3d", ".off", ".usd", ".usda", ".usdc", ".usdz", ".wrl", ".vrml") {
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
