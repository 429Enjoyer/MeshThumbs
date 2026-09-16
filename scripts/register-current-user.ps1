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
    @{ Ext = ".3ds"; Clsid = "{0AD51061-9A3C-4EC3-9757-874ECB89457C}"; DisableProcessIsolation = 1 },
    @{ Ext = ".3mf"; Clsid = "{35A24A7A-CC90-48B3-9849-548DDAA06B01}"; DisableProcessIsolation = 1 },
    @{ Ext = ".vrm"; Clsid = "{4C451BA6-CC4C-47FB-8F5F-3D32029E2F45}"; DisableProcessIsolation = 1 },
    @{ Ext = ".blend"; Clsid = "{AF3D3DCD-8C60-4E29-A483-46DB39C40FB9}"; DisableProcessIsolation = 1 },
    @{ Ext = ".x3d"; Clsid = "{098B9E30-DA6D-43B8-9C2D-F82D32B4B070}"; DisableProcessIsolation = 1 },
    @{ Ext = ".off"; Clsid = "{F0C794C3-D2B3-42D4-A861-966CED3C966B}"; DisableProcessIsolation = 1 },
    @{ Ext = ".usd"; Clsid = "{61B9231B-1E4B-4A1B-ADFB-812E3AE8A6BB}"; DisableProcessIsolation = 1 },
    @{ Ext = ".usda"; Clsid = "{3EC714F0-B74B-477C-B682-8B74AEE7C61A}"; DisableProcessIsolation = 1 },
    @{ Ext = ".usdc"; Clsid = "{125A17BA-7451-4726-BD0B-738E93ADD02F}"; DisableProcessIsolation = 1 },
    @{ Ext = ".usdz"; Clsid = "{EEBDC2F2-E564-476C-AD42-C8F35F8997F4}"; DisableProcessIsolation = 1 },
    @{ Ext = ".wrl"; Clsid = "{991F57D3-64BA-42C9-A65C-19530FF5C176}"; DisableProcessIsolation = 1 },
    @{ Ext = ".vrml"; Clsid = "{D84B7C81-992B-480A-9108-17321DCF1410}"; DisableProcessIsolation = 1 },
    @{ Ext = ".step"; Clsid = "{4C93500D-0E9B-46EC-9E14-831C72A0B34C}"; DisableProcessIsolation = 1 },
    @{ Ext = ".stp"; Clsid = "{EFE86458-F28C-43FE-BD01-FD661C662344}"; DisableProcessIsolation = 1 },
    @{ Ext = ".abc"; Clsid = "{C20FDBFB-A119-402B-9DF4-87768E6FB3AB}"; DisableProcessIsolation = 1 },
    @{ Ext = ".igs"; Clsid = "{3604F66F-141E-4741-BB85-6B5D1853C553}"; DisableProcessIsolation = 1 },
    @{ Ext = ".iges"; Clsid = "{C370DCC3-81F8-4B5A-9562-957D3588222B}"; DisableProcessIsolation = 1 },
    @{ Ext = ".3dm"; Clsid = "{5EABC4CA-0201-41A3-B6BC-F325AEFB5030}"; DisableProcessIsolation = 1 },
    @{ Ext = ".ifc"; Clsid = "{422DE617-9A3A-41AE-96B9-19E0ED555129}"; DisableProcessIsolation = 1 },
    @{ Ext = ".pmx"; Clsid = "{C0036F44-536A-4952-9F4C-9F088FFB5241}"; DisableProcessIsolation = 1 },
    @{ Ext = ".vox"; Clsid = "{DBD5A742-3E62-4AA6-88B8-8ABEA8DD3B27}"; DisableProcessIsolation = 1 },
    @{ Ext = ".lwo"; Clsid = "{8B08C226-6E4B-4D15-9629-A724BC4B753B}"; DisableProcessIsolation = 1 }
)

if (-not (Test-Path $Dll)) {
    throw "DLL not found at $Dll. Install the MSI first."
}

$Worker = Join-Path (Split-Path -Parent $Dll) "thumbgen.exe"
if (-not (Test-Path -LiteralPath $Worker)) {
    throw "thumbgen.exe must be next to thumbnail_provider.dll. Build/install both files."
}

foreach ($Provider in $Providers) {
    $Ext = $Provider.Ext
    $Clsid = $Provider.Clsid

    New-Item -Force "HKCU:\Software\Classes\CLSID\$Clsid\InprocServer32" | Out-Null
    Set-ItemProperty "HKCU:\Software\Classes\CLSID\$Clsid\InprocServer32" -Name "(default)" -Value $Dll
    Set-ItemProperty "HKCU:\Software\Classes\CLSID\$Clsid\InprocServer32" -Name "ThreadingModel" -Value "Both"
    Set-ItemProperty "HKCU:\Software\Classes\CLSID\$Clsid" -Name "DisableProcessIsolation" -Type DWord -Value $Provider.DisableProcessIsolation

    New-Item -Force "HKCU:\Software\Classes\$Ext\shellex\$ThumbHandler" | Out-Null
    Set-ItemProperty "HKCU:\Software\Classes\$Ext\shellex\$ThumbHandler" -Name "(default)" -Value $Clsid

    New-Item -Force "HKCU:\Software\Classes\SystemFileAssociations\$Ext\shellex\$ThumbHandler" | Out-Null
    Set-ItemProperty "HKCU:\Software\Classes\SystemFileAssociations\$Ext\shellex\$ThumbHandler" -Name "(default)" -Value $Clsid

    $UserChoice = Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$Ext\UserChoice" -ErrorAction SilentlyContinue
    if ($UserChoice.ProgId) {
        New-Item -Force "HKCU:\Software\Classes\$($UserChoice.ProgId)\shellex\$ThumbHandler" | Out-Null
        Set-ItemProperty "HKCU:\Software\Classes\$($UserChoice.ProgId)\shellex\$ThumbHandler" -Name "(default)" -Value $Clsid
    }
}

Stop-Process -Name explorer -Force -ErrorAction SilentlyContinue
Start-Process explorer.exe
Write-Host "MeshThumbs current-user shell registration refreshed."

