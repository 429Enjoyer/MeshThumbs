# Open CASCADE source and replacement libraries

STEP and IGES use Open CASCADE Technology (OCCT) 7.9.3 under LGPL-2.1 with the
Open CASCADE exception. Copyright notices, the LGPL, and its exception are
included in `THIRD-PARTY-NOTICES.txt` and in the source archive.

The installer includes `step/occt-source-7.9.3.tar.gz`: the unmodified upstream
`src` and `adm` trees, top-level CMake build script, README, and license files.
Samples, test data, and generated documentation are omitted; they are not needed
to rebuild the libraries. The source archive is installed beside the DLLs so
the corresponding source is available offline.

Upstream: <https://github.com/Open-Cascade-SAS/OCCT/tree/V7_9_3>

Original full archive:
<https://codeload.github.com/Open-Cascade-SAS/OCCT/tar.gz/refs/tags/V7_9_3>

SHA-256 of the original full archive:
`5ecf094ec6b12d5413dfb851d8c3590c354058aee556e32e408bdfbf8c357d57`

## Rebuild on Windows

Extract the installed source archive into `occt-source`. With CMake 3.20+ and
an x64 C++ compiler available, run the following from the installed `step`
directory (or a writable copy of it):

```powershell
cmake -S occt-source -B occt-build -C occt-options.cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=occt-install
cmake --build occt-build --config Release --parallel
cmake --install occt-build --config Release
```

Use the same compiler ABI as the installed DLLs. The Windows release uses
MinGW-w64 GCC 13's POSIX-thread compiler; the included `mingw-toolchain.cmake`
supports cross-compilation with that compiler from Linux. Its GCC, C++, and
pthread runtime DLLs are distributed in the same directory. End users do not
need to install a compiler. Exception checks remain enabled in the OCCT release
build; the supplied `occt-options.cmake` records the build configuration.

Stop thumbnail workers before replacing libraries. Compatible modified OCCT
DLLs can replace those in `step/`; MeshThumbs does not check their hashes or
prevent replacement. Preserve the public OCCT 7.9.3 ABI. Changing the ABI also
requires rebuilding the MIT-licensed adapter in `native/step` from the MeshThumbs
source repository. The adapter exports a small versioned C interface to Rust.

Modification of OCCT for your own use and reverse engineering for debugging
those modifications are permitted under the LGPL. This does not change the
license of independent MeshThumbs code or preview assets.
