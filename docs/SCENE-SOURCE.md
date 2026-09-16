# Alembic and Rhino scene backend

`scene/meshthumbs_scene.dll` is loaded only when rendering `.abc` or `.3dm`.
Its small C interface adapter is MIT-licensed. The statically linked libraries
retain their upstream licenses and are identified in `THIRD-PARTY-NOTICES.txt`:

| Library | Pinned source | Terms |
| --- | --- | --- |
| Alembic 1.8.8 | [43a1489](https://github.com/alembic/alembic/tree/43a1489a0f5e15420e4be7225df86e819884b6fa) | BSD-style license and attached notices |
| Imath 3.1.12 | [c0396a0](https://github.com/AcademySoftwareFoundation/Imath/tree/c0396a055a01bc537d32f435aee11a9b7ed6f0b5) | BSD 3-Clause |
| openNURBS 8.x | [eb92af3](https://github.com/mcneel/opennurbs/tree/eb92af3ba1806b0a34a99aba0d3bda83e3d46083) | McNeel openNURBS terms, with bundled zlib notices |

Alembic and Imath archives are verified with SHA-256 before extraction:

- Alembic: `ba1f34544608ef7d3f68cafea946ec9cc84792ddf9cda3e8d5590821df71f6c6`
- Imath: `8a1bc258f3149b5729c2f4f8ffd337c0e57f09096e4ba9784329f40c4a9035da`

openNURBS is checked out at the exact Git revision, with a sparse checkout of
its root sources and library directories. `native/scene/patch-opennurbs.cmake`
applies two portability fixes: standard wide-string macro expansion in a
diagnostic, and use of the Windows UUID API with MinGW. These are MeshThumbs
changes, not unmodified upstream code.

`native/scene/compat/` supplies MinGW SDK/CRT compatibility declarations without
changing the upstream file readers. Secure variadic scan functions use Windows'
UCRT and a C locale allocated by that same runtime. No application scripts,
Alembic rigs, or Rhino plug-ins are executed. HDF5 and OpenNURBS meshing engines
are not included; see `docs/USAGE.md` in the source repository for format limits.

Run `scripts/build-scene.ps1` from the MeshThumbs source tree to rebuild. It
requires Git, CMake 3.20+, and an x64 C++17 compiler. MinGW requires its POSIX
thread variant. The 1.0.8 distribution uses MinGW-w64 GCC 13 and includes its
GCC/C++/pthread runtime DLLs in `scene/`, so users need no developer toolchain.
Linux cross-builds use `native/step/mingw-toolchain.cmake`; Alembic additionally
uses the `native/scene/compat` include directory for `Windows.h` capitalization.
