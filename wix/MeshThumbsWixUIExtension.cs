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
        int exitDialogs = 0;
        foreach (Section section in original.Sections)
        {
            bool filesInUse = false;
            Table dialogs = section.Tables["Dialog"];
            if (dialogs != null)
                foreach (Row row in dialogs.Rows)
                    if ((string)row[0] == "FilesInUse") filesInUse = true;
            if (filesInUse) { removed++; continue; }
            if (dialogs != null)
                foreach (Row row in dialogs.Rows)
                    if ((string)row[0] == "ExitDialog")
                    {
                        AddExplorerRecovery(section.Tables["Control"]);
                        exitDialogs++;
                    }
            filtered.Sections.Add(section);
        }
        if (removed != 1)
            throw new System.InvalidOperationException("Expected one WiX FilesInUse dialog to replace.");
        if (exitDialogs != 1)
            throw new System.InvalidOperationException("Expected one WiX ExitDialog to extend.");
        foreach (Localization localization in original.GetLocalizations(new string[] { "en-us" }))
            filtered.AddLocalization(localization);
        library = filtered;
        return library;
    }

    // Keep the standard completion dialog, navigation and artwork. Add an
    // explicit recovery action so a blank desktop never requires Task Manager.
    private static void AddExplorerRecovery(Table controls)
    {
        object next = null;
        foreach (Row row in controls.Rows)
            if ((string)row[1] == "Finish")
            {
                next = row[10];
                row[10] = "RestoreExplorer";
            }
        Row text = controls.CreateRow(null);
        text[0] = "ExitDialog"; text[1] = "ExplorerRecoveryText"; text[2] = "Text";
        text[3] = "135"; text[4] = "110"; text[5] = "220"; text[6] = "50"; text[7] = 3;
        text[9] = "If the desktop or taskbar is missing, click Restart Explorer. This closes folder windows and interrupts Explorer file operations.";
        Row button = controls.CreateRow(null);
        button[0] = "ExitDialog"; button[1] = "RestoreExplorer"; button[2] = "PushButton";
        button[3] = "135"; button[4] = "165"; button[5] = "105"; button[6] = "18"; button[7] = 3;
        button[9] = "Restart Explorer"; button[10] = next;
    }
}
