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
$WixBin = Join-Path $Root "wix\bin"
$ProductWxs = Join-Path $Root "wix\Product.wxs"
$OutputMsi = Join-Path $Root "MeshThumbs-1.0.7-x64.msi"
$LocalWix = Join-Path $Root ".tools\wix314"

if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot "build-step.ps1") -Configuration $Configuration -OutputDir (Join-Path $TargetDir "step")
    $CargoArgs = @("build", "--manifest-path", (Join-Path $Root "Cargo.toml"), "-p", "thumbnail_provider", "-p", "thumbgen")
    if ($Configuration -eq "release") { $CargoArgs += "--release" }
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Thumbnail provider build failed." }
}
if (-not (Test-Path (Join-Path $TargetDir "thumbnail_provider.dll")) -or -not (Test-Path (Join-Path $TargetDir "thumbgen.exe"))) {
    throw "Build both thumbnail_provider.dll and thumbgen.exe in $TargetDir."
}

New-Item -ItemType Directory -Force -Path $WixObj, $WixBin | Out-Null

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

$candle = Get-Command candle.exe -ErrorAction SilentlyContinue
$light = Get-Command light.exe -ErrorAction SilentlyContinue
if (-not $candle -and (Test-Path (Join-Path $LocalWix "candle.exe"))) {
    $candle = Get-Item (Join-Path $LocalWix "candle.exe")
}
if (-not $light -and (Test-Path (Join-Path $LocalWix "light.exe"))) {
    $light = Get-Item (Join-Path $LocalWix "light.exe")
}
if ($candle -and $light) {
    & $candle.FullName -arch x64 "-dTargetDir=$TargetDir" "-dProjectDir=$Root" -out (Join-Path $WixObj "Product.wixobj") $ProductWxs
    if ($LASTEXITCODE -ne 0) { throw "WiX compilation failed." }
    & $candle.FullName -arch x64 -out (Join-Path $WixObj "StepRuntime.wixobj") $StepWxs
    if ($LASTEXITCODE -ne 0) { throw "STEP runtime WiX compilation failed." }
    & $light.FullName -out $OutputMsi (Join-Path $WixObj "Product.wixobj") (Join-Path $WixObj "StepRuntime.wixobj")
    exit $LASTEXITCODE
}

throw "WiX v3.14 not found. Install its candle.exe/light.exe tools or place them in .tools\wix314. Product.wxs uses the WiX v3 schema."
