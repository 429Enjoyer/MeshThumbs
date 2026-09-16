# Changelog

[Back to MeshThumbs](../README.md)

## 1.0.7

- Add STEP and STP previews with Open CASCADE tessellation of CAD solids,
  surfaces, and self-contained assemblies, without a separate CAD application.
- Load the CAD backend only for STEP files and include replaceable libraries,
  corresponding source, and license notices in the installer.
- Add a STEP example to the 1080p README preview and update asset credits.
- Include the README preview models, textures, and licenses in `examples/`.

## 1.0.6

- Add static VRML 2.0 / VRML97 previews for WRL and VRML files, including meshes,
  primitives, DEF/USE instances, transforms, colors, and local textures.
- Validate face indices and skip empty groups before importing VRML and X3D.
- Update the 1080p README preview and asset credits with a VRML example.

## 1.0.5

- Add USD, USDA, USDC, and USDZ previews using a native Rust reader.
- Support static meshes, basic primitives, scene transforms, local references,
  vertex colors, material subsets, and UsdPreviewSurface base-color textures.
- Read USDZ package assets without extracting them to disk or requiring Blender.
- Preserve original paths for USD and X3D sidecar textures and scene references.
- Refresh the README preview at 1920×1080 with a USD-family example and updated asset credits.

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
