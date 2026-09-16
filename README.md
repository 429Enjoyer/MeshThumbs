# MeshThumbs

3D model thumbnails for Windows Explorer, with textures, vertex colors, and automatic framing.

**Supported formats**

`OBJ` · `FBX` · `GLB` · `glTF` · `STL` · `DAE` · `PLY` · `3DS` · `3MF` · `VRM`

![MeshThumbs previews for all ten supported 3D formats](docs/images/meshthumbs-preview.png)

## Install

[![Download latest release](https://img.shields.io/github/v/release/429Enjoyer/MeshThumbs?color=2ea44f&labelColor=black&label=Download)](https://github.com/429Enjoyer/MeshThumbs/releases/latest)

Download the `.msi` installer from the latest release and run it to install or upgrade.
Then refresh the model folder.

Uninstall through **Windows Settings → Apps → Installed apps**.

## Build

Requires Windows x64, Rust 1.88+, a C++ compiler, CMake, and WiX 3.14.

```powershell
.\scripts\build-msi.ps1
```

Keep `thumbnail_provider.dll` and `thumbgen.exe` together for manual registration.

## CLI

```powershell
cargo run -p thumbgen -- model.glb preview.png 256
```

<details>
<summary>Rendering details & troubleshooting</summary>

CPU rendering runs in a separate worker, with a **5-second timeout**, a
**300 MiB file limit**, and a **5-million-triangle limit**. Unsupported, corrupt,
or oversized models may not produce a thumbnail.

VRM previews show avatars in their rest pose using base colors and textures;
MToon outlines, animation, and physics are not rendered. Advanced 3MF properties
such as composite materials, beam lattices, and slice-only models are unsupported.
Expanded 3MF package data is also limited to 300 MiB.

Logs: `C:\ProgramData\MeshThumbs\meshthumbs.log`.

</details>

## Changelog

### 1.0.2

- Add 3MF meshes, component assemblies, colors, and textures.
- Add VRM 0.x/1.0 avatar previews.
- Fix fully transparent texels obscuring geometry behind them.

### 1.0.1

- Fix dotted PLY previews by rendering mesh surfaces in full.
- Reject models above five million triangles instead of removing faces.

### 1.0.0

- Initial release with OBJ, FBX, GLB, glTF, STL, DAE, PLY, and 3DS support.

## License

MIT · Fork of [3DThumbnails](https://github.com/While402/3DThumbnails) · [Third-party notices](docs/THIRD-PARTY-NOTICES.txt)

[Preview credits](docs/images/ATTRIBUTION.md)
