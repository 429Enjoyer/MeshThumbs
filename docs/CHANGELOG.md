# Changelog

[Back to MeshThumbs](../README.md)

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
