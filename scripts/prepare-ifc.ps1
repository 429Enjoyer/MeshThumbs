param(
    [ValidateSet("release", "debug")][string]$Configuration = "release",
    [string]$OutputDir,
    [string]$SourceDir,
    [switch]$SkipBuild,
    [string]$Distribution = "Ubuntu",
    [string]$CacheDir,
    [string]$OcctPrefix,
    [ValidateRange(1,16)][int]$Jobs = 4
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $SourceDir) { $SourceDir = Join-Path $Root ".tools\ifc-mingw" }
if (-not $OutputDir) {
    $TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
    $OutputDir = Join-Path $TargetRoot "$Configuration\ifc"
}
New-Item -ItemType Directory -Force -Path $SourceDir,$OutputDir | Out-Null
$SourceDir = (Resolve-Path -LiteralPath $SourceDir).Path
if (-not $SkipBuild) {
    $ScriptPath = (Join-Path $Root "scripts\build-ifc-mingw.sh").Replace('\','/')
    $UnixScript = & wsl -d $Distribution -- wslpath -a $ScriptPath
    if ($LASTEXITCODE -ne 0) { throw "WSL with MinGW/CMake is required; see docs/IFC-SOURCE.md" }
    $UnixScript = ([string]$UnixScript).Trim()
    $UnixOutput = & wsl -d $Distribution -- wslpath -a $SourceDir.Replace('\','/')
    if ($LASTEXITCODE -ne 0) { throw "Cannot resolve IFC build output in WSL" }
    $UnixOutput = ([string]$UnixOutput).Trim()
    $BuildArgs = @("-d", $Distribution, "--", "env", "MESHTHUMBS_BUILD_JOBS=$Jobs")
    if ($CacheDir) { $BuildArgs += "MESHTHUMBS_IFC_CACHE=$CacheDir" }
    if ($OcctPrefix) { $BuildArgs += "MESHTHUMBS_OCCT_PREFIX=$OcctPrefix" }
    $BuildArgs += @("bash", $UnixScript, $UnixOutput)
    & wsl @BuildArgs
    if ($LASTEXITCODE -ne 0) { throw "IFC4 MinGW source build failed" }
}
$ManifestPath = Join-Path $SourceDir "build-info.json"
& (Join-Path $SourceDir "check-float.exe")
if ($LASTEXITCODE -ne 0) { throw "IFC real-number parser regression check failed" }
$Manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
if ($Manifest.source -ne "1c5b825d8ef05ab9d14a15dac12e9eae2f5a37c2" -or $Manifest.schema -ne "IFC4" -or $Manifest.occt -ne "7.9.3") { throw "Unexpected IFC source build" }
foreach ($Property in $Manifest.inputs.PSObject.Properties) {
    if ((Get-FileHash -LiteralPath (Join-Path $Root $Property.Name) -Algorithm SHA256).Hash -ne $Property.Value) { throw "IFC build inputs changed; rebuild: $($Property.Name)" }
}
$Files = @()
foreach ($Property in $Manifest.files.PSObject.Properties) {
    $Name = $Property.Name
    if ($Name -notmatch '^[A-Za-z0-9][A-Za-z0-9_.+-]*$' -or $Name -match '^(msvc|vcruntime)') { throw "Unexpected IFC runtime filename: $Name" }
    $Source = Join-Path $SourceDir $Name
    if ((Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash -ne $Property.Value) { throw "IFC runtime checksum mismatch: $Name" }
    Copy-Item -LiteralPath $Source -Destination (Join-Path $OutputDir $Name)
    $Files += $Name
}
if ("IfcConvert.exe" -notin $Files) { throw "Missing source-built IFC converter" }
Copy-Item -LiteralPath $ManifestPath -Destination $OutputDir
$Files += "build-info.json"
foreach ($Name in @("IFC-SOURCE.md", "IFC-NOTICES.txt")) {
    Copy-Item -LiteralPath (Join-Path $Root "docs\$Name") -Destination $OutputDir
    $Files += $Name
}
foreach ($Entry in @(
    @("docs\third-party\ifcconvert-source-0.8.5.tar.gz", "ifcconvert-source-0.8.5.tar.gz"),
    @("scripts\build-ifc-mingw.sh", "build-ifc-mingw.sh"),
    @("native\ifc\patch-ifc.cmake", "patch-ifc.cmake"),
    @("native\ifc\check-float.cpp", "check-float.cpp"),
    @("native\ifc\occt-options.cmake", "ifc-occt-options.cmake")
)) {
    Copy-Item -LiteralPath (Join-Path $Root $Entry[0]) -Destination (Join-Path $OutputDir $Entry[1])
    $Files += $Entry[1]
}
$Files | Sort-Object -Unique | Set-Content -LiteralPath (Join-Path $OutputDir "runtime-files.txt") -Encoding ascii
Write-Host "IFC4 MinGW helper prepared in $OutputDir (no Microsoft runtime)"
