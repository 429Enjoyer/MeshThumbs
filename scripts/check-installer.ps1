param([Parameter(Mandatory=$true)][string]$Msi)
# Read-only package regression checks. Does not install or run custom actions.
$ErrorActionPreference = 'Stop'
function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $null
function Read-Rows([string]$Query, [int]$Columns) {
    $view = $database.OpenView($Query)
    try {
        $null = $view.Execute()
        while ($record = $view.Fetch()) {
            try {
                $values = @()
                for ($i = 1; $i -le $Columns; $i++) { $values += $record.StringData($i) }
                Write-Output -NoEnumerate $values
            } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record) }
        }
    } finally {
        $null = $view.Close()
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
    }
}
try {
    $database = $installer.OpenDatabase((Resolve-Path -LiteralPath $Msi).Path,0)
    $properties = @{}
    foreach ($row in (Read-Rows 'SELECT `Property`, `Value` FROM `Property`' 2)) { $properties[$row[0]] = $row[1] }
    Assert ($properties['MSIRESTARTMANAGERCONTROL'] -eq 'Disable') 'Restart Manager must be fully disabled'
    $embedded = [Text.Encoding]::Unicode.GetString([Convert]::FromBase64String($properties['MeshThumbsExplorerScript']))
    Assert ($embedded -eq (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'restart-explorer.ps1') -Raw)) 'Embedded recovery script is stale'
    $controls = @(Read-Rows 'SELECT `Dialog_`, `Control`, `Text`, `Attributes` FROM `Control`' 4)
    foreach ($pair in @(@('FilesInUse','Restart'), @('ExitDialog','RestoreExplorer'))) {
        $control = @($controls | Where-Object { $_[0] -eq $pair[0] -and $_[1] -eq $pair[1] })
        Assert ($control.Count -eq 1) "Missing control: $pair"
        Assert (($control[0][2] -eq 'Restart Explorer') -and (([int]$control[0][3] -band 3) -eq 3)) "Recovery button must be visible and enabled: $pair"
    }
    $events = @(Read-Rows 'SELECT `Dialog_`, `Control_`, `Event`, `Argument` FROM `ControlEvent`' 4)
    $recovery = @($events | Where-Object { $_[2] -eq 'DoAction' -and $_[3] -eq 'RestartExplorerFromUI' })
    Assert ($recovery.Count -eq 2) 'Recovery must be connected to the two explicit buttons'
    foreach ($table in 'InstallExecuteSequence','InstallUISequence') {
        $actions = @(Read-Rows ('SELECT `Action` FROM `' + $table + '`') 1)
        Assert (-not @($actions | Where-Object { $_[0] -eq 'RestartExplorerFromUI' }).Count) 'Explorer restart must not run automatically'
    }
    Write-Output ('MSI ' + $properties['ProductVersion'] + ': Restart Manager disabled, recovery buttons wired, embedded script current, no automatic restart action.')
} finally {
    if ($database) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) }
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
}
