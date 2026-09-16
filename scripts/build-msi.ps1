param(
    [ValidateSet("release", "debug")]
    [string]$Configuration = "release",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
$TargetDir = Join-Path $TargetRoot $Configuration
$WixObj = Join-Path $Root "wix\obj"
$ProductWxs = Join-Path $Root "wix\Product.wxs"
$InstallerUiWxs = Join-Path $Root "wix\InstallerUI.wxs"
$RestartExplorerSource = Get-Content -LiteralPath (Join-Path $Root "scripts\restart-explorer.ps1") -Raw
$RestartExplorerCommand = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($RestartExplorerSource))
$OutputMsi = Join-Path $Root "MeshThumbs-1.1.5-x64.msi"
$LocalWix = Join-Path $Root ".tools\wix314"

if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot "build-step.ps1") -Configuration $Configuration -OutputDir (Join-Path $TargetDir "step")
    & (Join-Path $PSScriptRoot "build-scene.ps1") -Configuration $Configuration -OutputDir (Join-Path $TargetDir "scene")
    & (Join-Path $PSScriptRoot "prepare-ifc.ps1") -Configuration $Configuration -OutputDir (Join-Path $TargetDir "ifc")
    $CargoArgs = @("build", "--manifest-path", (Join-Path $Root "Cargo.toml"), "-p", "thumbnail_provider", "-p", "thumbgen", "-p", "png_export")
    if ($Configuration -eq "release") { $CargoArgs += "--release" }
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Thumbnail provider build failed." }
}
if (-not (Test-Path (Join-Path $TargetDir "thumbnail_provider.dll")) -or -not (Test-Path (Join-Path $TargetDir "thumbgen.exe")) -or -not (Test-Path (Join-Path $TargetDir "meshthumbs-export.exe"))) {
    throw "Build thumbnail_provider.dll, thumbgen.exe and meshthumbs-export.exe in $TargetDir."
}

New-Item -ItemType Directory -Force -Path $WixObj | Out-Null

