# Usage & Build

[Back to MeshThumbs](../README.md)

Run the commands below from the repository root.

## CLI

Generate a PNG thumbnail from a model file. The final argument sets the image size in pixels.

```powershell
cargo run -p thumbgen -- model.glb preview.png 256
```

## Build

Requires Windows x64, Rust 1.88+, a C++ compiler, CMake, and WiX 3.14.

```powershell
.\scripts\build-msi.ps1
```

The MSI installer is written to the repository root.

## Manual registration

Keep `thumbnail_provider.dll` and `thumbgen.exe` together for manual registration.
Registration scripts are available for the [current user](../scripts/register-current-user.ps1)
or [all users](../scripts/register-machine.ps1).

## Rendering & Troubleshooting

CPU rendering runs in a separate worker, with a **5-second timeout**, a
**300 MiB file limit**, and a **5-million-triangle limit**. Unsupported, corrupt,
or oversized models may not produce a thumbnail.

VRM previews show avatars in their rest pose using base colors and textures;
MToon outlines, animation, and physics are not rendered. Advanced 3MF properties
such as composite materials, beam lattices, and slice-only models are unsupported.
Expanded 3MF package data is also limited to 300 MiB.

BLEND files use their saved preview image; Blender does not need to be installed.
Files without an embedded preview show no thumbnail. Little-endian legacy and
Blender 5.0+ headers are supported, including gzip and Zstandard compression.
Previews are resized with their original framing and aspect ratio; scenes are
not re-rendered.

Logs: `C:\ProgramData\MeshThumbs\meshthumbs.log`.
