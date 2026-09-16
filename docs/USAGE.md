# Usage & Build

[Back to MeshThumbs](../README.md)

Run the commands below from the repository root.

## CLI

Generate a PNG thumbnail from a model file. The final argument sets the image size in pixels.

```powershell
cargo run -p thumbgen -- model.glb preview.png 256
```

## Build

Requires Windows x64, Rust 1.96+, a C++ compiler, CMake, and WiX 3.14.

```powershell
.\scripts\build-msi.ps1
```

The MSI installer is written to the repository root.

When adding a format, update its renderer, CLI, Explorer registration, installer,
and documentation together. Include an actual render in the 1920×1080 README
preview, check its source license, and update the asset credits and changelog.
Check dependency notices whenever the dependency graph changes.

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

X3D supports static XML mesh/primitive scenes, DEF/USE instances, materials, and
local image textures. Animation and scripts are not executed. Text geometry,
Inline scenes, prototypes, remote textures, and non-XML encodings are unsupported. Keep local textures
beside the model or in their referenced relative folders.

OFF supports ASCII OFF, COFF, NOFF, and CNOFF files, including RGB/RGBA vertex
colors in 0–1 or 0–255 ranges. Binary OFF, per-face colors, and other header
variants are unsupported. Line-only and point-only files have no mesh thumbnail.

WRL and VRML support UTF-8 VRML 2.0 / VRML97 scenes: Group, Transform, Shape,
IndexedFaceSet, Box, Sphere, Cylinder, Cone, DEF/USE instances, materials,
vertex/face colors and normals, and local ImageTexture/TextureTransform nodes.
Camera, lighting, background, and navigation nodes are parsed but do not control
the thumbnail. VRML 1.0, compressed VRML, PROTO/EXTERNPROTO, Inline, scripts,
ROUTE/animation, Switch/LOD, text, lines, points, and other node types are
unsupported. Unsupported scene nodes cause the preview to fail rather than
silently omit geometry. Parsing is bounded to 64 nested nodes, 100,000 nodes,
and 300 MiB of expanded data; DEF/USE expansion also has node and depth limits.
Keep local textures beside the model or in their referenced relative folders.
Both extensions require a filesystem-backed item in Explorer to resolve textures.

USD, USDA, USDC, and USDZ are read directly, without an external USD application.
Previews use the stage's start time and support polygon meshes, Cube/Sphere/
Cylinder/Cone primitives, transforms, local references, material subsets,
display colors, and UsdPreviewSurface base-color textures. USDZ contents and
referenced source assets share a 300 MiB read budget. Missing optional textures
fall back to material colors; unresolved scene references may prevent a preview.
Explorer uses the original file path for USD/USDA/USDC and X3D so relative assets
remain resolvable. These formats require a filesystem-backed item; USDZ can also
be read from an anonymous stream because its resources are packaged together.
Subdivision uses the control mesh. Skinning, simulation, scene lighting,
PointInstancer geometry, and nested USDZ packages are unsupported. Other shader
graphs, including MaterialX, use an untextured fallback; this is not a full PBR render.

Logs: `C:\ProgramData\MeshThumbs\meshthumbs.log`.
