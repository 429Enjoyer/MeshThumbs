# Usage & Build

[Back to MeshThumbs](../README.md)

Run the commands below from the repository root.

## Install and upgrade

The 1.1.1 MSI upgrades earlier releases, including 1.1.0 and local 1.0.10 builds. Matching
versions are also treated as upgrades. The previous release is removed inside
the upgrade transaction after the new shared components are installed.

The interactive installer uses the standard WiX Minimal welcome/license,
progress, completion, and repair/remove dialogs. The license page displays the
project MIT license. Only the files-in-use dialog is customized to add the
Restart Explorer button.

If the installer lists **Windows Explorer** as using a file, closing folder
windows may not be enough: Explorer also runs the desktop and taskbar. In the
interactive installer, click **Restart Explorer**, wait for the desktop to
return, then click **Retry**. This forcibly closes Explorer's folder windows
and interrupts its file operations; finish copies/moves before using it.
The button only targets the current user's Windows Explorer processes in the
current session. It waits for Windows recovery before using one fallback launch,
so it does not deliberately open another Explorer when one has already returned.
Other applications in the list must be closed separately.

The restart script is embedded in the MSI and works before the old installation
is replaced. It only runs on a button click; silent/basic-UI deployments do not
execute it. Double-click the rebuilt MSI for the custom dialog (or use `/qf`).
`/qb` uses Windows Installer's built-in dialog without the extra button; `/qn`
remains unattended. If an older installer dialog is already open, cancel it
and reopen the rebuilt MSI. The **Ignore** option may require a Windows restart.

The installer uses `MSIRESTARTMANAGERCONTROL=DisableShutdown`: Restart Manager
still detects files in use, but the package does not ask it to automatically
shut down Explorer. If a DLL remains locked, Windows Installer may request a
Windows restart to complete file replacement. Honor that request before judging
the new thumbnails; the old DLL can remain active until then. Reboot requests
are not suppressed. Silent deployments should handle MSI exit code 3010.

This avoids the failed shutdown/restart path observed in Windows event logs
(Restart Manager 10006 and 10010, including an application/conductor SID mismatch).
The provider now permits COM to unload its DLL after the last factory, provider
object, and server lock is released; an active thumbnail request keeps it loaded.
Unloading is controlled by Windows and is not guaranteed to happen immediately.

The refresh action only removes stale current-user overrides and sends a shell
association notification. It does not stop/start Explorer or delete open cache
databases. The explicit Restart Explorer button is a separate UI action and
does not delete caches or registry entries. Old-product removal skips the
refresh notification during an upgrade.
The cached uninstaller of a previous build still carries its own policy; in
particular, original 1.0.8 and earlier builds can run their old restart script.
The new package cannot rewrite those cached actions. A first transition from
an older package may still require a reboot or encounter its restart behavior.

For an explicit manual deep cache reset, `scripts/clear-explorer-cache.ps1`
still clears cache files and restarts Explorer in the current session only.
It first waits for Windows' automatic recovery and starts Explorer only if it
remains absent. `-NoRestartExplorer` suppresses that manual recovery;
`-RefreshOnly` performs notification and stale override cleanup without any
process/cache-file changes. The MSI always uses `-RefreshOnly`.

