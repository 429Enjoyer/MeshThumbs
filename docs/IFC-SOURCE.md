# IFC4 helper sources

MeshThumbs 1.1.4 uses a separate **MinGW build of IfcConvert 0.8.5**, limited to
IFC4 geometry and self-contained GLB output. IFC2x3 retains Assimp. Users need
no Blender, Python, or Visual C++ Redistributable installation. The helper is
not loaded into Explorer and performs no runtime downloads.

## Source and licenses

IfcOpenShell is LGPL-3.0-or-later, copyright its contributors. The project uses
revision [`1c5b825d8ef05ab9d14a15dac12e9eae2f5a37c2`](https://github.com/IfcOpenShell/IfcOpenShell/tree/1c5b825d8ef05ab9d14a15dac12e9eae2f5a37c2).
The upstream Microsoft-compiled executable is **not** redistributed.
MeshThumbs application code remains MIT.

`ifcconvert-source-0.8.5.tar.gz` accompanies the MSI in `ifc/`.
SHA-256: `d4e95a9cee6a5d4f7dd118cb3365b2fb056a6e586085fe9ea087472de1b9ac45`.
It preserves the upstream C++ converter/parser/geometry/serializer sources,
schema generator, CMake files, build recipes and license texts. Independent
applications and Python tests/examples/docs are omitted. Retained files are
unchanged; compatibility edits are supplied separately in `patch-ifc.cmake`.
The [complete upstream archive](https://codeload.github.com/IfcOpenShell/IfcOpenShell/tar.gz/1c5b825d8ef05ab9d14a15dac12e9eae2f5a37c2)
has SHA-256 `6d24345ad08936f68ffcb947e5cedbc674e8c04db6a5a25432462883ecfcf886`.

The compatibility patch replaces the obsolete `BRepOffsetAPI_Sewing` alias
with its underlying `BRepBuilderAPI_Sewing` class in `base_utils.cpp`,
`layerset.cpp`, and `wire_utils.cpp`. OCCT 7.9 removed the old header.
MinGW also needs `-Wa,-mbig-obj` for the generated IFC4 schema translation unit.
The MinGW real-number fallback also needed a fix: `tellg()` returns -1 at EOF,
which rejected complete decimal tokens. The patch handles EOF and failed reads
explicitly in `IfcParse.cpp`; `check-float.cpp` links the real parser and tests
six valid numbers and five invalid tokens. The preparation step runs this check.
`buildinfo.cpp` labels this custom build `0.8.5-meshthumbs-mingw`. Geometry
algorithms and the IFC schema are unchanged.

| Dependency | Source / license |
| --- | --- |
| Open CASCADE 7.9.3 | [V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/tree/V7_9_3), LGPL-2.1 with OCCT exception. Full source is already installed in `../step/occt-source-7.9.3.tar.gz`. |
| Boost 1.86.0 | [Complete source archive](https://github.com/boostorg/boost/releases/download/boost-1.86.0/boost-1.86.0-b2-nodocs.tar.xz), Boost Software License 1.0. SHA-256 `a4d99d032ab74c9c5e76eddcecc4489134282245fffa7e079c5804b92b45f51d`. |
| Eigen 3.3.9 | [3.3.9](https://gitlab.com/libeigen/eigen/-/tree/3.3.9), MPL-2.0 and per-file permissive terms. Included as `eigen-source-3.3.9.tar.gz`; SHA-256 `7985975b787340124786f092b3a07d594b2e9cd53bbfe5f3d9b1daee7d55f56f`. |
| JSON for Modern C++ 3.6.1 | [Original single-header source](https://raw.githubusercontent.com/nlohmann/json/v3.6.1/single_include/nlohmann/json.hpp), MIT. Included as `json-3.6.1.hpp`; SHA-256 `d2eeb25d2e95bffeb08ebb7704cdffd2e8fca7113eba9a0b38d60a5c391ea09a`. |
| MinGW GCC/C++/pthread runtime | Same runtime family as the CAD backend; notices and exceptions are in `../THIRD-PARTY-NOTICES.txt`. |

Full helper license texts are in `IFC-NOTICES.txt`. Preserve the source archive,
patches, build recipes and dependency source access with the helper. For network
distribution, keep this information available alongside the MSI; for offline
redistribution, also supply the linked Boost source archive.
The binary and every staged dependency have SHA-256 entries in `build-info.json`,
which also records compiler version and build-input hashes.

CGAL, MPIR, MPFR, HDF5, OpenCOLLADA, PROJ, RocksDB, IFCXML and Microsoft runtime
libraries are disabled or absent from this build. Private OCCT/GCC DLLs live
beside the helper; the installer does not replace system runtime libraries.

## Rebuild

On Windows, use WSL Ubuntu with `cmake`, `curl`, `python3`, `g++`, `make`,
`xz-utils`, and the POSIX MinGW x64 C/C++ compilers. Then from the repository root:

```powershell
.\scripts\prepare-ifc.ps1
```

This runs `scripts/build-ifc-mingw.sh`, verifies all source downloads, builds
Open CASCADE and the IFC4 helper, and stages the files in `target/release/ifc`.
The first native build takes longer; later builds reuse `~/.cache/meshthumbs-ifc`.
`build-msi.ps1` invokes this step unless `-SkipBuild` is used.

Optional `-CacheDir` and `-OcctPrefix` accept **Linux paths** for reusing an
existing WSL cache and MinGW OCCT SDK. That SDK must include the extra modeling
toolkits in `native/ifc/occt-options.cmake`, not just the STEP import subset.
`-Distribution` defaults to `Ubuntu`; `-Jobs` defaults to four.

The installed `build-ifc-mingw.sh`, `patch-ifc.cmake`, `ifc-occt-options.cmake`,
and the CAD backend's toolchain/options files record the build recipe; the same
files are in the source repository. Build a modified compatible helper and
replace the installed executable/DLLs to use it. Packaging verifies the build
manifest, but MeshThumbs imposes no runtime signature or checksum restriction.
The protocol is the documented IFC input / GLB output CLI, with ASCII temporary
leaf names and a Unicode Windows working directory.
