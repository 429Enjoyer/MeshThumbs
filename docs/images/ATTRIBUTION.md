# Preview credits

`meshthumbs-preview.png` contains nineteen examples rendered by MeshThumbs 1.0.8
in a 1920×1080 layout (seven, seven, and five centered examples per row). Each
model was rendered at 768×768 before resizing for the contact sheet. Ten use
CC0 models from the Khronos glTF Sample Assets collection; the USD-family,
3MF, VRM, and original VRML/STEP/Alembic/IGES/3DM/IFC examples are credited separately below.

## CC0 examples

These sources are pinned to Khronos revision
`90d7ede14c7e280af263824604b427a1ca02cb66`. Each linked model page identifies its
authors and [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/)
dedication. They retain that status; the project MIT license does not replace it.

| Preview | Source model | Credit |
| --- | --- | --- |
| 3DS | [Suzanne](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/Suzanne/README.md) | Norbert Nopper / UX3D |
| DAE | [Lantern](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/Lantern/README.md) | Microsoft (sbtron); Frank Galligan |
| FBX | [ToyCar](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/ToyCar/README.md) | Guido Odendahl; Eric Chadwick |
| GLB | [BoomBox](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/BoomBox/README.md) | Microsoft |
| glTF | [FlightHelmet](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/FlightHelmet/README.md) | Gary Hsu |
| OBJ | [Avocado](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/Avocado/README.md) | Microsoft |
| PLY | [BoxVertexColors](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/BoxVertexColors/README.md) | Marco Hutter |
| STL | [WaterBottle](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/WaterBottle/README.md) | Microsoft |
| X3D | [BarramundiFish](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/BarramundiFish/README.md) | Microsoft |
| OFF | [SheenChair](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/SheenChair/README.md) | Eric Chadwick / Wayfair, LLC |

Changes: models were converted to the displayed file formats where necessary,
triangulated, framed, rendered, and resized. The ToyCar preview omits the fabric
and glass props. Suzanne uses a uniform diffuse color; STL omits materials.
Avocado's base-color texture was extracted and assigned to its OBJ material.
BarramundiFish was converted to X3D with its base-color texture assigned to an
ImageTexture node. SheenChair was converted to NOFF with geometry and normals;
its materials and sheen effects are omitted.

## Other examples

- **Alembic:** `Knot.abc`, an original procedural trefoil tube, © 2026 MeshThumbs contributors, [MIT](../../LICENSE). Exported from Blender 4.0.2 as Ogawa with rotation samples for frames 1–12. The thumbnail uses the initial authored time.
- **IGES:** `Gear.igs` / `Gear.iges`, original cylindrical/box CAD geometry with a central cut, © 2026 MeshThumbs contributors, [MIT](../../LICENSE). Exported through Open CASCADE and rendered with a neutral material.
- **3DM:** `Vase.3dm`, an original procedural quad mesh with vertex normals and a blue display color, © 2026 MeshThumbs contributors, [MIT](../../LICENSE). Written as Rhino 8 3DM with openNURBS; no Rhino application or external model data was used.
- **IFC:** `House.ifc`, an original IFC2x3 scene of colored extruded building profiles, © 2026 MeshThumbs contributors, [MIT](../../LICENSE). No third-party geometry or textures are used.
- **STEP / STP:** `Bracket.step`, an original procedural CAD model created for MeshThumbs, © 2026 MeshThumbs contributors, under the project's [MIT license](../../LICENSE). Constructed from boxes and cylinders with boolean cuts, using no third-party model data or textures. Exported to STEP, tessellated with Open CASCADE, and rendered with MeshThumbs 1.0.8 in a neutral material before resizing. Both `.step` and `.stp` were checked. Library licenses do not change the license of this original model or its rendered image.
- **WRL / VRML:** `CircuitBoard.wrl`, an original procedural model created for MeshThumbs, © 2026 MeshThumbs contributors, under the project's [MIT license](../../LICENSE). The board, traces, chips, connectors, and mounting pads use original geometric constructions, with no third-party models or textures. Rendered from VRML97 with MeshThumbs 1.0.8 and resized; the same scene was checked using both `.wrl` and `.vrml` extensions. All examples were rendered again at 768×768 for this layout.
- **USD / USDA / USDC / USDZ:** [GlamVelvetSofa](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/GlamVelvetSofa/README.md), Eric Chadwick, © 2021 Wayfair, LLC. [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/). Converted from glTF into all four USD encodings with Blender 4.0.2; the fabric base color uses the source's pale-pink variant. MeshThumbs renders base color and geometry; the original sheen, normal-map, and specular effects are omitted. All four exports produced identical thumbnail pixels; `Sofa.usdz` is shown once to represent the family. This rendered adaptation retains CC BY 4.0; the project MIT license does not replace it. No endorsement is implied.
- **3MF:** [dodeca_chain_loop_color.3mf](https://github.com/3MFConsortium/3mf-samples/blob/665e20dc4d7777fd4c9702bca86a2d4028440337/examples/material/dodeca_chain_loop_color.3mf), © 2018 3MF Consortium. BSD 2-Clause license, reproduced below.
- **VRM:** [VRM1_Constraint_Twist_Sample](https://github.com/vrm-c/vrm-specification/tree/821c11b250d8c70d5804ee13431e42bee56ea9c0/samples/VRM1_Constraint_Twist_Sample), © 2022 pixiv Inc. [VRM Public License 1.0](https://vrm.dev/licenses/1.0/), with the usage permissions embedded in sample version 1.0.1. Its metadata allows commercial use by corporations, use by everyone, redistribution, and modification/redistribution; credit notation is optional. Antisocial or hate usage is prohibited. The neutral model preview shown here follows those settings; the model itself is unchanged.

The 3MF and VRM examples were also rendered again at 768×768 and resized for
the contact sheet. All asset source links are pinned to the revisions checked
for this image. The corresponding models and their required textures are now
included in [examples](../../examples/README.md), with offline license texts
and the VRM sample's embedded license settings. The installer does not include
these example assets. Source links above identify the original assets and terms.

## 3MF sample license

BSD 2-Clause License

Copyright (c) 2018, 3MF Consortium
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this
  list of conditions and the following disclaimer.

* Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
