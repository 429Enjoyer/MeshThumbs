param(
    [ValidateSet("release", "debug")]
    [string]$Configuration = "release",
    [string]$OutputDir,
    [string]$OcctSource,
    [string]$Generator,
    [ValidateRange(1, 64)]
    [int]$Jobs = [Math]::Min(16, [Environment]::ProcessorCount)
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Cache = Join-Path $Root ".tools\step"
$Native = Join-Path $Root "native\step"
$Config = if ($Configuration -eq "release") { "Release" } else { "Debug" }
if (-not $OutputDir) {
    $TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
    $OutputDir = Join-Path $TargetRoot "$Configuration\step"
}
New-Item -ItemType Directory -Force -Path $Cache, $OutputDir | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path

function Invoke-CMake {
    & cmake @args
    if ($LASTEXITCODE -ne 0) { throw "STEP backend CMake command failed ($LASTEXITCODE)." }
}

if (-not $OcctSource) {
    $Archive = Join-Path $Cache "OCCT-7.9.3.tar.gz"
    if (-not (Test-Path -LiteralPath $Archive)) {
        Invoke-WebRequest -Uri "https://codeload.github.com/Open-Cascade-SAS/OCCT/tar.gz/refs/tags/V7_9_3" -OutFile $Archive
    }
    $Expected = "5ecf094ec6b12d5413dfb851d8c3590c354058aee556e32e408bdfbf8c357d57"
    if ((Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash -ne $Expected) {
        throw "Open CASCADE source archive checksum mismatch: $Archive"
    }
    $OcctSource = Join-Path $Cache "OCCT-7_9_3"
    if (-not (Test-Path -LiteralPath (Join-Path $OcctSource "CMakeLists.txt"))) {
        Push-Location $Cache
        try { Invoke-CMake -E tar xzf $Archive } finally { Pop-Location }
    }
}
$OcctSource = (Resolve-Path -LiteralPath $OcctSource).Path
$OcctBuild = Join-Path $Cache "occt-$Configuration"
$OcctInstall = Join-Path $Cache "install-$Configuration"
$BackendBuild = Join-Path $Cache "backend-$Configuration"
$GeneratorArgs = if ($Generator) { @("-G", $Generator) } else { @() }
Invoke-CMake -S $OcctSource -B $OcctBuild -C (Join-Path $Native "occt-options.cmake") "-DCMAKE_BUILD_TYPE=$Config" "-DCMAKE_INSTALL_PREFIX=$OcctInstall" @GeneratorArgs
$CompilerInfo = Get-Content -Path (Join-Path $OcctBuild "CMakeFiles\*\CMakeCXXCompiler.cmake") -Raw
Invoke-CMake --build $OcctBuild --config $Config --parallel $Jobs
Invoke-CMake --install $OcctBuild --config $Config
Invoke-CMake -S $Native -B $BackendBuild "-DOpenCASCADE_DIR=$OcctInstall\cmake" "-DCMAKE_BUILD_TYPE=$Config" "-DCMAKE_INSTALL_PREFIX=$OutputDir" @GeneratorArgs
Invoke-CMake --build $BackendBuild --config $Config --parallel $Jobs
Invoke-CMake --install $BackendBuild --config $Config

# Include all library source and build scripts needed to rebuild the LGPL DLLs.
$SourceArchive = Join-Path $OutputDir "occt-source-7.9.3.tar.gz"
Push-Location $OcctSource
try {
    Invoke-CMake -E tar czf $SourceArchive --format=gnutar CMakeLists.txt LICENSE_LGPL_21.txt OCCT_LGPL_EXCEPTION.txt README.md adm src
} finally { Pop-Location }
Copy-Item -LiteralPath (Join-Path $Root "docs\OCCT-SOURCE.md"), (Join-Path $Native "occt-options.cmake"), (Join-Path $Native "mingw-toolchain.cmake") -Destination $OutputDir
$RuntimeFiles = @(
    Get-Content -LiteralPath (Join-Path $BackendBuild "install_manifest.txt") |
        Where-Object { $_ -match '\.dll$' } | ForEach-Object { Split-Path -Leaf $_ }
) + @("occt-source-7.9.3.tar.gz", "OCCT-SOURCE.md", "occt-options.cmake", "mingw-toolchain.cmake")
if ($CompilerInfo -match 'set\(CMAKE_CXX_COMPILER_ID "GNU"\)') {
    $Compiler = [regex]::Match($CompilerInfo, 'set\(CMAKE_CXX_COMPILER "([^"]+)"\)').Groups[1].Value
    foreach ($Runtime in @("libgcc_s_seh-1.dll", "libstdc++-6.dll", "libwinpthread-1.dll")) {
        $RuntimePath = (& $Compiler "-print-file-name=$Runtime").Trim()
        if (-not (Test-Path -LiteralPath $RuntimePath -PathType Leaf)) { throw "Cannot locate MinGW runtime $Runtime" }
        Copy-Item -LiteralPath $RuntimePath -Destination $OutputDir
        $RuntimeFiles += $Runtime
    }
}
$RuntimeFiles | Sort-Object -Unique | Set-Content -LiteralPath (Join-Path $OutputDir "runtime-files.txt") -Encoding ascii
Write-Host "STEP runtime and corresponding source: $OutputDir"
