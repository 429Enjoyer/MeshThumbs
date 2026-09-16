# Internet example search

Checked 16 September 2026. Prefer downloadable, redistributable models in the
target format before creating a procedural example. A download being free does
not by itself establish redistribution rights.

The [example inventory](README.md) credits the selected files. The
[download manifest](sources.json) records source URLs, revisions, original and
distributed SHA-256 hashes, license evidence, alterations, and archive hashes.
The earlier Khronos, 3MF, and VRM assets retain their existing pinned credits.

## Selected and rendered

| Format | Internet example | Source / terms |
| --- | --- | --- |
| ABC | `Classroom.abc` | [Christophe Seux / Gaffer Examples](https://github.com/ezequielmastrasso/gaffer-examples/blob/eab0243e478e6a573f44d1289643c6e40cf79ab0/README.md#licences), CC0 |
| WRL / VRML | USB Micro-B connector | [KiCad](https://github.com/KiCad/kicad-packages3D/tree/b8b3cfdfad88ba66f21002b3de51dc6f7d55ba5a/Connector_USB.3dshapes), CC BY-SA 4.0 with library exception |
| STEP / STP | Same connector, native CAD export | Same KiCad source and terms |
| 3DM | `hello_mesh.3dm` | [McNeel samples](https://github.com/mcneel/rhino-developer-samples/tree/786cf13c4a1ea50444fbf6471b1a8a1507b3114f/rhino3dm/js/SampleViewer/01_basic), permissive McNeel license |
| IFC | Duplex Apartment architectural model | [buildingSMART community](https://github.com/buildingsmart-community/Community-Sample-Test-Files/tree/7ddf57a201f88a0c213d5322b02ed15e94a60a40/IFC%202.3.0.1%20(IFC%202x3)/Duplex%20Apartment), CC BY 4.0 |
| PMX | MikuMikuDayo airplane | [Author's CC0 statement](https://github.com/pennennennennennenem/MikuMikuDayo/blob/aa7650ad3cb140c128ae08f095235a6870560461/README.md#faq); model extracted from release 1.30 |
| VOX | `T-Rex.vox` | [ephtracy](https://github.com/ephtracy/voxel-model/tree/3bff8feaa7d94a6237f2c07dc4ca3c3d268ef06c), MIT |
| LWO | `boxuv.lwo` and texture | [Assimp test collection](https://github.com/assimp/assimp/tree/392a658f9c271be965271f45e7521a1b80ea4392/test/models), BSD-3-Clause |
| SMD | `holy_grailref.smd` and texture | Same Assimp collection and terms |
| MD2 | `faerie.md2` and skin | Assimp's Irrlicht media sample, [asset-specific Zlib notice](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/MD2/faerie-source.txt) |
| MD3 | Cartoon Helicopter | [Gobusto / OpenGameArt](https://opengameart.org/content/cartoon-helicopter), CC BY-SA 3.0 |
| ASE | `ThreeCubesGreen.ASE` | Assimp test collection, BSD-3-Clause |
| LXO | `CrazyEngine.lxo` | Assimp test collection, BSD-3-Clause |
| LWS | `move_x.lws` with `simple_cube.lwo` | Assimp test collection, BSD-3-Clause |
| DXF | `wuson.dxf` | Assimp test collection, BSD-3-Clause |

These replace fifteen procedural preview tiles. All selected models were
rendered from the distributed files with MeshThumbs 1.1.3 at 768×768, then used
in the 1920×1080 README sheet. The source files are unchanged except the MD2
skin-name record, documented in the inventory. Renames and aliases do not change
model bytes. Thumbnail rendering shows the format's supported static pose and
materials, not a full rendering of the source application's effects.

## Resolved ABC failure

The 19 MB Classroom export failed in 1.1.2 with `polygon cannot be triangulated`.
It contains 96 zero-length edges represented by consecutive identical corners.
Version 1.1.3 removes redundant corners before triangulation while preserving
original corner attributes. The original file now renders and replaces `Knot.abc`.
No geometry was edited to make the download pass.

## Candidates not selected

| Format | Candidate examined | Result |
| --- | --- | --- |
| IGES | [John Burkardt's three IGES samples](https://people.math.sc.edu/Burkardt/data/iges/iges.html), LGPL | All three download but contain no faces renderable by the current backend. |
| IGES | [CADFormats CC0 cylinder](https://cadformats.com/workbench/guides/step-vs-iges) | The documented download returned HTTP 403. Keep `Gear.igs` / `.iges` pending a suitable accessible source. |
| MD5MESH | [Assimp SimpleCube provenance](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models/MD5/SimpleCube.source.txt) | Renders, but its provenance points to an old third-party forum without a specific permission statement. Not adopted on the strength of the repository license alone. |
| MD5MESH | [Assimp Bob notice](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models-nonbsd/MD5/bob.source.txt) | Commercial use requires separate written permission. Keep the original MIT `Mech.md5mesh` for now. |
| MD5MESH | [Assimp BoarMan notice](https://github.com/assimp/assimp/blob/392a658f9c271be965271f45e7521a1b80ea4392/test/models-nonbsd/MD5/BoarMan.source.txt) | Says Creative Commons attribution/share-alike but omits a license version/link; not adopted without clearer terms. |

These are results of this search, not a claim that online assets do not exist
for the two remaining formats. No new procedural model was generated.

## Rebuild the preview

With the built renderer and its native backend DLLs in `target/release/`, run:

```powershell
python scripts/render-example-previews.py
```

Requires Python 3.10+ and Pillow. It renders all example model files (including
aliases and the LWS dependency) before assembling the thirty selected tiles.
Intermediate PNGs go to `target/example-previews/`.
