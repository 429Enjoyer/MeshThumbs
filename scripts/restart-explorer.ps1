# Embedded in the MSI UI action so it works before installed files are replaced.
# This script runs only when the user clicks Restart Explorer, never unattended.
$ErrorActionPreference = 'Stop'

function Get-OwnedExplorer {
    param([int]$Session, [string]$OwnerSid, [string]$Executable)
    foreach ($process in @(Get-Process -Name explorer -ErrorAction SilentlyContinue)) {
        if ($process.SessionId -ne $Session) { continue }
        $null = $process.Handle # Pin the process identity before validating it.
        # Do not terminate similarly named executables or another user's shell.
        if (-not [string]::Equals($process.Path, $Executable, [StringComparison]::OrdinalIgnoreCase)) { continue }
        $instance = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$($process.Id)"
        if (-not $instance) { continue }
        $owner = Invoke-CimMethod -InputObject $instance -MethodName GetOwnerSid
        if ($owner.ReturnValue -eq 0 -and $owner.Sid -eq $OwnerSid) { $process }
    }
}

function Restart-CurrentUserExplorer {
    $session = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
    if ($session -eq 0 -or -not [Environment]::UserInteractive) {
        throw 'Restart Explorer is available only in an interactive user session.'
    }
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $executable = Join-Path $env:WINDIR 'explorer.exe'
    $processes = @(Get-OwnedExplorer -Session $session -OwnerSid $sid -Executable $executable)
    if ($processes.Count -eq 0) {
        throw 'No Explorer process owned by this user was found. Close other applications in the list, then click Retry.'
    }

    $stopped = $false
    try {
        foreach ($process in $processes) {
            # Retain Process objects from the snapshot; never reacquire by name
            # and accidentally terminate the shell Windows has just restored.
            if (-not $process.HasExited) {
                $process.Kill()
                $stopped = $true
                if (-not $process.WaitForExit(3000)) { throw 'Explorer did not exit. Please restart it using Task Manager.' }
            }
        }
    } finally {
        if ($stopped) {
            $restored = $false
            for ($attempt = 0; $attempt -lt 20; $attempt++) {
                if (@(Get-OwnedExplorer -Session $session -OwnerSid $sid -Executable $executable).Count) {
                    $restored = $true
                    break
                }
                Start-Sleep -Milliseconds 250
            }
            # Winlogon normally restores Explorer. Do not open a second window
            # if recovery happened during the final polling interval.
            if (-not $restored -and -not @(Get-OwnedExplorer -Session $session -OwnerSid $sid -Executable $executable).Count) {
                Start-Process -FilePath $executable -WindowStyle Hidden
            }
        }
    }
}

try {
    Restart-CurrentUserExplorer
} catch {
    # The action does not abort setup: the FilesInUse dialog retains Retry,
    # Ignore and Cancel. Show the reason instead of silently doing nothing.
    if ([Environment]::UserInteractive -and [System.Diagnostics.Process]::GetCurrentProcess().SessionId -ne 0) {
        Add-Type -AssemblyName System.Windows.Forms
        [void][System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'MeshThumbs - Restart Explorer', 'OK', 'Warning')
    }
    exit 1
}