This follows Microsoft's guidance for
[shell handler notification](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shchangenotify)
and [Restart Manager control](https://learn.microsoft.com/en-us/windows/win32/msi/msirestartmanagercontrol).

## CLI

Generate a PNG thumbnail from a model file. The final argument sets the image size in pixels.

```powershell
cargo run -p thumbgen -- model.glb preview.png 256
```

## Build

Requires Windows x64, Rust 1.96+, an x64 C++ compiler, CMake 3.20+, and WiX 3.14.
Include `WixUIExtension.dll` beside the WiX tools; packaging uses the Windows
.NET Framework 4 C# compiler for the small UI build adapter. See
[installer UI sources](WIX-UI-SOURCE.md) for the pinned library and licenses.
The native backends need Git and a C++17 compiler; MinGW builds require its POSIX-thread
variant. The first build downloads the pinned Open CASCADE 7.9.3 source and
compiles it, which takes substantially longer than incremental Rust builds.
The scene backend also downloads pinned Alembic, Imath, and openNURBS sources.

```powershell
.\scripts\build-msi.ps1
```

The MSI installer is written to the repository root.

For CLI development, run `scripts/build-step.ps1` once to put the CAD backend
in `target/release/step`, then build/run `thumbgen` with `--release`. Use
`scripts/build-step.ps1 -Configuration debug` for the default debug CLI build.
Keep the `step` directory beside `thumbgen.exe` when copying a build. Other
formats remain usable when the optional development backend has not been built;
the MSI always includes it. Run `scripts/build-scene.ps1` (or add
`-Configuration debug`) for the Alembic/3DM backend, and keep its `scene` directory
beside `thumbgen.exe` too. See [scene backend sources](SCENE-SOURCE.md).
The backends can also be cross-compiled from Linux
using `native/step/mingw-toolchain.cmake`. See [OCCT source and rebuild notes](OCCT-SOURCE.md).

When adding a format, update its renderer, CLI, Explorer registration, installer,
and documentation together. Include an actual render in the 1920×1080 README
preview, check its source license, and update the asset credits and changelog.
Keep the matching models, required textures, and license texts in `examples/`.
Check dependency notices whenever the dependency graph changes.

## Manual registration

Keep `thumbnail_provider.dll` and `thumbgen.exe` together for manual registration.
Include the accompanying `step` directory for STEP previews.
Include `scene` for Alembic/3DM; IGES uses `step` as well.
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

STEP and STP read self-contained ISO 10303-21 text files using Open CASCADE
7.9.3. CAD solids, trimmed surfaces, and assembly placements are tessellated
with a chord tolerance of 0.1% of the model's largest bounding-box dimension
(minimum 0.0000001 model units). Meshes use smooth surface normals and a neutral
material; CAD colors, textures, PMI/annotations, and wire-only geometry are not
rendered. External assembly files are not followed, so only geometry stored in
the main file is shown. Compressed STEP and STEP XML are unsupported.
The CAD reader retains exception checks, rejects more than 100,000 roots/faces,
and shares the worker's five-second deadline and five-million-triangle limit.
Large or difficult CAD assemblies can exceed that deadline and show no preview.
Both extensions support file, item, and anonymous stream initialization.
Recentered double-precision CAD coordinates are converted to the thumbnail
renderer's coordinate system before rendering, treating CAD Z as up.

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

## Formats added in 1.0.8

Alembic (`.abc`) reads **Ogawa** archives. It uses the earliest authored animated
sampling time (zero for static archives), sampling every property at or before
that time. Polygon meshes, local transform hierarchies, transform inheritance,
visibility, indexed normals, and subdivision control cages with holes are
supported. Missing normals use face normals. Subdivision refinement, curves,
points, NURBS patches, materials/textures, and HDF5 archives are unsupported.
The reader treats Alembic coordinates as Y-up and converts clockwise polygon
winding for rendering. Non-mesh objects do not contribute thumbnail geometry.

IGES (`.igs`, `.iges`) uses Open CASCADE to read surfaces, solids, and their
placements. It shares STEP's tessellation, neutral material, precision handling,
face/triangle limits, and Z-up convention. Curves/points alone have no thumbnail;
annotations, CAD colors, textures, and externally referenced files are not rendered.

Rhino (`.3dm`) reads mesh objects and **saved render meshes** on Breps and
extrusions. Local block instances, object/layer visibility, object/layer colors,
vertex colors, normals, and Z-up coordinates are supported. Default black layer
wireframe colors use the neutral thumbnail material. It does not calculate new
NURBS/SubD meshes: a visible surface without the required cached mesh causes
the preview to fail. Save with render meshes in Rhino or export mesh objects.
Linked external blocks, textures, per-instance inherited colors, curves, points,
and annotations are unsupported. Mesh files from Rhino 5, 7, and 8 were checked.

IFC (`.ifc`) uses Assimp's **IFC2x3** reader, with product placements, common
swept-solid/profile and faceted building geometry, and basic surface colors.
Space representations are omitted. This is a building geometry preview, not a
complete BIM viewer: unsupported representation types may be omitted. IFC4/4.3,
IFCZIP, IFCXML, annotations, and full material/texture graphs are unsupported.

ABC, IGS, IGES, 3DM, and IFC accept file, item, or anonymous stream initialization.
The native scene reader limits hierarchy depth to 64, visits to 100,000, polygons
to 4,096 corners, and expanded polygon storage to 300 MiB. All formats retain
the Explorer worker's five-second deadline and five-million-triangle ceiling;
complex models can exceed those limits and produce no thumbnail.


## PMX, VOX, and LWO

PMX (`.pmx`) uses the bundled Assimp reader for PMX 2.0/2.1 model geometry in
its rest pose, diffuse colors, opacity, normals, UVs, and local diffuse textures.
MMD does not need to be installed. Motion files, morph animation, IK, physics,
toon shading/outlines, and sphere maps are not applied. PMD and VMD are not
registered. Keep the model's texture folders beside it.

MagicaVoxel (`.vox`) uses a native Rust reader for versions 150 and 200, with
the default or embedded RGBA palette. It emits only exposed voxel faces and
supports scene groups, instances, integer rotations/translations, and hidden
nodes/layers. Each animated transform or shape uses its earliest authored
frame; older PACK animation files show their first model. Model pivots and
Z-up coordinates are converted for rendering. MATL/MATT shading, emission,
glass/refraction, cameras, lighting, and animation playback are not rendered;
palette RGBA determines thumbnail color. Parsing rejects invalid coordinates,
references, cycles, counts, and truncated data. Limits include eight million
stored voxels, 32 million instanced voxels, 256 million expanded grid cells,
100,000 chunks/node visits, and 64 hierarchy levels, plus the shared triangle
limit and Explorer deadline. MagicaVoxel does not need to be installed.

LightWave (`.lwo`) uses Assimp for polygon meshes, layers, surface/vertex colors,
normals, UVs, and local diffuse image textures. LWOB and LWO2 samples were
checked. Subdivision surfaces show their control mesh, not a subdivided result.
Procedural/node materials and animation are not reproduced. Separate LWS scenes
and LXO mesh files are described below. LightWave does not need to be installed.

PMX and LWO require file or filesystem-backed item initialization in Explorer
so relative textures retain their original paths. Anonymous streams are not
advertised for these two formats. VOX is self-contained and accepts all three
initialization methods. Missing optional textures fall back to material colors.


## SMD, MD2, MD3, and MD5MESH

These game-model formats use the bundled Assimp reader; a game or modeling
application does not need to be installed. All four require a file or
filesystem-backed item in Explorer to resolve local textures. Anonymous streams
are not advertised. The same file, triangle, and Explorer timeout limits apply.

- **SMD:** version 1 reference mesh geometry, stored normals, and UVs. Model-space
  Z-up coordinates are converted to Y-up. Skeleton-only/animation-only SMDs have
  no preview; bone animation, VTA morphs, and animation-list autoloading are disabled.
- **MD2:** version 8 geometry, normals, UVs, the first vertex-animation frame,
  and the first declared skin. Later frames and GL-command rendering are not used.
- **MD3:** version 15 surfaces, normals, UVs, and the first vertex-animation frame.
  Local `<model>_default.skin` mappings may override surface image names. Each
  selected file is rendered independently: adjacent head/upper/lower files are
  not assembled, and tags, animation, and Quake shader scripts are not rendered.
- **MD5MESH:** version 10 mesh geometry reconstructed from joint transforms and
  weighted vertex offsets in the bind pose, with UVs and local diffuse textures.
  LF and CRLF line endings are accepted. MD5ANIM and MD5CAMERA are not registered
  or automatically loaded.

Keep referenced textures beside the model or in its relative subfolders. Exact
paths are tried first, with a filename-only fallback beside the model. Image
names without extensions try TGA, PNG, JPEG, DDS, BMP, then PCX. An extensionless
MD5 shader name uses Assimp's `<name>_d.tga` diffuse convention. PCX v5 supports
8-bit indexed images with a trailing 256-color palette, or three 8-bit RGB planes,
with raw or scanline RLE pixels; limits are 64 MiB input/decoded scanlines,
8192 pixels per dimension, and 16,777,216 pixels. Other PCX variants are unsupported.
PCX is a texture decoder, not an additional model extension.

Missing or unsupported textures fall back to material colors. Game installations,
PAK/PK3/PK4 archives, VMT/VTF materials, Doom material declarations, shader effects,
and normal/specular maps are not resolved. Extract geometry and supported image
textures before requesting a thumbnail.


## ASE, LXO, LWS, and DXF

These four extensions use the bundled Assimp reader. No 3ds Max, Modo,
LightWave, or CAD application is required.

- **ASE:** static mesh exports, object transforms, materials, vertex colors,
  UVs, and local diffuse textures. Plain text, UTF-8 BOM, and UTF-16 LE/BE BOM
  inputs are supported. Geometry uses the reference mesh; animation, skeletal
  deformation, cameras, lights, and procedural material graphs are not rendered.
- **LXO:** polygon mesh layers stored in Modo's LXOB container, with layer
  pivots, vertex colors, normals, UVs, and supported diffuse image materials.
  This is a mesh-layer preview: Modo ITEM/CHAN scene-item transforms, instances,
  deformation, procedural geometry, and full shader graphs are not evaluated.
  Subdivision surfaces show their control mesh. Tested with real LXOB exports.
- **LWS:** UTF-8 LWSC 3–5 scenes referencing local LWOB/LWO2/LXOB `.lwo` objects,
  including LoadObject/LoadObjectLayer, object parenting, pivots, and transforms
  from the initial authored channel keys. Local object paths are resolved beside
  the scene or up to two parent folders for packaged Scenes/Objects layouts.
  Texture paths are rebased against each object before merging, so same-named
  textures in different folders remain distinct. Missing objects, nested scene
  references, invalid parenting, and truncated object containers fail the preview.
  Plugins are skipped; animation playback, LWO3 objects, LWSC 1/2, external scene
  nesting, object visibility/dissolve settings, and full LightWave rendering are unsupported.
  Limits are 1,024 object references, 4,096 nodes, 64 nested blocks/parent levels,
  100,000 source lines, and 300 MiB of collected/normalized package data.
- **DXF:** ASCII 3DFACE and POLYLINE polyface meshes, with indexed entity/vertex
  colors and Z-up conversion. Block INSERTs, ACIS solids, REGION/BODY/SURFACE,
  modern MESH, SOLID and HATCH entities are rejected; explode or export surfaces
  as 3DFACE/polyfaces first. Lines, curves and annotations are not tessellated;
  line-only drawings have no thumbnail. Binary DXF, textures, true-color/layer
  material inheritance, and full CAD document rendering are unsupported. Parsing
  is limited to ten million group-code pairs and requires the EOF record.

ASE, LXO, and LWS use file or filesystem-backed item initialization in Explorer
to retain their sidecar paths. Keep referenced objects and textures in place.
DXF geometry is self-contained and also supports anonymous streams. The shared
five-second Explorer deadline and five-million-triangle ceiling still apply.
