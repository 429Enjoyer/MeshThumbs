# Licensing notes

[Back to MeshThumbs](../README.md)

## Project code

[LICENSE](../LICENSE) contains the MIT terms and credits for MeshThumbs and
3DThumbnails. The upstream [manifest](https://github.com/While402/3DThumbnails/blob/main/Cargo.toml)
and README declare MIT; the imported tree had no standalone license file.
The PNG exporter, context-menu handler, and native adapters are original MIT code.
The Blender conversion script is also original MIT code. It invokes the user's
installed Blender; no Blender executable or add-on source is redistributed.

## Dependencies and redistribution

[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt) records the locked Windows GNU
dependency inventory, selected license texts, source locations, and runtime
notices. It includes build dependencies and conservative cross-platform notices;
not every listed component is linked into the Windows binaries.

MIT is selected when offered as an alternative. Other terms include BSD,
MPL-2.0, Unicode, Zlib, Apache-2.0, and GCC's GPL with Runtime Library Exception.
Third-party code retains those terms.

Keep these sources and notices with redistributed installers:

| Component | Included sources / instructions |
| --- | --- |
| option-ext 0.2.0 (MPL-2.0) | Exact published [crate archive](third-party/option-ext-0.2.0.crate), matching `Cargo.lock`, installed beside the notices. |
| Open CASCADE 7.9.3 (LGPL-2.1 with exception) | Replaceable CAD/scene DLLs, `step/occt-source-7.9.3.tar.gz`, and [rebuild instructions](OCCT-SOURCE.md). |
| Alembic, Imath, openNURBS | Pinned sources, upstream notices, and compatibility patches described in [SCENE-SOURCE.md](SCENE-SOURCE.md). |
| IFC4 / IfcConvert 0.8.5 | Source archive, patches, build recipe, and dependency information in [IFC-SOURCE.md](IFC-SOURCE.md); full terms in [IFC-NOTICES.txt](IFC-NOTICES.txt). Include the linked Boost source for offline redistribution. |
| WiX 3.14.1 UI (MS-RL) | Upstream source/artwork and MeshThumbs installer authoring archives; see [WIX-UI-SOURCE.md](WIX-UI-SOURCE.md). |

The `ufbx` and `openusd` crates omit standalone license files from their archives.
Their notices retain pinned upstream license evidence: ufbx's declared terms and
bundled C-library MIT notice, and openusd's MIT license from its recorded commit.

The MinGW release bundles GCC/C++/pthread runtime DLLs, with their notices and
exceptions. The PNG exporter reuses existing dependencies and Windows system UI.
When changing dependencies or compilers, recheck the inventory and source bundles.
The current runtime inventory is based on Rust 1.98.1 and MinGW.

## Preview assets

The MIT license does not relicense models or rendered images. [Preview credits](images/ATTRIBUTION.md)
identify the source, author, license, and display changes for each README tile.
Preserve the individual credits and terms, including share-alike licenses,
the KiCad exception, and the VRM sample's embedded permissions.

[Examples](../examples/README.md) contains model files, textures, offline license
texts, and [source checksums](../examples/sources.json). Two original MIT fallbacks
remain for IGES and MD5MESH; the [search log](../examples/SOURCES.md) explains why.
Example assets are not installed by the MSI. These credits describe the current
preview; they do not change previously published images or installers.
