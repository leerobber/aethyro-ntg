# Developer shortcuts for aethyro-ntg (Windows PowerShell).
# Usage (from repo root):
#   .\tools\dev.ps1 school
#   .\tools\dev.ps1 test
#   .\tools\dev.ps1 phase4

$ErrorActionPreference = "Stop"

# tools/ is under repo root
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "kernel"))) {
    throw "Cannot find kernel/ under $Root — run from repo clone"
}
$K = Join-Path $Root "kernel"
$Docs = Join-Path $Root "docs"
$Art = if ($env:ARTIFACTS) { $env:ARTIFACTS } else { Join-Path $Root "artifacts" }
$SchoolOut = Join-Path $Docs "schooling\runs"

Set-Location $K

$cmd = if ($args.Count -ge 1) { $args[0] } else { "help" }
$rest = @()
if ($args.Count -gt 1) { $rest = $args[1..($args.Count - 1)] }

function Invoke-Cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArgs)
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

switch -Regex ($cmd) {
    '^test$' {
        Invoke-Cargo test @rest
    }
    '^test-quiet$' {
        Invoke-Cargo test -q
    }
    '^clippy$' {
        Invoke-Cargo clippy -- -D warnings
    }
    '^check$' {
        Invoke-Cargo test -q
        Invoke-Cargo clippy -- -D warnings
    }
    '^phase4$' {
        Invoke-Cargo run --release --bin phase4_calib -- --docs $Docs --json @rest
    }
    '^phase4-fixtures$' {
        Invoke-Cargo run --release --bin phase4_calib -- --json @rest
    }
    '^phase4-self-mod$' {
        Invoke-Cargo run --release --bin phase4_calib -- --docs $Docs --self-mod --json @rest
    }
    '^density$' {
        Invoke-Cargo run --release --bin density_bench @rest
    }
    '^graph-overhead$' {
        Invoke-Cargo run --release --bin graph_overhead_bench @rest
    }
    '^model$' {
        $models = Join-Path $Art "models"
        New-Item -ItemType Directory -Force -Path $models | Out-Null
        $Model = if ($env:MODEL_PATH) { $env:MODEL_PATH } else { Join-Path $models "ntg.calib" }
        $Sparse = if ($env:SPARSE_PATH) { $env:SPARSE_PATH } else { Join-Path $models "ntg.sparse" }
        $Report = if ($env:REPORT_PATH) { $env:REPORT_PATH } else { Join-Path $models "ntg.report.json" }
        Invoke-Cargo run --release --bin phase4_calib -- --docs $Docs `
            --write-model $Model --write-sparse $Sparse --write-report $Report --json
        Invoke-Cargo run --release --bin phase4_calib -- --docs $Docs --eval-model $Model --json
        Invoke-Cargo run --release --bin phase4_calib -- --eval-model $Model `
            --predict 'fn main() { println!("hi"); }' --sparse-score
        Write-Host "# artifacts: $Model $Sparse $Report"
    }
    '^school$' {
        $runs = if ($env:SCHOOL_RUNS) { $env:SCHOOL_RUNS } else { "5" }
        $maxA = if ($env:SCHOOL_MAX_ATTEMPTS) { $env:SCHOOL_MAX_ATTEMPTS } else { "5" }
        New-Item -ItemType Directory -Force -Path $SchoolOut | Out-Null
        Invoke-Cargo run --release --bin ntg_school -- `
            --docs $Docs `
            --out $SchoolOut `
            --runs $runs `
            --max-attempts $maxA
    }
    '^school-phase$' {
        $p = if ($rest.Count -ge 1) { $rest[0] } else { "4" }
        $runs = if ($env:SCHOOL_RUNS) { $env:SCHOOL_RUNS } else { "3" }
        New-Item -ItemType Directory -Force -Path $SchoolOut | Out-Null
        Invoke-Cargo run --release --bin ntg_school -- `
            --docs $Docs `
            --out $SchoolOut `
            --phase $p `
            --runs $runs
    }
    '^all$' {
        Invoke-Cargo test -q
        Invoke-Cargo run --release --bin density_bench
        Invoke-Cargo run --release --bin graph_overhead_bench
        Invoke-Cargo run --release --bin phase4_calib -- --docs $Docs --json
        New-Item -ItemType Directory -Force -Path $SchoolOut | Out-Null
        Invoke-Cargo run --release --bin ntg_school -- --docs $Docs --out $SchoolOut --runs 3
    }
    default {
        Write-Host @"
usage: .\tools\dev.ps1 <cmd>

  test | test-quiet     cargo test
  clippy | check        clippy; check = test + clippy
  phase4                calib on docs/ + --json
  phase4-fixtures       calib on fixtures
  phase4-self-mod       calib + --self-mod
  density               density_bench
  graph-overhead        graph_overhead_bench
  model                 train -> artifacts/models
  school                doctorate ntg_school (SCHOOL_RUNS default 5)
  school-phase N        school only phase N
  all                   test + benches + phase4 + school(3)

Env: ARTIFACTS, MODEL_PATH, SCHOOL_RUNS, SCHOOL_MAX_ATTEMPTS

Native cargo (no script):
  cd kernel
  cargo run --release --bin ntg_school -- --docs ../docs --out ../docs/schooling/runs --runs 5
"@
    }
}
