# Licensing notes

[Back to MeshThumbs](../README.md)

## Project code

[LICENSE](../LICENSE) contains the MIT terms for MeshThumbs and credits both
the original 3DThumbnails authors and MeshThumbs contributors. The upstream
[Cargo.toml](https://github.com/While402/3DThumbnails/blob/main/Cargo.toml)
and README declare MIT; the imported upstream tree did not include a standalone
license file. The upstream attribution is retained in the project license.

## Dependencies and distribution

[Third-party notices](THIRD-PARTY-NOTICES.txt) records 119 packages in the locked
Windows GNU dependency graph, including build dependencies and procedural
macros. It also contains notices for bundled native code and Rust/MinGW runtimes.
Optional native components and other-platform runtime notices are retained
conservatively; inclusion is not a claim that every component is linked.

MIT is selected where a dependency offers it as an alternative. Other selected
terms include BSD, MPL-2.0, and the additional Unicode license. The bundled GCC
runtime retains its GPL terms with the GCC Runtime Library Exception.

The `ufbx` Rust crate omits a standalone license file from its archive. Its
manifest declares `MIT OR PDDL-1.0`; the notices retain that declaration, its
package author, pinned source links, and the MIT notice for bundled ufbx 0.21.1.

USD parsing uses `openusd` 0.7.0 under MIT. Its crate archive omits the workspace
license file; the notices include the license from the exact upstream commit
recorded in the package's `.cargo_vcs_info.json`. New transitive dependency
notices include the Zlib terms for `zlib-rs` and Apache-2.0 terms for `zopfli`.

The unmodified MPL-2.0 source for `option-ext` 0.2.0 is included as
[option-ext-0.2.0.crate](third-party/option-ext-0.2.0.crate). Its SHA-256 matches
`Cargo.lock`. This is a gzip-compressed tar archive containing the published
source and license. The MSI installs the same archive alongside `LICENSE` and
`THIRD-PARTY-NOTICES.txt`, so its source remains available offline.

STEP and IGES use Open CASCADE Technology 7.9.3 under LGPL-2.1 with the Open CASCADE
exception. Its libraries remain separate, replaceable DLLs; MeshThumbs code and
the small C interface adapter remain MIT. The installer includes the library's
corresponding source and build system in `step/occt-source-7.9.3.tar.gz`, plus
[rebuild and replacement instructions](OCCT-SOURCE.md). Notices include the LGPL,
its exception, the bundled DELABELLA triangulator, and Flex/Bison parser notices.
The MinGW CAD build also ships its GCC, C++, and pthread runtime DLLs; their
terms and exceptions are retained in the runtime notices. No new Rust package
is added by this integration.

Alembic and 3DM use a separate scene DLL containing Alembic 1.8.8, Imath 3.1.12,
and a pinned openNURBS 8.x revision with its prefixed zlib. Their upstream
notices and the two openNURBS compatibility patches are documented in the
[scene backend sources](SCENE-SOURCE.md) and third-party notices. IFC2x3 uses
the existing Assimp dependency; no new Rust dependency is added.

When updating dependencies, compare the locked Windows dependency graph with
the notice inventory and retain the applicable license texts and source links.
When changing compilers, update the runtime notices as well. This inventory
corresponds to the Rust 1.98.1 / MinGW build of MeshThumbs 1.1.0.

The standard WiX 3.14.1 installer dialogs and artwork retain MS-RL. Their
unmodified sources, license, and this build's original installer authoring
accompany the MSI as two source archives. See [installer UI sources](WIX-UI-SOURCE.md).
The independent MeshThumbs application code remains MIT.

## Preview assets

The project MIT license does not relicense models or their rendered images.
[Preview credits](images/ATTRIBUTION.md) identifies every model in the current
README image, its source, license, and the changes made for display. Earlier
preview images and previously published installers are not updated by this
documentation change.

The 1080p preview uses ten CC0 examples, a CC BY 4.0 sofa representing the USD
family, a BSD-2-Clause 3MF sample, a VRM Public License 1.0 avatar with its
embedded usage settings, and original MIT-licensed circuit board and bracket
models for VRML and STEP, plus original MIT-licensed knot, gear, vase, and house
models for Alembic, IGES, 3DM, and IFC.
The original robot, island, satellite, turret, crate, drone, and mech examples
and their textures for PMX, VOX, LWO, SMD, MD2, MD3, and MD5MESH are also MIT.
The [examples](../examples/README.md) directory redistributes the corresponding
model files and required textures under those same licenses. It includes license
texts, credits, conversion notes, and the unmodified VRM sample's license settings.
The sofa credit includes its author, copyright holder,
source, license link, and rendering changes. Asset revisions are pinned in the
credits; their licenses remain separate from the project MIT license.
