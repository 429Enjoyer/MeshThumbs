param(
    [string]$Dll = "C:\Program Files\3DThumbnails\thumbnail_provider.dll"
)

$ErrorActionPreference = "Stop"

$ThumbHandler = "{e357fccd-a995-4576-b01f-234630154e96}"
$Dll = (Resolve-Path $Dll -ErrorAction SilentlyContinue).Path
$Providers = @(
    @{ Ext = ".obj";  Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00001}"; DisableProcessIsolation = 1 },
    @{ Ext = ".fbx";  Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00002}"; DisableProcessIsolation = 1 },
    @{ Ext = ".glb";  Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00003}"; DisableProcessIsolation = 1 },
    @{ Ext = ".gltf"; Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00004}"; DisableProcessIsolation = 1 },
    @{ Ext = ".stl"; Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00005}"; DisableProcessIsolation = 1 },
    @{ Ext = ".dae"; Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00006}"; DisableProcessIsolation = 1 },
    @{ Ext = ".ply"; Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D00007}"; DisableProcessIsolation = 1 },
    @{ Ext = ".3ds"; Clsid = "{0EF2C8D1-7B70-48C9-B7B8-0F45D3D0000A}"; DisableProcessIsolation = 1 }
)

if (-not (Test-Path $Dll)) {
    throw "DLL not found at $Dll."
}

$Worker = Join-Path (Split-Path -Parent $Dll) "thumbgen.exe"
if (-not (Test-Path -LiteralPath $Worker)) {
    throw "thumbgen.exe must be next to thumbnail_provider.dll. Build/install both files."
}

foreach ($Provider in $Providers) {
    $Ext = $Provider.Ext
    $Clsid = $Provider.Clsid

    New-Item -Force "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid\InprocServer32" | Out-Null
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid" -Name "(default)" -Value "3D Model Thumbnail Provider $Ext"
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid" -Name "DisableProcessIsolation" -Type DWord -Value $Provider.DisableProcessIsolation
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid\InprocServer32" -Name "(default)" -Value $Dll
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid\InprocServer32" -Name "ThreadingModel" -Value "Both"

    New-Item -Force "Registry::HKEY_LOCAL_MACHINE\Software\Classes\$Ext\shellex\$ThumbHandler" | Out-Null
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\$Ext\shellex\$ThumbHandler" -Name "(default)" -Value $Clsid
}

Remove-Item "Registry::HKEY_LOCAL_MACHINE\Software\Classes\AppXmgw6pxxs62rbgfp9petmdyb4fx7rnd4k\shellex\$ThumbHandler" -Recurse -Force -ErrorAction SilentlyContinue
Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Process explorer.exe
Write-Host "3DThumbnails machine shell registration refreshed."

