# PowerShell demo for Windows ? naive block vs local-loop done + PR export
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Bin = Join-Path $Root "target\release\prove.exe"
if (-not (Test-Path $Bin)) {
  cargo build --release --manifest-path (Join-Path $Root "Cargo.toml")
}

Write-Host "=== 1) trap eval ===" -ForegroundColor Cyan
& $Bin eval traps
if ($LASTEXITCODE -ne 0) { throw "eval traps failed" }

function Invoke-ProveScenario {
  param([string]$Name, [string]$Backend)
  $Tmp = Join-Path $env:TEMP ("prove_demo_" + $Name + "_" + [guid]::NewGuid().ToString("N").Substring(0,8))
  New-Item -ItemType Directory -Path $Tmp | Out-Null
  Copy-Item -Recurse (Join-Path $Root "fixtures\traps\visible-pass-hidden-fail\*") $Tmp
  Push-Location $Tmp
  try {
    Write-Host "`n=== $Name ($Backend) ===" -ForegroundColor Cyan
    & $Bin init | Out-Null
    $policy = @"
gates:
  test:
    commands:
      - [python, -m, pytest, -q, tests/test_visible.py]
      - [python, -m, pytest, -q, tests/test_hidden_contract.py]
    repair_limit: 2
  review:
    type: checklist
    require: [diff_non_empty, tests_fresh, no_todo_marker]
budgets:
  max_steps: 20
  max_minutes: 10
safety:
  deny_command_regex: []
"@
    [System.IO.File]::WriteAllText((Join-Path $Tmp ".prove\policy.yml"), $policy)
    & $Bin run "fix discount validation" --backend $Backend
    & $Bin status
    if ($Backend -eq "local-loop") {
      & $Bin pr
    }
  } finally {
    Pop-Location
  }
}

Invoke-ProveScenario -Name "naive_block" -Backend "naive"
Invoke-ProveScenario -Name "local_done" -Backend "local-loop"
Write-Host "`nDemo complete." -ForegroundColor Green
