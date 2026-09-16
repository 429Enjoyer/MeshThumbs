// Original MeshThumbs build adapter, MIT. WiX resources retain their MS-RL terms.
using Microsoft.Tools.WindowsInstallerXml;

[assembly: AssemblyDefaultWixExtension(typeof(MeshThumbsWixUIExtension))]

public sealed class MeshThumbsWixUIExtension : WixExtension
{
    private Library library;
    public override string DefaultCulture { get { return "en-us"; } }

    public override Library GetLibrary(TableDefinitionCollection definitions)
    {
        if (library != null) return library;
        var original = new Microsoft.Tools.WindowsInstallerXml.Extensions.UIExtension().GetLibrary(definitions);
        var filtered = new Library();
        int removed = 0;
        foreach (Section section in original.Sections)
        {
            bool filesInUse = false;
            Table dialogs = section.Tables["Dialog"];
            if (dialogs != null)
                foreach (Row row in dialogs.Rows)
                    if ((string)row[0] == "FilesInUse") filesInUse = true;
            if (filesInUse) { removed++; continue; }
            filtered.Sections.Add(section);
        }
        if (removed != 1)
            throw new System.InvalidOperationException("Expected one WiX FilesInUse dialog to replace.");
        foreach (Localization localization in original.GetLocalizations(new string[] { "en-us" }))
            filtered.AddLocalization(localization);
        library = filtered;
        return library;
    }
}
