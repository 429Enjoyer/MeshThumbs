# Preview credits

`meshthumbs-preview.png` contains thirty-two examples rendered by MeshThumbs
in a 1920×1080 layout. Models were rendered at 768×768 (BLEND at 1024×1024)
before resizing for the contact sheet. Ten use
CC0 models from the Khronos glTF Sample Assets collection; the USD-family,
3MF, VRM, downloaded native-format examples, and two procedural fallbacks are credited below.

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

## Native-format internet examples

- **suzanne.blend:** [Soft8Soft Three.js-Blender Template](https://github.com/Soft8Soft/threejs-blender-template/blob/b26e55701c0a1323ce9aa57d357cddf0de3f854f/suzanne.blend), © 2021 Soft8Soft LLC.
  [MIT](../../examples/LICENSES/Soft8Soft-MIT.txt), with [upstream terms](https://github.com/Soft8Soft/threejs-blender-template/blob/b26e55701c0a1323ce9aa57d357cddf0de3f854f/LICENSE).
  Model bytes are unchanged. The tile shows evaluated geometry exported with Blender 4.0,
  rendered, framed, and resized by MeshThumbs. It demonstrates PNG export, not the
  embedded image used by Explorer. Scene lighting/HDRI and full shader effects are omitted;
  the optional HDRI is not redistributed. No endorsement is implied.

- **Basin.ifc:** [basin-tessellation.ifc](https://github.com/buildingSMART/Sample-Test-Files/blob/80d976a9b193a26a8e928c3e79bff67af1de68a8/IFC%204.0.2.1%20(IFC%204%20ADD2%20TC1)/ISO%20Spec%20-%20ReferenceView_V1.2/basin-tessellation.ifc), © buildingSMART
  International Ltd.; original file header credits "Jon" and the Geometry Gym
  exporter. [CC BY 4.0](../../examples/LICENSES/CC-BY-4.0.txt), with the
  [upstream license](https://github.com/buildingSMART/Sample-Test-Files/blob/80d976a9b193a26a8e928c3e79bff67af1de68a8/LICENSE). Model bytes are unchanged; renamed only.
  Converted to geometry with IfcConvert, rendered with MeshThumbs, framed and
  resized. Its rendered adaptation remains CC BY 4.0. No endorsement is implied.


- **Classroom.abc:** [Classroom](https://github.com/ezequielmastrasso/gaffer-examples/blob/eab0243e478e6a573f44d1289643c6e40cf79ab0/assets/classroom/abc/classroom.abc), original model by Christophe Seux,
  Alembic export distributed by Ezequiel Mastrasso / Gaffer Examples.
  [CC0](../../examples/LICENSES/CC0-1.0.txt); [upstream credit](https://github.com/ezequielmastrasso/gaffer-examples/blob/eab0243e478e6a573f44d1289643c6e40cf79ab0/README.md#licences).
  Native Ogawa file renamed only. Rendered without materials/textures, framed,
  and resized; the complete scene includes its ceiling and exterior walls.
  The source model geometry is unchanged.


Sources and byte checksums are recorded in [sources.json](../../examples/sources.json).
The files were rendered at 768×768 and resized for this 1920×1080 collection.

- **ThreeCubesGreen.ASE:** [Assimp contributors (upstream test collection)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/ASE/ThreeCubesGreen.ASE). [BSD-3-Clause](../../examples/LICENSES/Assimp-BSD-3-Clause.txt). Unchanged file. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/LICENSE).
- **CrazyEngine.lxo:** [Assimp contributors (upstream test collection)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/LWO/LXOB_Modo/CrazyEngine.lxo). [BSD-3-Clause](../../examples/LICENSES/Assimp-BSD-3-Clause.txt). Unchanged file. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/LICENSE).
- **boxuv.lwo:** [Assimp contributors (upstream test collection)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/LWO/LWO2/boxuv.lwo). [BSD-3-Clause](../../examples/LICENSES/Assimp-BSD-3-Clause.txt). Unchanged file. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/LICENSE).
- **move_x.lws:** [Assimp contributors (upstream test collection)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/LWS/move_x.lws). [BSD-3-Clause](../../examples/LICENSES/Assimp-BSD-3-Clause.txt). Unchanged file. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/LICENSE).
- **holy_grailref.smd:** [Assimp contributors (upstream test collection)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/SMD/holy_grailref.smd). [BSD-3-Clause](../../examples/LICENSES/Assimp-BSD-3-Clause.txt). Unchanged file. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/LICENSE).
- **wuson.dxf:** [Assimp contributors (upstream test collection)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/DXF/wuson.dxf). [BSD-3-Clause](../../examples/LICENSES/Assimp-BSD-3-Clause.txt). Unchanged file. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/LICENSE).
- **faerie.md2:** [Irrlicht media; upstream notice credits Nikolaus Gebhardt (2002–2007)](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/MD2/faerie.md2). [Zlib](../../examples/LICENSES/Irrlicht-faerie.txt). Added one MD2 skin-name record referencing the supplied faerie2.bmp; geometry, UVs, and animation samples are unchanged. MeshThumbs shows frame zero. [Upstream attribution/terms](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/MD2/faerie-source.txt).
- **T-Rex.vox:** [© 2026 ephtracy](https://github.com/ephtracy/voxel-model/blob/3bff8feaa7d94a6237f2c07dc4ca3c3d268ef06c/vox/anim/T-Rex.vox). [MIT](../../examples/LICENSES/ephtracy-MIT.txt). Unchanged file. [Upstream attribution/terms](https://github.com/ephtracy/voxel-model/blob/3bff8feaa7d94a6237f2c07dc4ca3c3d268ef06c/LICENSE).
- **hello_mesh.3dm:** [© 1993–2018 Robert McNeel & Associates](https://github.com/mcneel/rhino-developer-samples/blob/786cf13c4a1ea50444fbf6471b1a8a1507b3114f/rhino3dm/js/SampleViewer/01_basic/hello_mesh.3dm). [McNeel permissive license](../../examples/LICENSES/McNeel.txt). Unchanged file. [Upstream attribution/terms](https://github.com/mcneel/rhino-developer-samples/blob/786cf13c4a1ea50444fbf6471b1a8a1507b3114f/License.md).
- **Duplex_A_20110907.ifc:** [BSI (2020) "Duplex Apartment Test Files," buildingSMART International](https://github.com/buildingsmart-community/Community-Sample-Test-Files/blob/7ddf57a201f88a0c213d5322b02ed15e94a60a40/IFC%202.3.0.1%20(IFC%202x3)/Duplex%20Apartment/Duplex_A_20110907.ifc). [CC BY 4.0](../../examples/LICENSES/CC-BY-4.0.txt). Unchanged file. [Upstream attribution/terms](https://github.com/buildingsmart-community/Community-Sample-Test-Files/blob/7ddf57a201f88a0c213d5322b02ed15e94a60a40/IFC%202.3.0.1%20(IFC%202x3)/Duplex%20Apartment/README.md).
- **copter.md3:** [Gobusto (2011), Cartoon Helicopter](https://opengameart.org/content/cartoon-helicopter). [CC BY-SA 3.0](../../examples/LICENSES/CC-BY-SA-3.0.txt). Extracted unchanged from fishcopter.zip; preview shows frame zero. [Upstream attribution/terms](https://opengameart.org/content/cartoon-helicopter).
- **Airplane.pmx:** [pennennennennennenem, MikuMikuDayo airplane](https://github.com/pennennennennennenem/MikuMikuDayo/releases/tag/MikuMikuDayo130). [CC0](../../examples/LICENSES/CC0-1.0.txt). Extracted sample/ひこうき.pmx from MikuMikuDayo130.zip and renamed Airplane.pmx; model bytes unchanged. [Upstream attribution/terms](https://github.com/pennennennennennenem/MikuMikuDayo/blob/aa7650ad3cb140c128ae08f095235a6870560461/README.md#faq).
- **USB_Micro-B.wrl:** [Joan Obijuan; adapted for KiCad by Frank Shackmeister (Shack)](https://github.com/KiCad/kicad-packages3D/blob/b8b3cfdfad88ba66f21002b3de51dc6f7d55ba5a/Connector_USB.3dshapes/USB_Micro-B_Molex_47346-0001.wrl). [CC BY-SA 4.0 with KiCad library exception](../../examples/LICENSES/CC-BY-SA-4.0.txt). Renamed only. STEP renders with a neutral material; WRL preserves authored colors. [Upstream attribution/terms](https://github.com/KiCad/kicad-packages3D/blob/b8b3cfdfad88ba66f21002b3de51dc6f7d55ba5a/Connector_USB.3dshapes/CREDITS.md).
- **USB_Micro-B.step:** [Joan Obijuan; adapted for KiCad by Frank Shackmeister (Shack)](https://github.com/KiCad/kicad-packages3D/blob/b8b3cfdfad88ba66f21002b3de51dc6f7d55ba5a/Connector_USB.3dshapes/USB_Micro-B_Molex_47346-0001.step). [CC BY-SA 4.0 with KiCad library exception](../../examples/LICENSES/CC-BY-SA-4.0.txt). Renamed only. STEP renders with a neutral material; WRL preserves authored colors. [Upstream attribution/terms](https://github.com/KiCad/kicad-packages3D/blob/b8b3cfdfad88ba66f21002b3de51dc6f7d55ba5a/Connector_USB.3dshapes/CREDITS.md).

`boxuv.png`, `holygrail.tga`, and `simple_cube.lwo` come from the same Assimp
revision under its BSD terms. `faerie2.bmp` accompanies the Irrlicht sample
under the included Irrlicht notice; `copter.pcx` is part of Gobusto's CC BY-SA 3.0
download. The PMX release archive was only unpacked to obtain the author's CC0
airplane; its application and other bundled assets are not redistributed.

The helicopter's rendered adaptation retains CC BY-SA 3.0, the two KiCad
tiles retain CC BY-SA 4.0, and the Duplex render retains CC BY 4.0. Keep their
credits, source links, license texts, and this change notice when sharing.
The contact sheet is a collection; individual asset terms remain separate.
The project MIT license does not replace those terms. No endorsement is implied.
KiCad's [library exception](../../examples/LICENSES/KiCad-exception.md) and
[original credits](../../examples/LICENSES/KiCad-CREDITS.md) are included.

## Retained examples

- **IGES:** `Gear.igs` / `Gear.iges`, original cylindrical/box CAD geometry with a central cut, © 2026 MeshThumbs contributors, [MIT](../../LICENSE). Exported through Open CASCADE and rendered with a neutral material.
- **USD / USDA / USDC / USDZ:** [GlamVelvetSofa](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/90d7ede14c7e280af263824604b427a1ca02cb66/Models/GlamVelvetSofa/README.md), Eric Chadwick, © 2021 Wayfair, LLC. [Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/). Converted from glTF into all four USD encodings with Blender 4.0.2; the fabric base color uses the source's pale-pink variant. MeshThumbs renders base color and geometry; the original sheen, normal-map, and specular effects are omitted. All four exports produced identical thumbnail pixels; `Sofa.usdz` is shown once to represent the family. This rendered adaptation retains CC BY 4.0; the project MIT license does not replace it. No endorsement is implied.
- **3MF:** [dodeca_chain_loop_color.3mf](https://github.com/3MFConsortium/3mf-samples/blob/665e20dc4d7777fd4c9702bca86a2d4028440337/examples/material/dodeca_chain_loop_color.3mf), © 2018 3MF Consortium. BSD 2-Clause license, reproduced below.
- **VRM:** [VRM1_Constraint_Twist_Sample](https://github.com/vrm-c/vrm-specification/tree/821c11b250d8c70d5804ee13431e42bee56ea9c0/samples/VRM1_Constraint_Twist_Sample), © 2022 pixiv Inc. [VRM Public License 1.0](https://vrm.dev/licenses/1.0/), with the usage permissions embedded in sample version 1.0.1. Its metadata allows commercial use by corporations, use by everyone, redistribution, and modification/redistribution; credit notation is optional. Antisocial or hate usage is prohibited. The neutral model preview shown here follows those settings; the model itself is unchanged.
- **MD5MESH:** `Mech.md5mesh` and `assets/Mech_d.tga`, original procedural weighted mesh and diffuse texture, © 2026 MeshThumbs contributors, [MIT](../../LICENSE). Rendered in its bind pose.

[Search results and remaining candidate limitations](../../examples/SOURCES.md).

Model files, textures, and offline licenses are in [examples](../../examples/README.md). They are not included in the installer.

## Additional example (not shown)

`hello_mesh.3dm` is the only 3DM tile in the README image. The following model
remains in `examples/` to demonstrate support without a cached mesh.

- **PlanarSurface.3dm:** [McNeel `03.3dm`](https://github.com/mcneel/rhino-developer-samples/blob/786cf13c4a1ea50444fbf6471b1a8a1507b3114f/rhino.inside/dotnet-netcore/BatchOperation/files/03.3dm), © 1993–2018 Robert McNeel
  & Associates, [permissive McNeel license](../../examples/LICENSES/McNeel.txt).
  File bytes are unchanged; renamed only. Its uncached planar Brep is meshed
  in memory by MeshThumbs, then rendered, framed and resized. No endorsement
  is implied. [Upstream terms](https://github.com/mcneel/rhino-developer-samples/blob/786cf13c4a1ea50444fbf6471b1a8a1507b3114f/License.md).

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
