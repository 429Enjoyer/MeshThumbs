# MeshThumbs

Windows Explorer thumbnails for **OBJ, FBX, GLB, glTF, STL, DAE, PLY, and 3DS**.
CPU rendering with textures, vertex colors, and automatic framing. Rendering runs
in a separate worker with a five-second timeout and a 300 MiB input-file limit.

Fork of [3DThumbnails](https://github.com/While402/3DThumbnails).

## Install

Run `MeshThumbs-1.0.0-x64.msi`, then refresh the model folder. Uninstall through
Windows Installed apps. Unsupported or corrupt models may not produce a thumbnail.

## Build

Requires Windows x64, Rust, a C++ compiler, CMake, and WiX 3.14.

```powershell
.\scripts\build-msi.ps1
```

Keep `thumbnail_provider.dll` and `thumbgen.exe` together for manual registration.

## CLI

```powershell
cargo run -p thumbgen -- model.glb preview.png 256
```

Logs: `C:\ProgramData\MeshThumbs\meshthumbs.log`.

## License

MIT. See [third-party notices](THIRD-PARTY-NOTICES.txt).
