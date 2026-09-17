# Usage & Build

[Back to MeshThumbs](../README.md)

[Install](#install-and-upgrade) · [Export PNG](#export-png-thumbnails) ·
[CLI](#cli) · [Build](#build) · [Troubleshooting](#rendering--troubleshooting) ·
[Format limits](#format-limits)

## Install and upgrade

Run the latest MSI to install, upgrade, or replace a matching version.
Uninstall through **Windows Settings → Apps → Installed apps**.

If setup reports Explorer files in use, finish any copies or moves, click
**Restart Explorer**, wait for the desktop to return, then click **Retry**.
Closing folder windows alone may leave Explorer running. The button forcibly
closes the current user's Explorer windows; close other listed apps separately.
If setup requests a Windows restart, restart to finish replacing locked files.
The completion page also has **Restart Explorer** to recover a missing desktop
or taskbar without opening Task Manager. It checks that both shell windows are
responding; a background `explorer.exe` alone does not count as recovery.

The installer refreshes file associations without automatically restarting
Explorer. Old cached uninstallers retain their own restart policy; upgrading
from them can still invoke their older behavior.

For deployment: `/qf` uses the full UI with **Restart Explorer**; `/qb` uses
basic Windows Installer UI; `/qn` runs silently. Handle exit code **3010** as
a required restart. The MSI uses `MSIRESTARTMANAGERCONTROL=Disable` to fully
disable Restart Manager interaction and uses the classic files-in-use dialog.
Reboot requests are not suppressed. `DisableShutdown` was insufficient with the
bundled Restart Manager UI; Windows logged failed Explorer shutdown/recovery
(events 10006 and 10010). See [Microsoft's property documentation](https://learn.microsoft.com/en-us/windows/win32/msi/msirestartmanagercontrol).

## Export PNG thumbnails

Select supported model files, right-click → **MeshThumbs**, and choose
**PNG 256 × 256**, **PNG 512 × 512**, or **PNG 1024 × 1024**. On Windows 11, use
**Show more options** first. Folders and selections containing unsupported files
do not show the menu. The MSI installs and removes the menu automatically.

Each PNG is saved beside its model: `Chair.fbx` → `Chair.png`.
**Any existing matching PNG is replaced without confirmation**, including a
same-named texture. For models such as `boxuv.lwo` with `boxuv.png` as a texture,
use the [CLI](#cli) to choose a different output name.

Files run in filename order. Models with the same basename share one output;
the last successful export wins. A failed or cancelled render preserves the old
PNG. Read-only destinations produce an error without elevation.

One dialog shows progress and offers **Cancel**. Completed PNGs remain; individual
failures do not stop the batch and are listed in the final summary. Rendering
runs outside Explorer. Each file has a 30-second deadline and a 2 GiB worker
memory limit; BLEND has the larger explicit-export budget below and IFC4 keeps
its shorter converter limits. Selections are
limited to 10,000 files and 16 MiB of path data.

For **BLEND**, the menu and `--export-png` try installed Blender to read real
geometry before rendering it with MeshThumbs. If Blender is missing, fails,
or times out, the saved preview is used and identified in the final summary.
If neither works, the export fails without replacing the old PNG.

## CLI

From the installation folder, render to a chosen filename:

```powershell
.\thumbgen.exe model.glb model-preview.png 512
```

The optional size defaults to 256 and is clamped to 32–1024 pixels.
To export beside multiple models using the same replacement rules as the menu:

```powershell
.\thumbgen.exe --export-png 512 "C:\Models\Chair.fbx" "C:\Models\Table.obj"
```

Batch mode accepts 256, 512, or 1024, prints progress and errors, and returns
a nonzero exit code if any file fails. `meshthumbs-export.exe` is the menu's GUI
helper; use `thumbgen.exe` for command-line work.

## Build

Run build commands from the repository root. Requirements:

- Windows x64, Rust 1.96+, Git, CMake 3.20+, and an x64 C++17 compiler.
- WiX 3.14.1 with `WixUIExtension.dll`; packaging also uses Windows' .NET
  Framework 4 C# compiler. See [installer UI sources](WIX-UI-SOURCE.md).
- WSL Ubuntu and POSIX-thread MinGW compilers for the IFC4 helper.
  See [IFC build prerequisites and cache options](IFC-SOURCE.md#rebuild).

```powershell
.\scripts\build-msi.ps1
```

The first build downloads and compiles pinned native sources; later builds reuse
them. The MSI is written to the repository root. `-SkipBuild` packages already
staged files, which must match the current source and documentation.

For CLI development:

```powershell
cargo run -p thumbgen -- model.glb preview.png 256
```

Build native backends as needed with `scripts/build-step.ps1`,
`scripts/build-scene.ps1`, and `scripts/prepare-ifc.ps1`. Use `-Configuration debug`
for a debug CLI build; their default is release. Build STEP before the scene
backend, which needs the matching OCCT SDK.

Keep `thumbnail_provider.dll`, `thumbgen.exe`, and `meshthumbs-export.exe`
(package `png_export`) together. Keep the `step`, `scene`, and `ifc` folders
beside them for CAD, Alembic/Rhino, and IFC4 previews. The MSI includes all three.
See [CAD rebuild notes](OCCT-SOURCE.md) and [scene rebuild notes](SCENE-SOURCE.md)
for compiler/ABI requirements and Linux cross-builds.

When adding a format, update the loader, CLI, Explorer registration, installer,
and docs together. Test a licensed internet example first, keep its files and
credits in `examples/`, and refresh the 1920×1080 README preview and changelog.
Update dependency notices when dependencies or compilers change.

## Manual registration

For development, register thumbnails for the [current user](../scripts/register-current-user.ps1)
or [all users](../scripts/register-machine.ps1). Keep the binaries and backend
folders together as described above. These scripts register thumbnail providers;
use the MSI to install the PNG context menu.

## Rendering & Troubleshooting

Automatic Explorer previews render in a separate CPU worker with a **5-second
timeout**, **300 MiB mesh file limit**, and **5-million-triangle limit**. BLEND
saved previews only read a bounded prefix, so the total file can exceed 300 MiB. Corrupt,
unsupported, or complex models may have no preview. PNG export uses the same
renderer and format limits, with the longer deadline described above.

- Keep referenced textures and scene objects in their original relative folders.
  Missing textures generally fall back to material colors; missing geometry
  references may prevent a preview.
- For BLEND, save an embedded preview in Blender. MeshThumbs reads that image
  without needing Blender installed.
- For curved 3DM surfaces or SubD, save render meshes in Rhino or export meshes.
- After an upgrade, complete any Windows restart requested by setup.
- Check `C:\ProgramData\MeshThumbs\meshthumbs.log` for thumbnail-provider errors.

For a manual cache reset, `scripts/clear-explorer-cache.ps1` clears thumbnails
and restarts the current user's Explorer. Finish file operations first.
`-NoRestartExplorer` skips restarting; `-RefreshOnly` only refreshes associations
and stale overrides, as the MSI does.

## Format limits

Previews show static geometry and basic materials. Animation playback, simulation,
and full application-specific rendering are not reproduced.

### VRM, PMX, and 3MF

VRM and PMX 2.0/2.1 show rest-pose geometry, base colors, and supported textures.
Toon outlines, physics, IK, animated morphs, and sphere maps are not applied.
Keep PMX texture folders beside the model; PMD and VMD are not supported.
3MF composite materials, beam lattices, and slice-only models are unsupported;
expanded package data is limited to 300 MiB.

### BLEND

Automatic Explorer thumbnails and the explicit-output CLI use the saved preview
image, preserving its framing and aspect ratio. Supports
little-endian legacy and Blender 5.0+ headers, gzip, and Zstandard compression.
Files without an embedded preview have no automatic thumbnail.
Preview reads are limited to 32 MiB of decompressed header data and four million
preview pixels, independent of the total BLEND size. Anonymous Shell streams
also copy at most the first 32 MiB.

The context menu and `--export-png` can instead read geometry through **Blender
3.6 or newer** (tested with 4.0). MeshThumbs searches standard Blender Foundation
installation folders and `PATH`. For portable/custom installations, set the
`MESHTHUMBS_BLENDER` environment variable to the absolute path of `blender.exe`.
An invalid override uses the saved-preview fallback; restart Explorer after
changing its environment so the context menu receives the setting.

Conversion uses the active scene/view layer at its saved frame, evaluated mesh
and curve geometry, instances, viewport modifiers, and supported glTF materials.
Hidden objects are omitted. MeshThumbs supplies framing and lighting; Cycles/EEVEE
effects, procedural shaders, volumes, simulations, and source cameras/lights are
not reproduced. Missing linked assets can limit the result.

Large BLEND sources are accepted for manual export. The converted GLB is limited
to **1 GiB and ten million triangles**, with referenced textures resized in memory
to at most 1024 pixels per side. Repeated instances share converted mesh data.

Blender runs hidden with factory startup, automatic file scripts disabled, and
two requested threads. Conversion and the subsequent mesh render each have a
**120-second deadline**. Each process gets a memory budget based on one quarter
of installed RAM or half of currently available RAM, whichever is smaller,
bounded to 512 MiB–8 GiB. This reserves capacity for other applications.
MeshThumbs starts only one Blender conversion per Windows session;
other exports wait up to 120 seconds, with Cancel available, then fall back.
Cancel terminates the child process and preserves the previous PNG. The original BLEND is never
saved or modified. Blender is optional and is not bundled with the MSI.

### X3D, WRL / VRML, and OFF

X3D reads static XML meshes/primitives, DEF/USE instances, materials, and local
textures. Non-XML encodings, Inline scenes, prototypes, text, scripts, animation,
and remote textures are unsupported.

WRL/VRML reads UTF-8 VRML 2.0 / VRML97 meshes/primitives, transforms, DEF/USE,
colors, normals, and local textures. VRML 1.0, compression, prototypes, Inline,
Switch/LOD, scripts, animation, text, lines, and points are unsupported.
Unsupported scene nodes fail the preview; source cameras and lights do not
control it.

OFF reads ASCII OFF, COFF, NOFF, and CNOFF, including vertex colors and normals.
Binary OFF, per-face colors, and line/point-only files are unsupported.

### USD / USDA / USDC / USDZ

Reads meshes, basic primitives, transforms, local references, material subsets,
display colors, and UsdPreviewSurface base-color textures at the stage's start
time. USDZ assets are read directly from the package. Referenced assets share a
300 MiB read budget.

Subdivision shows the control mesh. Skinning, simulation, PointInstancer,
nested USDZ packages, and full shader graphs are unsupported. MaterialX and
other shaders use an untextured fallback. Missing scene references may fail.

### STEP / STP and IGES / IGS

Open CASCADE reads self-contained STEP text and IGES surfaces/solids with assembly
placements, smooth normals, and a neutral material. Tessellation tolerance is
0.1% of the model's largest dimension. CAD colors, textures, annotations,
external assemblies, and curve/point-only geometry are not rendered. Compressed
STEP and STEP XML are unsupported. Large assemblies may exceed the render limits.

### Alembic

Reads Ogawa archives at the earliest authored sample: polygon meshes,
transforms, visibility, indexed normals, and subdivision control cages with holes.
Repeated polygon corners are removed before triangulation. Subdivision refinement,
curves, points, NURBS patches, materials/textures, and HDF5 archives are unsupported.

### Rhino 3DM

Reads mesh objects and saved render meshes. Without a saved mesh, extrusions and
planar Brep faces/surfaces can be meshed in memory, including curved profiles,
holes, open profiles, and mitered ends. Local blocks, visibility, transforms,
object/layer colors, and vertex colors are supported. Source files are unchanged.

Uncached curved Breps/NURBS and SubD fail the preview. Linked blocks, textures,
per-instance inherited colors, curves, points, and annotations are not rendered.
Default black layer colors use the neutral material. Complex profile boundaries
may exceed tessellation limits. See [scene backend notes](SCENE-SOURCE.md).

### IFC

IFC2x3 uses Assimp. IFC4 uses the bundled IfcConvert helper for tessellated meshes,
swept solids, Breps, boolean openings, and surface materials. Spaces and separate
opening volumes are excluded. Textures, indexed face-color maps, annotations,
and BIM metadata are not represented; unsupported upstream geometry may be omitted.
IFC4.3, IFCZIP, and IFCXML are not enabled.

The IFC4 converter has a **4-second deadline** and **768 MiB memory limit**,
within the overall render deadline. No separate application, Python, or runtime
download is needed. Temporary files are cleaned on completion, failure, or menu
cancellation; externally killing a standalone CLI process can leave them behind.
See [converter sources and licenses](IFC-SOURCE.md).

### VOX

Reads MagicaVoxel 150/200 with palette colors, exposed voxel faces, groups,
instances, integer transforms, and hidden nodes/layers. Animated scenes use the
earliest authored frame; older PACK files show their first model. Material
effects, glass/refraction, lighting, and animation playback are not rendered.
Large or heavily instanced voxel scenes may exceed the geometry limits.

### LWO, LXO, LWS, and ASE

- **LWO:** LWOB/LWO2 meshes, layers, surface/vertex colors, UVs, and local diffuse
  textures. Subdivision shows the control mesh; procedural/node materials are omitted.
- **LXO:** LXOB polygon mesh layers, pivots, colors, normals, UVs, and supported
  diffuse textures. Scene-item transforms, instances, deformation, procedural
  geometry, and full shader graphs are not evaluated.
- **LWS:** UTF-8 LWSC 3–5 scenes referencing local LWOB/LWO2/LXOB objects.
  Supports parenting, pivots, and initial channel transforms. Objects are resolved
  beside the scene or up to two parent folders for Scenes/Objects layouts.
  Missing objects or invalid hierarchies fail. LWO3, LWSC 1/2, nested scenes,
  plugins, visibility/dissolve settings, and animation playback are unsupported.
- **ASE:** static mesh geometry, transforms, colors, UVs, and diffuse textures.
  Accepts plain text, UTF-8 BOM, and UTF-16 LE/BE BOM. Skeletal deformation,
  animation, and procedural materials are not rendered.

Keep local object and texture paths intact. The source applications are not required.

### SMD, MD2, MD3, and MD5MESH

- **SMD:** version 1 reference meshes; skeleton/animation-only files have no preview.
- **MD2:** version 8, first vertex-animation frame and first declared skin.
- **MD3:** version 15, first frame and optional `<model>_default.skin` mapping.
  Head/upper/lower files are not assembled; tags and shader scripts are ignored.
- **MD5MESH:** version 10, weighted bind-pose geometry and local diffuse textures.
  MD5ANIM and MD5CAMERA are not loaded.

Keep textures beside the model or in relative subfolders. Extensionless image
names try TGA, PNG, JPEG, DDS, BMP, then PCX; MD5 uses Assimp's `<name>_d.tga`
convention. PCX v5 supports 8-bit indexed or three-plane RGB, raw or RLE.
Game archives, VMT/VTF materials, shader effects, and normal/specular maps are
not resolved. Extract geometry and supported textures first.

### DXF

Reads ASCII 3DFACE and POLYLINE polyface meshes, with local BLOCK/INSERT
hierarchies, transforms, arrays, and layer/ByBlock/true-color materials.
Off/frozen layers, invisible entities, and paper-space geometry are omitted.

Binary DXF, XREFs, ACIS solids, REGION/BODY/SURFACE, modern MESH, SOLID, and HATCH
are unsupported. Curves/text are not tessellated; line-only drawings have no
thumbnail. Missing/cyclic references and incomplete sections fail. Textures,
transparency, plot styles, XCLIP, and dynamic blocks are not evaluated.
