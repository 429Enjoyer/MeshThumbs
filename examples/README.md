# Example models

These are the fifteen models shown in the [README preview](../docs/images/meshthumbs-preview.png).
There are **20 model files**, including the USD encodings and VRML/STEP aliases.
All models are in this directory so they can be viewed together in Explorer.
Keep `assets/`, `textures/`, and `Avocado.mtl` beside them; those files supply
textures and materials. FBX, GLB, VRM, 3MF, and USDZ include their required assets.

| Model | Files | Credit | License |
| --- | --- | --- | --- |
| [Suzanne](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/Suzanne/README.md) | [Suzanne.3ds](Suzanne.3ds) | Norbert Nopper / UX3D | [CC0](LICENSES/CC0-1.0.txt) |
| [Lantern](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/Lantern/README.md) | [Lantern.dae](Lantern.dae) | Microsoft (sbtron); Frank Galligan | [CC0](LICENSES/CC0-1.0.txt) |
| [ToyCar](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/ToyCar/README.md) | [ToyCar.fbx](ToyCar.fbx) | Guido Odendahl; Eric Chadwick | [CC0](LICENSES/CC0-1.0.txt) |
| [BoomBox](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/BoomBox/README.md) | [BoomBox.glb](BoomBox.glb) | Microsoft | [CC0](LICENSES/CC0-1.0.txt) |
| [ColorChain](https://github.com/3MFConsortium/3mf-samples/blob/665e20dc4d7777fd4c9702bca86a2d4028440337/examples/material/dodeca_chain_loop_color.3mf) | [ColorChain.3mf](ColorChain.3mf) | © 2018 3MF Consortium | [BSD 2-Clause](LICENSES/3MF-BSD-2-Clause.txt) |
| [FlightHelmet](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/FlightHelmet/README.md) | [FlightHelmet.gltf](FlightHelmet.gltf) | Gary Hsu | [CC0](LICENSES/CC0-1.0.txt) |
| [Avocado](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/Avocado/README.md) | [Avocado.obj](Avocado.obj) | Microsoft | [CC0](LICENSES/CC0-1.0.txt) |
| [BoxVertexColors](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/BoxVertexColors/README.md) | [BoxVertexColors.ply](BoxVertexColors.ply) | Marco Hutter | [CC0](LICENSES/CC0-1.0.txt) |
| [WaterBottle](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/WaterBottle/README.md) | [WaterBottle.stl](WaterBottle.stl) | Microsoft | [CC0](LICENSES/CC0-1.0.txt) |
| [Avatar](https://github.com/vrm-c/vrm-specification/tree/821c11b250d8c70d5804ee13431e42bee56ea9c0/samples/VRM1_Constraint_Twist_Sample) | [Avatar.vrm](Avatar.vrm) | © 2022 pixiv Inc. | [VRM Public License 1.0](LICENSES/VRM-Public-License-1.0.html), with [sample settings](LICENSES/Avatar-license-settings.json) |
| [BarramundiFish](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/BarramundiFish/README.md) | [BarramundiFish.x3d](BarramundiFish.x3d) | Microsoft | [CC0](LICENSES/CC0-1.0.txt) |
| [SheenChair](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/SheenChair/README.md) | [SheenChair.off](SheenChair.off) | Eric Chadwick / Wayfair, LLC | [CC0](LICENSES/CC0-1.0.txt) |
| [GlamVelvetSofa](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/GlamVelvetSofa/README.md) | [Sofa.usd](Sofa.usd), [Sofa.usda](Sofa.usda), [Sofa.usdc](Sofa.usdc), [Sofa.usdz](Sofa.usdz) | Eric Chadwick; © 2021 Wayfair, LLC | [CC BY 4.0](LICENSES/CC-BY-4.0.txt) |
| CircuitBoard | [CircuitBoard.wrl](CircuitBoard.wrl), [CircuitBoard.vrml](CircuitBoard.vrml) | © 2026 MeshThumbs contributors | [MIT](LICENSES/MIT.txt) |
| Bracket | [Bracket.step](Bracket.step), [Bracket.stp](Bracket.stp) | © 2026 MeshThumbs contributors | [MIT](LICENSES/MIT.txt) |

## Changes and licensing

The ten CC0 models and the sofa use the pinned Khronos revision linked above.
Models were converted with Blender 4.0.2 or small format writers where needed,
and triangulated for display. Suzanne has a uniform diffuse color. ToyCar omits
its fabric and glass props. WaterBottle's STL and SheenChair's NOFF omit materials;
NOFF retains normals. Avocado and BarramundiFish use their extracted base-color
textures. FlightHelmet retains its geometry and textures with relative asset
paths moved into `assets/FlightHelmet/`.

The sofa was converted from glTF to the four USD encodings. Its fabric base color
uses the original pale-pink variant; the thumbnail renderer omits the original
sheen, normal-map, and specular effects. The converted sofa and its textures
remain **CC BY 4.0**. Keep the author/copyright credit, source and license links,
and this change notice when sharing them. No endorsement is implied.

ColorChain is the unmodified 3MF sample, renamed for display. Avatar is the
unmodified `VRM1_Constraint_Twist_Sample` v1.0.1, also renamed. Its embedded
settings allow redistribution, commercial use by corporations, and modification
and redistribution; credit notation is optional. Antisocial or hate usage is
prohibited. Read its license and settings before reuse; the license includes
its disclaimer of warranties. The original settings remain in `Avatar.vrm`.

CircuitBoard and Bracket are original procedural models with no third-party
model data or textures. CircuitBoard uses boxes, cylinders, and polygonal pads.
Bracket combines CAD boxes and cylinders with boolean cuts and was exported
as STEP using Open CASCADE. `.vrml` and `.stp` are identical copies of their
`.wrl` and `.step` counterparts. The four USD encodings render identically.

The project's MIT license does **not** relicense third-party models or textures.
`assets/FlightHelmet/` belongs to FlightHelmet; the Avocado, BarramundiFish, and
Lantern files in `assets/` retain their corresponding CC0 terms. `textures/`
contains the sofa's CC BY 4.0 images, also embedded in `Sofa.usdz`.
Full license texts are included in `LICENSES/`; source and license links also
appear in the [preview credits](../docs/images/ATTRIBUTION.md).

## Render a thumbnail

With MeshThumbs built, run from the repository root:

```powershell
.\target\release\thumbgen.exe .\examples\Bracket.step .\preview.png 768
```

STEP requires the bundled `step` backend beside `thumbgen.exe`. See the
[build instructions](../docs/USAGE.md). These examples are repository assets
and are not installed by the MSI.
