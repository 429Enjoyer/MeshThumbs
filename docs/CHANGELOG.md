# Changelog

[Back to MeshThumbs](../README.md)

## 1.0.8

- Add Alembic Ogawa previews using the initial authored time, scene transforms,
  visibility, polygon meshes, and subdivision control meshes.
- Add IGES surface and solid previews through the existing Open CASCADE backend.
- Add Rhino 3DM mesh and saved render-mesh previews, including local block instances.
- Add IFC2x3 building geometry previews through Assimp.
- Stop restarting Explorer from installer cache refresh actions; let Windows
  Installer manage in-use files and application recovery through Restart Manager.
- Skip the old-package refresh during upgrades and avoid launching duplicate
  Explorer processes during manual cache resets.
- Allow the rebuilt 1.0.8 installer to replace an earlier 1.0.8 installation.

## 1.0.7

- Add STEP and STP previews with Open CASCADE tessellation of CAD solids,
  surfaces, and self-contained assemblies, without a separate CAD application.
- Load the CAD backend only for STEP files and include replaceable libraries,
  corresponding source, and license notices in the installer.

## 1.0.6

- Add static VRML 2.0 / VRML97 previews for WRL and VRML files, including meshes,
  primitives, DEF/USE instances, transforms, colors, and local textures.
- Validate face indices and skip empty groups before importing VRML and X3D.

## 1.0.5

- Add USD, USDA, USDC, and USDZ previews using a native Rust reader.
- Support static meshes, basic primitives, scene transforms, local references,
  vertex colors, material subsets, and UsdPreviewSurface base-color textures.
- Read USDZ package assets without extracting them to disk or requiring Blender.
- Preserve original paths for USD and X3D sidecar textures and scene references.

## 1.0.4

- Add static XML X3D previews with DEF/USE instances, materials, and local textures.
- Add ASCII OFF, COFF, NOFF, and CNOFF meshes with polygon triangulation and vertex colors.

## 1.0.3

- Add BLEND embedded previews without requiring Blender.
- Support uncompressed, gzip, and Zstandard files, including Blender 5.0+ headers.

## 1.0.2

- Add 3MF meshes, component assemblies, colors, and textures.
- Add VRM 0.x/1.0 avatar previews.
- Fix fully transparent texels obscuring geometry behind them.

## 1.0.1

- Fix dotted PLY previews by rendering mesh surfaces in full.
- Reject models above five million triangles instead of removing faces.

## 1.0.0

- Initial release with OBJ, FBX, GLB, glTF, STL, DAE, PLY, and 3DS support.
