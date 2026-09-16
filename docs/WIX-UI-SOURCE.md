# Installer UI sources

[Back to MeshThumbs](../README.md)

The installer uses the unmodified **WiX Toolset 3.14.1 WixUI_Minimal** dialog
library. Only `FilesInUse` is replaced by the original MeshThumbs dialog in
`wix/InstallerUI.wxs`, adding the explicit Restart Explorer button. The welcome
page's license text is generated from the project `LICENSE` during packaging.
The build adapter `wix/MeshThumbsWixUIExtension.cs` filters only the built-in
`FilesInUse` section from the WiX library before linking. This avoids duplicate
dialog IDs while preserving the original Minimal navigation, dialogs, and assets.
The adapter is a build tool and is not executed by the installed application.

WiX UI sources and artwork retain the **Microsoft Reciprocal License (MS-RL)**
and the copyright of the .NET Foundation and contributors. The full license is
in [third-party notices](THIRD-PARTY-NOTICES.txt) and the source archive. The
independent MeshThumbs application, original installer authoring, and build
adapter remain MIT.

The upstream sources are pinned to tag `wix3141rtm`, commit
`b40e9a32c24033e11b77baf2c91a704382f898ed`:

- [UIExtension source and artwork](https://github.com/wixtoolset/wix3/tree/b40e9a32c24033e11b77baf2c91a704382f898ed/src/ext/UIExtension)
- [Upstream license](https://github.com/wixtoolset/wix3/blob/b40e9a32c24033e11b77baf2c91a704382f898ed/LICENSE.TXT)
- [Offline WiX source archive](third-party/wix-source-3.14.1.zip)

The MSI installs `wix-source-3.14.1.zip` with the complete unmodified
upstream source tree, including UIExtension, the native print helper and shared
libraries, and license, plus `meshthumbs-installer-source.zip`
with this build's original WiX files, build adapter, packaging/restart/refresh
scripts, and MIT license. These archives accompany the installer UI and are
not runtime code.

To rebuild MeshThumbs, use the repository's [build instructions](USAGE.md#build)
and WiX 3.14.1, including `WixUIExtension.dll` beside `light.exe` and the .NET
Framework 4 C# compiler supplied with Windows. The archive of original
installer files is a source snapshot; building the complete MSI
also needs the application/native payloads described in those instructions.
