param(
    [switch]$NoRestartExplorer,
    # MSI handles locked files and any required reboot. Its refresh must not
    # kill Explorer, delete open cache files, or start another window.
    [switch]$RefreshOnly
)

$ErrorActionPreference = "SilentlyContinue"

function Send-ShellAssociationChange {
    if (-not ("MeshThumbs.ShellNotification" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace MeshThumbs {
    public static class ShellNotification {
        [DllImport("shell32.dll")]
        private static extern void SHChangeNotify(int eventId, uint flags, IntPtr item1, IntPtr item2);

        public static void AssociationsChanged() {
            // SHCNE_ASSOCCHANGED, SHCNF_IDLIST | SHCNF_FLUSHNOWAIT:
            // start delivering cache/handler invalidation before exiting,
            // without waiting on a busy Explorer window. No Explorer launch.
            SHChangeNotify(0x08000000, 0x3000, IntPtr.Zero, IntPtr.Zero);
        }
    }
}
'@ -ErrorAction Stop
    }
    [MeshThumbs.ShellNotification]::AssociationsChanged()
}

function Get-CurrentSessionExplorer {
    $SessionId = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
    Get-Process -Name explorer -ErrorAction SilentlyContinue |
        Where-Object { $_.SessionId -eq $SessionId }
}

function Restore-ExplorerIfNeeded {
    # Winlogon normally brings the desktop back automatically. Give it time,
    # then launch only if no Explorer process exists in this user's session.
    for ($Attempt = 0; $Attempt -lt 20; $Attempt++) {
        if (Get-CurrentSessionExplorer) { return }
        Start-Sleep -Milliseconds 250
    }
    if (-not (Get-CurrentSessionExplorer)) {
        Start-Process -FilePath (Join-Path $env:WINDIR "explorer.exe") -WindowStyle Hidden
    }
}

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
    "{D84B7C81-992B-480A-9108-17321DCF1410}",
    "{4C93500D-0E9B-46EC-9E14-831C72A0B34C}",
    "{EFE86458-F28C-43FE-BD01-FD661C662344}",
    "{C20FDBFB-A119-402B-9DF4-87768E6FB3AB}",
    "{3604F66F-141E-4741-BB85-6B5D1853C553}",
    "{C370DCC3-81F8-4B5A-9562-957D3588222B}",
    "{5EABC4CA-0201-41A3-B6BC-F325AEFB5030}",
    "{422DE617-9A3A-41AE-96B9-19E0ED555129}",
    "{C0036F44-536A-4952-9F4C-9F088FFB5241}",
    "{DBD5A742-3E62-4AA6-88B8-8ABEA8DD3B27}",
    "{8B08C226-6E4B-4D15-9629-A724BC4B753B}",
    "{07064A7A-86C6-43A2-86E8-E389CE69F157}",
    "{DDA72EDB-1092-43CF-B3D6-5C96A5151477}",
    "{4F1C159E-6B46-4CC9-B992-44DC908EC1FC}",
    "{4BAC9304-92F2-4177-82ED-FB4A9989CAAA}",
    "{E36A4A8D-C884-4E7A-A5D2-3571197FC4F7}",
    "{0ADA8766-AA7F-4E6D-95D1-D1161DD7F870}",
    "{F1CC9814-4F31-40FF-B825-F485D096268C}",
    "{7C584B0C-BFEB-401C-9BCE-0FED8C228F70}"
)

function Remove-ThumbnailKeyIfOurs {
    param([string]$Path)

    $value = (Get-ItemProperty -LiteralPath $Path -ErrorAction SilentlyContinue)."(default)"
    if ($providerClsids -contains $value) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Remove-CurrentUserShellOverrides {
    foreach ($ext in ".obj", ".fbx", ".glb", ".gltf", ".stl", ".dae", ".ply", ".3ds", ".3mf", ".vrm", ".blend", ".x3d", ".off", ".usd", ".usda", ".usdc", ".usdz", ".wrl", ".vrml", ".step", ".stp", ".abc", ".igs", ".iges", ".3dm", ".ifc", ".pmx", ".vox", ".lwo", ".smd", ".md2", ".md3", ".md5mesh", ".ase", ".lxo", ".lws", ".dxf") {
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

Write-Host "Removing stale current-user shell overrides..."
Remove-CurrentUserShellOverrides

if ($RefreshOnly) {
    Write-Host "Notifying Windows of updated thumbnail handlers..."
    Send-ShellAssociationChange
    Write-Host "Done. Windows Installer reports any required reboot for locked files."
    return
}

# Explicit manual cache reset only. Never stop other sessions or unrelated COM
# surrogate processes; the installer always takes the RefreshOnly branch above.
$Explorers = @(Get-CurrentSessionExplorer)
$ExplorerWasRunning = $Explorers.Count -gt 0
if ($ExplorerWasRunning) {
    Write-Host "Stopping Explorer in the current session..."
    $Explorers | Stop-Process -Force
    Start-Sleep -Milliseconds 800
}

Write-Host "Clearing Explorer thumbnail and icon cache databases..."
Remove-CacheFiles @($explorerCacheDir)
Remove-Item -LiteralPath $legacyIconCache -Force

$ie4uinit = Join-Path $env:WINDIR "System32\ie4uinit.exe"
if (Test-Path -LiteralPath $ie4uinit) {
    Write-Host "Asking Windows to refresh icon cache..."
    & $ie4uinit -ClearIconCache | Out-Null
    & $ie4uinit -show | Out-Null
}

Send-ShellAssociationChange
if ($ExplorerWasRunning -and -not $NoRestartExplorer) {
    Write-Host "Restoring Explorer if Windows has not already restarted it..."
    Restore-ExplorerIfNeeded
}

Write-Host "Done. Reopen the model folder and switch the view size once if Explorer still shows stale thumbnails."
