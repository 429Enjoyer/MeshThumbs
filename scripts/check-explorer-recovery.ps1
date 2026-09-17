# Tests use only fake processes/windows. They never stop or launch real Explorer.
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'restart-explorer.ps1') -DefinitionsOnly

function Assert($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Start-Sleep { param($Milliseconds) }
function Test-ExplorerProcessInSession { param($Session) return $script:Scenario -eq 'other-user' }
function Start-Process {
    param($FilePath, $WindowStyle)
    Assert ($FilePath -eq (Join-Path $env:WINDIR 'explorer.exe')) 'Unexpected executable'
    Assert ($WindowStyle -eq 'Normal') 'Recovered desktop must be visible'
    $script:Launches++
}
function New-FakeExplorer {
    $process = [pscustomobject]@{ HasExited = $false }
    $process | Add-Member ScriptMethod Kill { $this.HasExited = $true; $script:Kills++ }
    $process | Add-Member ScriptMethod WaitForExit { param($Timeout) return $true }
    return $process
}
function Get-OwnedExplorer {
    param($Session, $OwnerSid, $Executable)
    if ($script:Scenario -in 'absent','other-user') { return }
    if (-not $script:Old.HasExited) { return $script:Old }
    if ($script:Scenario -eq 'stuck') { return (New-FakeExplorer) }
}
function Test-ExplorerDesktop {
    param($Session, $OwnerSid, $Executable)
    $script:Checks++
    if ($script:Scenario -eq 'automatic') { return $script:Old.HasExited }
    if ($script:Scenario -eq 'stuck') { return $false }
    return $script:Launches -eq 1
}

foreach ($scenario in 'automatic', 'absent', 'manual') {
    $script:Scenario = $scenario
    $script:Old = New-FakeExplorer
    $script:Launches = 0; $script:Kills = 0; $script:Checks = 0
    Restart-CurrentUserExplorer
    Assert ($script:Launches -eq [int]($scenario -ne 'automatic')) "Wrong launch count: $scenario"
    Assert ($script:Kills -eq [int]($scenario -ne 'absent')) "Wrong kill count: $scenario"
    Assert ($script:Checks -gt 0) 'Must check desktop readiness'
}
$script:Scenario = 'stuck'; $script:Old = New-FakeExplorer
$script:Launches = 0; $script:Kills = 0; $script:Checks = 0
$failed = $false
try { Restart-CurrentUserExplorer } catch { $failed = $_.Exception.Message -like '*desktop and taskbar*' }
Assert $failed 'A background process without desktop must not count as recovered'
Assert ($script:Launches -eq 0) 'Do not open a folder through an already-running shell'
$script:Scenario = 'other-user'; $script:Launches = 0; $script:Kills = 0
$failed = $false
try { Restart-CurrentUserExplorer } catch { $failed = $_.Exception.Message -like '*another account*' }
Assert ($failed -and $script:Launches -eq 0 -and $script:Kills -eq 0) 'Never launch over an unverified or other-user shell'
Write-Output 'Explorer recovery checks passed: automatic recovery, absent shell, one fallback launch, unready desktop, and other-user isolation.'
