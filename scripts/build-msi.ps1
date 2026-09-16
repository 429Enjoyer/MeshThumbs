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
$OutputMsi = Join-Path $Root "MeshThumbs-1.0.3-x64.msi"
$LocalWix = Join-Path $Root ".tools\wix314"

if (-not $SkipBuild) {
    $CargoArgs = @("build", "--manifest-path", (Join-Path $Root "Cargo.toml"), "-p", "thumbnail_provider", "-p", "thumbgen")
    if ($Configuration -eq "release") { $CargoArgs += "--release" }
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Thumbnail provider build failed." }
}
if (-not (Test-Path (Join-Path $TargetDir "thumbnail_provider.dll")) -or -not (Test-Path (Join-Path $TargetDir "thumbgen.exe"))) {
    throw "Build both thumbnail_provider.dll and thumbgen.exe in $TargetDir."
}

New-Item -ItemType Directory -Force -Path $WixObj, $WixBin | Out-Null

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
    & $light.FullName -out $OutputMsi (Join-Path $WixObj "Product.wixobj")
    exit $LASTEXITCODE
}

throw "WiX v3.14 not found. Install its candle.exe/light.exe tools or place them in .tools\wix314. Product.wxs uses the WiX v3 schema."
