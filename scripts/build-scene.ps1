param(
    [ValidateSet("release", "debug")][string]$Configuration = "release",
    [string]$OutputDir,
    [string]$Generator,
    [ValidateRange(1, 64)][int]$Jobs = [Math]::Min(16, [Environment]::ProcessorCount)
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Native = Join-Path $Root "native\scene"
$Cache = Join-Path $Root ".tools\scene"
$Config = if ($Configuration -eq "release") { "Release" } else { "Debug" }
if (-not $OutputDir) {
    $TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
    $OutputDir = Join-Path $TargetRoot "$Configuration\scene"
}
New-Item -ItemType Directory -Force -Path $Cache,$OutputDir | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
$Install = Join-Path $Cache "install-$Configuration"
$GenArgs = if ($Generator) { @("-G", $Generator) } else { @() }
function Invoke-CMake { & cmake @args; if ($LASTEXITCODE -ne 0) { throw "Scene backend CMake failed ($LASTEXITCODE)." } }
function Invoke-Git { & git @args; if ($LASTEXITCODE -ne 0) { throw "Pinned openNURBS checkout failed." } }
function Get-SourceArchive($Name, $Url, $Hash, $Directory) {
    $Archive = Join-Path $Cache "$Name.tar.gz"
    if (-not (Test-Path -LiteralPath $Archive)) { Invoke-WebRequest -Uri $Url -OutFile $Archive }
    if ((Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash -ne $Hash) { throw "$Name source checksum mismatch" }
    $Source = Join-Path $Cache $Directory
    if (-not (Test-Path -LiteralPath $Source)) {
        Push-Location $Cache
        try { Invoke-CMake -E tar xzf $Archive } finally { Pop-Location }
    }
    return $Source
}
$Imath = Get-SourceArchive "Imath-3.1.12" "https://codeload.github.com/AcademySoftwareFoundation/Imath/tar.gz/refs/tags/v3.1.12" "8a1bc258f3149b5729c2f4f8ffd337c0e57f09096e4ba9784329f40c4a9035da" "Imath-3.1.12"
$Alembic = Get-SourceArchive "alembic-1.8.8" "https://codeload.github.com/alembic/alembic/tar.gz/refs/tags/1.8.8" "ba1f34544608ef7d3f68cafea946ec9cc84792ddf9cda3e8d5590821df71f6c6" "alembic-1.8.8"
$OpenNurbs = Join-Path $Cache "opennurbs"
$Revision = "eb92af3ba1806b0a34a99aba0d3bda83e3d46083"
if (-not (Test-Path -LiteralPath (Join-Path $OpenNurbs ".git"))) {
    Invoke-Git init $OpenNurbs
    Invoke-Git -C $OpenNurbs remote add origin https://github.com/mcneel/opennurbs.git
    Invoke-Git -C $OpenNurbs sparse-checkout init --cone
    Invoke-Git -C $OpenNurbs sparse-checkout set zlib android_uuid freetype263
    Invoke-Git -C $OpenNurbs fetch --depth 1 --filter=blob:none origin $Revision
    Invoke-Git -C $OpenNurbs checkout --detach FETCH_HEAD
}
$ActualRevision = (& git -C $OpenNurbs rev-parse HEAD).Trim()
if ($ActualRevision -ne $Revision) { throw "openNURBS checkout is not the pinned revision." }
Invoke-CMake "-DOPENNURBS_SOURCE_DIR=$OpenNurbs" -P (Join-Path $Native "patch-opennurbs.cmake")
$ImathBuild = Join-Path $Cache "imath-$Configuration"
$AlembicBuild = Join-Path $Cache "alembic-$Configuration"
$RhinoBuild = Join-Path $Cache "opennurbs-$Configuration"
$BackendBuild = Join-Path $Cache "backend-$Configuration"
Invoke-CMake -S $Imath -B $ImathBuild "-DCMAKE_BUILD_TYPE=$Config" -DBUILD_SHARED_LIBS=OFF -DBUILD_TESTING=OFF -DIMATH_BUILD_EXAMPLES=OFF "-DCMAKE_INSTALL_PREFIX=$Install" @GenArgs
Invoke-CMake --build $ImathBuild --config $Config --parallel $Jobs
Invoke-CMake --install $ImathBuild --config $Config
Invoke-CMake -S $Alembic -B $AlembicBuild "-DCMAKE_BUILD_TYPE=$Config" -DUSE_TESTS=OFF -DUSE_BINARIES=OFF -DUSE_HDF5=OFF -DALEMBIC_SHARED_LIBS=OFF "-DCMAKE_PREFIX_PATH=$Install" "-DCMAKE_INSTALL_PREFIX=$Install" @GenArgs
Invoke-CMake --build $AlembicBuild --config $Config --parallel $Jobs
Invoke-CMake --install $AlembicBuild --config $Config
Invoke-CMake -S $OpenNurbs -B $RhinoBuild "-DCMAKE_BUILD_TYPE=$Config" -DBUILD_TESTING=OFF @GenArgs
$CompilerInfo = Get-Content -Path (Join-Path $RhinoBuild "CMakeFiles\*\CMakeCXXCompiler.cmake") -Raw
if ($CompilerInfo -match 'set\(CMAKE_CXX_COMPILER_ID "GNU"\)') {
    $Header = (Join-Path $Native "compat\opennurbs_mingw.h").Replace('\','/')
    Invoke-CMake -S $OpenNurbs -B $RhinoBuild "-DCMAKE_CXX_FLAGS=-UWIN32 -include `"$Header`""
}
Invoke-CMake --build $RhinoBuild --config $Config --target opennurbsStatic --parallel $Jobs
$RhinoLibrary = @(Get-ChildItem -LiteralPath $RhinoBuild -Recurse -File | Where-Object { $_.Name -in @('opennurbsStatic.lib','libopennurbsStatic.a') })
$ZlibLibrary = @(Get-ChildItem -LiteralPath $RhinoBuild -Recurse -File | Where-Object { $_.Name -in @('zlib.lib','libzlib.a') })
if ($RhinoLibrary.Count -ne 1 -or $ZlibLibrary.Count -ne 1) { throw "Cannot uniquely locate openNURBS static libraries." }
Invoke-CMake -S $Native -B $BackendBuild "-DCMAKE_BUILD_TYPE=$Config" "-DCMAKE_PREFIX_PATH=$Install" "-DOPENNURBS_SOURCE_DIR=$OpenNurbs" "-DOPENNURBS_LIBRARY=$($RhinoLibrary[0].FullName)" "-DOPENNURBS_ZLIB=$($ZlibLibrary[0].FullName)" "-DCMAKE_INSTALL_PREFIX=$OutputDir" @GenArgs
Invoke-CMake --build $BackendBuild --config $Config --parallel $Jobs
Invoke-CMake --install $BackendBuild --config $Config
$Files = @("meshthumbs_scene.dll", "SCENE-SOURCE.md")
Copy-Item -LiteralPath (Join-Path $Root "docs\SCENE-SOURCE.md") -Destination $OutputDir
if ($CompilerInfo -match 'set\(CMAKE_CXX_COMPILER_ID "GNU"\)') {
    $Compiler = [regex]::Match($CompilerInfo, 'set\(CMAKE_CXX_COMPILER "([^"]+)"\)').Groups[1].Value
    foreach ($Name in @("libgcc_s_seh-1.dll","libstdc++-6.dll","libwinpthread-1.dll")) {
        $Runtime = (& $Compiler "-print-file-name=$Name").Trim()
        if (-not (Test-Path -LiteralPath $Runtime -PathType Leaf)) { throw "Missing MinGW runtime $Name" }
        Copy-Item -LiteralPath $Runtime -Destination $OutputDir
        $Files += $Name
    }
}
$Files | Sort-Object | Set-Content -LiteralPath (Join-Path $OutputDir "runtime-files.txt") -Encoding ascii
Write-Host "Alembic/3DM backend: $OutputDir"