# Feed the project's actual license to the standard WiX welcome/license page.
$LicenseText = Get-Content -LiteralPath (Join-Path $Root "LICENSE") -Raw
$LicenseText += "`r`nThird-party components retain their own terms. See THIRD-PARTY-NOTICES.txt for notices and source information.`r`n"
$LicenseRtf = $LicenseText.Replace('\', '\\').Replace('{', '\{').Replace('}', '\}')
$LicenseRtf = $LicenseRtf -replace '\r?\n', '\par '
$LicenseRtf = [regex]::Replace($LicenseRtf, '[^\x00-\x7F]', { param($match) '\u' + [int][char]$match.Value + '?' })
[IO.File]::WriteAllText((Join-Path $WixObj "License.rtf"), ('{\rtf1\ansi\deff0{\fonttbl{\f0 Segoe UI;}}\f0\fs18 ' + $LicenseRtf + '}'), [Text.Encoding]::ASCII)

# Accompany the compiled UI with this build's original installer authoring.
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$InstallerSourcePath = Join-Path $WixObj "meshthumbs-installer-source.zip"
$SourceStream = [IO.File]::Open($InstallerSourcePath, [IO.FileMode]::Create)
$SourceZip = New-Object IO.Compression.ZipArchive($SourceStream, [IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($Relative in @("wix/Product.wxs", "wix/InstallerUI.wxs", "wix/MeshThumbsWixUIExtension.cs", "scripts/build-msi.ps1", "scripts/restart-explorer.ps1", "scripts/clear-explorer-cache.ps1", "LICENSE", "docs/WIX-UI-SOURCE.md")) {
        [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($SourceZip, (Join-Path $Root $Relative), $Relative)
    }
} finally { $SourceZip.Dispose(); $SourceStream.Dispose() }

$StepDir = Join-Path $TargetDir "step"
$StepManifest = Join-Path $StepDir "runtime-files.txt"
if (-not (Test-Path -LiteralPath $StepManifest)) {
    throw "STEP runtime missing. Run scripts/build-step.ps1 before packaging."
}
$StepFiles = @(Get-Content -LiteralPath $StepManifest | Where-Object { $_ } | Sort-Object -Unique)
foreach ($Required in @("meshthumbs_step.dll", "occt-source-7.9.3.tar.gz", "OCCT-SOURCE.md", "occt-options.cmake", "mingw-toolchain.cmake")) {
    if ($Required -notin $StepFiles) { throw "STEP runtime manifest is missing $Required" }
}
$StepComponents = foreach ($Name in $StepFiles) {
    if ($Name -notmatch '^[A-Za-z0-9][A-Za-z0-9_.+-]*$') { throw "Invalid STEP runtime filename: $Name" }
    $Source = Join-Path $StepDir $Name
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Missing STEP runtime file: $Source" }
    $Id = "Step_" + ($Name -replace '[^A-Za-z0-9_]', '_')
    $EscapedSource = [System.Security.SecurityElement]::Escape($Source)
    "<Component Id=`"$Id`" Guid=`"*`" Win64=`"yes`"><File Id=`"${Id}_File`" Name=`"$Name`" Source=`"$EscapedSource`" KeyPath=`"yes`" /></Component>"
}
$StepWxs = Join-Path $WixObj "StepRuntime.wxs"
@"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi"><Fragment>
<ComponentGroup Id="StepRuntimeComponents" Directory="STEPFOLDER">
$($StepComponents -join "`n")
</ComponentGroup></Fragment></Wix>
"@ | Set-Content -LiteralPath $StepWxs -Encoding utf8

$SceneDir = Join-Path $TargetDir "scene"
$SceneManifest = Join-Path $SceneDir "runtime-files.txt"
if (-not (Test-Path -LiteralPath $SceneManifest)) { throw "Run scripts/build-scene.ps1 before packaging." }
$SceneFiles = @(Get-Content -LiteralPath $SceneManifest | Where-Object { $_ } | Sort-Object -Unique)
foreach ($Required in @("meshthumbs_scene.dll", "SCENE-SOURCE.md")) {
    if ($Required -notin $SceneFiles) { throw "Scene runtime manifest is missing $Required" }
}
$SceneComponents = foreach ($Name in $SceneFiles) {
    if ($Name -notmatch '^[A-Za-z0-9][A-Za-z0-9_.+-]*$') { throw "Invalid scene runtime filename: $Name" }
    $Source = Join-Path $SceneDir $Name
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Missing scene runtime file: $Source" }
    $Id = "Scene_" + ($Name -replace '[^A-Za-z0-9_]', '_')
    $EscapedSource = [System.Security.SecurityElement]::Escape($Source)
    "<Component Id=`"$Id`" Guid=`"*`" Win64=`"yes`"><File Id=`"${Id}_File`" Name=`"$Name`" Source=`"$EscapedSource`" KeyPath=`"yes`" /></Component>"
}
$SceneWxs = Join-Path $WixObj "SceneRuntime.wxs"
@"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi"><Fragment>
<ComponentGroup Id="SceneRuntimeComponents" Directory="SCENEFOLDER">
$($SceneComponents -join "`n")
</ComponentGroup></Fragment></Wix>
"@ | Set-Content -LiteralPath $SceneWxs -Encoding utf8

$IfcDir = Join-Path $TargetDir "ifc"
$IfcManifest = Join-Path $IfcDir "runtime-files.txt"
if (-not (Test-Path -LiteralPath $IfcManifest)) { throw "Run scripts/prepare-ifc.ps1 before packaging." }
$IfcFiles = @(Get-Content -LiteralPath $IfcManifest | Where-Object { $_ } | Sort-Object -Unique)
foreach ($Required in @("IfcConvert.exe", "IFC-SOURCE.md", "IFC-NOTICES.txt", "ifcconvert-source-0.8.5.tar.gz", "build-info.json", "patch-ifc.cmake", "build-ifc-mingw.sh")) {
    if ($Required -notin $IfcFiles) { throw "IFC runtime manifest is missing $Required" }
}
$IfcComponents = foreach ($Name in $IfcFiles) {
    if ($Name -notmatch '^[A-Za-z0-9][A-Za-z0-9_.+-]*$' -or $Name -match '^(msvc|vcruntime)') { throw "Invalid IFC runtime filename: $Name" }
    $Source = Join-Path $IfcDir $Name
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Missing IFC runtime file: $Source. Run scripts/prepare-ifc.ps1." }
    $Id = "Ifc_" + ($Name -replace '[^A-Za-z0-9_]', '_')
    $EscapedSource = [System.Security.SecurityElement]::Escape($Source)
    "<Component Id=`"$Id`" Guid=`"*`" Win64=`"yes`"><File Id=`"${Id}_File`" Name=`"$Name`" Source=`"$EscapedSource`" KeyPath=`"yes`" /></Component>"
}
$IfcWxs = Join-Path $WixObj "IfcRuntime.wxs"
@"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi"><Fragment>
<ComponentGroup Id="IfcRuntimeComponents" Directory="IFCFOLDER">
$($IfcComponents -join "`n")
</ComponentGroup></Fragment></Wix>
"@ | Set-Content -LiteralPath $IfcWxs -Encoding utf8

# Use the renderer's supported-extension list for the Explorer command too.
$RendererSource = Get-Content -LiteralPath (Join-Path $Root "crates\renderer\src\lib.rs") -Raw
$ExtensionBlock = [regex]::Match($RendererSource, '(?s)pub const SUPPORTED_EXTENSIONS:.*?= &\[(.*?)\];').Groups[1].Value
$MenuExtensions = @([regex]::Matches($ExtensionBlock, '"([a-z0-9]+)"') | ForEach-Object { $_.Groups[1].Value })
if ($MenuExtensions.Count -eq 0 -or ($MenuExtensions | Sort-Object -Unique).Count -ne $MenuExtensions.Count) { throw "Invalid supported-extension list" }
$MenuClass = "{2C7A8D3E-72CA-4B24-9D8C-426DDC3A1515}"
$MenuKeys = foreach ($Extension in $MenuExtensions) {
    @"
<RegistryKey Root="HKCR" Key="SystemFileAssociations\.$Extension\shell\MeshThumbs.GenerateThumbnailPNG">
  <RegistryValue Name="MUIVerb" Type="string" Value="MeshThumbs" />
  <RegistryValue Name="ExplorerCommandHandler" Type="string" Value="$MenuClass" />
  <RegistryValue Name="MultiSelectModel" Type="string" Value="Player" />
</RegistryKey>
"@
}
$MenuWxs = Join-Path $WixObj "ExportMenu.wxs"
@"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi"><Fragment>
<ComponentGroup Id="ExportMenuComponents" Directory="INSTALLFOLDER">
<Component Id="ExportContextMenu" Guid="*" Win64="yes">
<RegistryKey Root="HKCR" Key="CLSID\$MenuClass">
  <RegistryValue Type="string" Value="MeshThumbs PNG Export Command" KeyPath="yes" />
  <RegistryKey Key="InprocServer32"><RegistryValue Type="string" Value="[INSTALLFOLDER]thumbnail_provider.dll" />
    <RegistryValue Name="ThreadingModel" Type="string" Value="Both" /></RegistryKey>
</RegistryKey>
$($MenuKeys -join "`n")
</Component></ComponentGroup></Fragment></Wix>
"@ | Set-Content -LiteralPath $MenuWxs -Encoding utf8

$candle = Get-Command candle.exe -ErrorAction SilentlyContinue
$light = Get-Command light.exe -ErrorAction SilentlyContinue
if (-not $candle -and (Test-Path (Join-Path $LocalWix "candle.exe"))) {
    $candle = Get-Item (Join-Path $LocalWix "candle.exe")
}
if (-not $light -and (Test-Path (Join-Path $LocalWix "light.exe"))) {
    $light = Get-Item (Join-Path $LocalWix "light.exe")
}
if ($candle -and $light) {
    $UiExtension = Join-Path (Split-Path -Parent $light.FullName) "WixUIExtension.dll"
    if (-not (Test-Path -LiteralPath $UiExtension)) { throw "WixUIExtension.dll is required beside light.exe." }
    # WiX auto-includes its FilesInUse dialog. Filter that one library section
    # while keeping the standard Minimal dialogs/navigation and assets intact.
    $Csc = Join-Path $env:WINDIR "Microsoft.NET\Framework64\v4.0.30319\csc.exe"
    $WixSdk = Join-Path (Split-Path -Parent $light.FullName) "wix.dll"
    $UiAdapter = Join-Path $WixObj "MeshThumbsWixUIExtension.dll"
    & $Csc /nologo /target:library "/reference:$WixSdk" "/reference:$UiExtension" "/out:$UiAdapter" (Join-Path $Root "wix\MeshThumbsWixUIExtension.cs")
    if ($LASTEXITCODE -ne 0) { throw "WiX UI adapter compilation failed." }
    & $candle.FullName -arch x64 "-dTargetDir=$TargetDir" "-dProjectDir=$Root" "-dRestartExplorerCommand=$RestartExplorerCommand" -out (Join-Path $WixObj "Product.wixobj") $ProductWxs
    if ($LASTEXITCODE -ne 0) { throw "WiX compilation failed." }
    & $candle.FullName -arch x64 -out (Join-Path $WixObj "InstallerUI.wixobj") $InstallerUiWxs
    if ($LASTEXITCODE -ne 0) { throw "Installer UI compilation failed." }
    & $candle.FullName -arch x64 -out (Join-Path $WixObj "StepRuntime.wixobj") $StepWxs
    if ($LASTEXITCODE -ne 0) { throw "STEP runtime WiX compilation failed." }
    & $candle.FullName -arch x64 -out (Join-Path $WixObj "SceneRuntime.wixobj") $SceneWxs
    if ($LASTEXITCODE -ne 0) { throw "Scene runtime WiX compilation failed." }
    & $candle.FullName -arch x64 -out (Join-Path $WixObj "IfcRuntime.wixobj") $IfcWxs
    if ($LASTEXITCODE -ne 0) { throw "IFC runtime WiX compilation failed." }
    & $candle.FullName -arch x64 -out (Join-Path $WixObj "ExportMenu.wixobj") $MenuWxs
    if ($LASTEXITCODE -ne 0) { throw "Export menu WiX compilation failed." }
    & $light.FullName -ext $UiAdapter -cultures:en-us -out $OutputMsi (Join-Path $WixObj "Product.wixobj") (Join-Path $WixObj "InstallerUI.wixobj") (Join-Path $WixObj "StepRuntime.wixobj") (Join-Path $WixObj "SceneRuntime.wixobj") (Join-Path $WixObj "IfcRuntime.wixobj") (Join-Path $WixObj "ExportMenu.wixobj")
    exit $LASTEXITCODE
}

throw "WiX v3.14 not found. Install its candle.exe/light.exe tools or place them in .tools\wix314. Product.wxs uses the WiX v3 schema."
