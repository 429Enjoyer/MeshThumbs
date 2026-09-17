# Embedded in the MSI UI action so it works before installed files are replaced.
# This script runs only when the user clicks Restart Explorer, never unattended.
param([switch]$DefinitionsOnly)
$ErrorActionPreference = 'Stop'

function Get-OwnedExplorer {
    param([int]$Session, [string]$OwnerSid, [string]$Executable)
    foreach ($process in @(Get-Process -Name explorer -ErrorAction SilentlyContinue)) {
        try {
            if ($process.SessionId -ne $Session -or $process.HasExited) { continue }
            $null = $process.Handle # Pin the process identity before validating it.
            # Do not terminate similarly named executables or another user's shell.
            if (-not [string]::Equals($process.Path, $Executable, [StringComparison]::OrdinalIgnoreCase)) { continue }
            $instance = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$($process.Id)"
            if (-not $instance) { continue }
            $owner = Invoke-CimMethod -InputObject $instance -MethodName GetOwnerSid
            if ($owner.ReturnValue -eq 0 -and $owner.Sid -eq $OwnerSid) { $process }
        } catch {
            # Explorer may exit between enumeration and validation. Re-check on
            # the next poll; an unverified process must never be terminated.
            continue
        }
    }
}

function Test-ExplorerDesktop {
    param([int]$Session, [string]$OwnerSid, [string]$Executable)
    if (-not ('MeshThumbs.ExplorerDesktop' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace MeshThumbs {
    public static class ExplorerDesktop {
        [DllImport("user32.dll")] public static extern IntPtr GetShellWindow();
        [DllImport("user32.dll", CharSet=CharSet.Unicode)]
        public static extern IntPtr FindWindow(string className, string title);
        [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
        [DllImport("user32.dll", SetLastError=true)]
        private static extern IntPtr SendMessageTimeout(IntPtr window, uint message, UIntPtr wParam, IntPtr lParam, uint flags, uint timeout, out UIntPtr result);
        public static int Owner(IntPtr window) { uint pid; GetWindowThreadProcessId(window, out pid); return (int)pid; }
        public static bool Responds(IntPtr window) {
            UIntPtr result;
            return window != IntPtr.Zero && SendMessageTimeout(window, 0, UIntPtr.Zero, IntPtr.Zero, 3, 500, out result) != IntPtr.Zero;
        }
    }
}
'@
    }
    $desktop = [MeshThumbs.ExplorerDesktop]::GetShellWindow()
    $taskbar = [MeshThumbs.ExplorerDesktop]::FindWindow('Shell_TrayWnd', $null)
    if ($desktop -eq [IntPtr]::Zero -or $taskbar -eq [IntPtr]::Zero) { return $false }
    $owned = @(Get-OwnedExplorer -Session $Session -OwnerSid $OwnerSid -Executable $Executable)
    if ([MeshThumbs.ExplorerDesktop]::Owner($desktop) -notin $owned.Id -or
        [MeshThumbs.ExplorerDesktop]::Owner($taskbar) -notin $owned.Id) { return $false }
    return [MeshThumbs.ExplorerDesktop]::Responds($desktop) -and [MeshThumbs.ExplorerDesktop]::Responds($taskbar)
}

function Wait-ExplorerDesktop {
    param([int]$Session, [string]$OwnerSid, [string]$Executable, [int]$Attempts)
    for ($attempt = 0; $attempt -lt $Attempts; $attempt++) {
        if (Test-ExplorerDesktop -Session $Session -OwnerSid $OwnerSid -Executable $Executable) { return $true }
        Start-Sleep -Milliseconds 250
    }
    return $false
}

function Restore-ExplorerDesktop {
    param([int]$Session, [string]$OwnerSid, [string]$Executable)
    # Process existence is insufficient: RM can leave explorer.exe alive with
    # no desktop/taskbar. Wait for actual shell windows to respond.
    if (Wait-ExplorerDesktop -Session $Session -OwnerSid $OwnerSid -Executable $Executable -Attempts 40) { return }
    if (-not @(Get-OwnedExplorer -Session $Session -OwnerSid $OwnerSid -Executable $Executable).Count) {
        if (Test-ExplorerProcessInSession -Session $Session) {
            throw 'Explorer is running under another account or cannot be verified. Run setup from the same Windows account as your desktop.'
        }
        # This is the user's visible desktop, not a hidden background helper.
        Start-Process -FilePath $Executable -WindowStyle Normal
    }
    if (-not (Wait-ExplorerDesktop -Session $Session -OwnerSid $OwnerSid -Executable $Executable -Attempts 60)) {
        throw 'Windows has not restored the desktop and taskbar. Finish setup and restart Windows to complete recovery.'
    }
}

function Test-ExplorerProcessInSession {
    param([int]$Session)
    return @(Get-Process -Name explorer -ErrorAction SilentlyContinue | Where-Object { $_.SessionId -eq $Session }).Count -gt 0
}

function Restart-CurrentUserExplorer {
    $session = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
    if ($session -eq 0 -or -not [Environment]::UserInteractive) {
        throw 'Restart Explorer is available only in an interactive user session.'
    }
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $executable = Join-Path $env:WINDIR 'explorer.exe'
    $processes = @(Get-OwnedExplorer -Session $session -OwnerSid $sid -Executable $executable)

    $stopped = $false
    try {
        foreach ($process in $processes) {
            # Retain Process objects from the snapshot; never reacquire by name
            # and accidentally terminate the shell Windows has just restored.
            if (-not $process.HasExited) {
                $process.Kill()
                $stopped = $true
                if (-not $process.WaitForExit(3000)) { throw 'Explorer did not exit. Finish setup and restart Windows to complete recovery.' }
            }
        }
    } finally {
        if ($stopped -or $processes.Count -eq 0) {
            Restore-ExplorerDesktop -Session $session -OwnerSid $sid -Executable $executable
        }
    }
}

if ($DefinitionsOnly) { return }

try {
    Restart-CurrentUserExplorer
} catch {
    # The action does not abort setup. The files-in-use and completion dialogs
    # remain available; show a recovery failure rather than claiming success.
    if ([Environment]::UserInteractive -and [System.Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) {
        Add-Type -AssemblyName System.Windows.Forms
        [void][System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'MeshThumbs - Restart Explorer', 'OK', 'Warning')
    }
    exit 1
}
