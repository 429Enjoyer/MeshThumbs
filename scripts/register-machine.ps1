param(
    [string]$Dll = "C:\Program Files\MeshThumbs\thumbnail_provider.dll"
)

$ErrorActionPreference = "Stop"

$ThumbHandler = "{e357fccd-a995-4576-b01f-234630154e96}"
$Dll = (Resolve-Path $Dll -ErrorAction SilentlyContinue).Path
$Providers = @(
    @{ Ext = ".obj";  Clsid = "{A9FFD4C4-3FA9-4EB7-8B47-B89A7F09D059}"; DisableProcessIsolation = 1 },
    @{ Ext = ".fbx";  Clsid = "{4E5FD91F-C018-4850-9636-3069629C6D3D}"; DisableProcessIsolation = 1 },
    @{ Ext = ".glb";  Clsid = "{E859325C-5506-4419-8AC5-6A4B03F3A138}"; DisableProcessIsolation = 1 },
    @{ Ext = ".gltf"; Clsid = "{B7265976-0DBA-44B5-9303-0B0DAFD034E0}"; DisableProcessIsolation = 1 },
    @{ Ext = ".stl"; Clsid = "{A3BAFD17-52CD-4CF6-869E-A4BB020591EF}"; DisableProcessIsolation = 1 },
    @{ Ext = ".dae"; Clsid = "{7BF654CD-6B62-4A1C-BE5F-53DF447C2BE6}"; DisableProcessIsolation = 1 },
    @{ Ext = ".ply"; Clsid = "{AB2CDE52-5C15-4DAF-B43A-E4C9F1EAAEC0}"; DisableProcessIsolation = 1 },
    @{ Ext = ".3ds"; Clsid = "{0AD51061-9A3C-4EC3-9757-874ECB89457C}"; DisableProcessIsolation = 1 }
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
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid" -Name "(default)" -Value "MeshThumbs Thumbnail Provider $Ext"
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid" -Name "DisableProcessIsolation" -Type DWord -Value $Provider.DisableProcessIsolation
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid\InprocServer32" -Name "(default)" -Value $Dll
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\CLSID\$Clsid\InprocServer32" -Name "ThreadingModel" -Value "Both"

    New-Item -Force "Registry::HKEY_LOCAL_MACHINE\Software\Classes\$Ext\shellex\$ThumbHandler" | Out-Null
    Set-ItemProperty "Registry::HKEY_LOCAL_MACHINE\Software\Classes\$Ext\shellex\$ThumbHandler" -Name "(default)" -Value $Clsid
}

Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Process explorer.exe
Write-Host "MeshThumbs machine shell registration refreshed."

