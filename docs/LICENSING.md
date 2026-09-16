# Licensing notes

[Back to MeshThumbs](../README.md)

## Project code

[LICENSE](../LICENSE) contains the MIT terms for MeshThumbs and credits both
the original 3DThumbnails authors and MeshThumbs contributors. The upstream
[Cargo.toml](https://github.com/While402/3DThumbnails/blob/main/Cargo.toml)
and README declare MIT; the imported upstream tree did not include a standalone
license file. The upstream attribution is retained in the project license.

## Dependencies and distribution

[Third-party notices](THIRD-PARTY-NOTICES.txt) records 92 packages in the locked
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

The unmodified MPL-2.0 source for `option-ext` 0.2.0 is included as
[option-ext-0.2.0.crate](third-party/option-ext-0.2.0.crate). Its SHA-256 matches
`Cargo.lock`. This is a gzip-compressed tar archive containing the published
source and license. The MSI installs the same archive alongside `LICENSE` and
`THIRD-PARTY-NOTICES.txt`, so its source remains available offline.

When updating dependencies, compare the locked Windows dependency graph with
the notice inventory and retain the applicable license texts and source links.
When changing compilers, update the runtime notices as well. This inventory
corresponds to the Rust 1.98.1 / MinGW build of MeshThumbs 1.0.3.

## Preview assets

The project MIT license does not relicense models or their rendered images.
[Preview credits](images/ATTRIBUTION.md) identifies every model in the current
README image, its source, license, and the changes made for display. Earlier
preview images and previously published installers are not updated by this
documentation change.
