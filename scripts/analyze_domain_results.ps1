# Domain Disease Detection - Results Analysis Script
# Purpose: Parse test output, generate metrics, create publication-ready report
# Usage: ./analyze_domain_results.ps1 -TestOutput "path/to/output.txt"

param(
    [Parameter(Mandatory=$true)]
    [string]$TestOutput,

    [Parameter(Mandatory=$false)]
    [string]$OutputDir = "../results"
)

Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Domain Disease Detection: Results Analysis                   ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

# Create output directory if not exists
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Read test output
if (-not (Test-Path $TestOutput)) {
    Write-Host "❌ Error: Test output file not found: $TestOutput" -ForegroundColor Red
    exit 1
}

$content = Get-Content $TestOutput -Raw
Write-Host "`n✓ Read test output file ($([Math]::Round((Get-Item $TestOutput).Length / 1KB)) KB)" -ForegroundColor Green

# Extract metrics using regex
$domains = @("Genomic", "Code Quality", "Malware", "Injection", "Supply Chain", "Cryptographic")
$metrics = @()

foreach ($domain in $domains) {
    Write-Host "`nParsing: $domain..." -ForegroundColor Yellow

    # Pattern: "Domain: Severity: XXX | Score: 0.XXX"
    $pattern = "($domain).*?Severity: ([A-Z]+).*?Score: (0\.\d+)"
    $matches = [regex]::Matches($content, $pattern, 'IgnoreCase')

    if ($matches.Count -gt 0) {
        $match = $matches[0]
        $severity = $match.Groups[2].Value
        $score = $match.Groups[3].Value

        # Extract patterns count
        $patternMatch = [regex]::Match($content, "Patterns: (\d+)")
        $patterns = if ($patternMatch.Success) { $patternMatch.Groups[1].Value } else { "0" }

        $metrics += @{
            Domain = $domain
            Severity = $severity
            Score = [float]$score
            Patterns = [int]$patterns
        }

        Write-Host "  ✓ Severity: $severity, Score: $score, Patterns: $patterns" -ForegroundColor Green
    } else {
        Write-Host "  ⚠️  No data found" -ForegroundColor Yellow
    }
}

# Generate CSV
$csvPath = "$OutputDir/metrics.csv"
Write-Host "`nGenerating CSV: $csvPath" -ForegroundColor Cyan

$csvContent = "Domain,Severity,Score,Patterns`n"
foreach ($m in $metrics) {
    $csvContent += "$($m.Domain),$($m.Severity),$($m.Score),$($m.Patterns)`n"
}
Set-Content -Path $csvPath -Value $csvContent

# Calculate statistics
if ($metrics.Count -gt 0) {
    $meanScore = ($metrics | Measure-Object -Property Score -Average).Average
    $criticalCount = ($metrics | Where-Object { $_.Severity -eq "CRITICAL" }).Count
    $highCount = ($metrics | Where-Object { $_.Severity -eq "HIGH" }).Count
    $mediumCount = ($metrics | Where-Object { $_.Severity -eq "MEDIUM" }).Count
    $lowCount = ($metrics | Where-Object { $_.Severity -eq "LOW" }).Count

    Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║  AGGREGATE STATISTICS                                        ║" -ForegroundColor Cyan
    Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

    Write-Host "`nDomain Results: $($metrics.Count)/6 parsed" -ForegroundColor Green
    Write-Host "Mean Risk Score: $([Math]::Round($meanScore, 3))" -ForegroundColor Yellow
    Write-Host "`nSeverity Breakdown:" -ForegroundColor Cyan
    Write-Host "  🔴 CRITICAL: $criticalCount domains" -ForegroundColor Red
    Write-Host "  🟠 HIGH: $highCount domains" -ForegroundColor Yellow
    Write-Host "  🟡 MEDIUM: $mediumCount domains" -ForegroundColor Yellow
    Write-Host "  🟢 LOW: $lowCount domains" -ForegroundColor Green

    # Generate JSON summary
    $jsonPath = "$OutputDir/summary.json"
    $jsonContent = @{
        test_date = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
        total_domains = $metrics.Count
        mean_score = [Math]::Round($meanScore, 3)
        critical_count = $criticalCount
        high_count = $highCount
        medium_count = $mediumCount
        low_count = $lowCount
        domains = @()
    }

    foreach ($m in $metrics) {
        $jsonContent.domains += @{
            name = $m.Domain
            severity = $m.Severity
            score = $m.Score
            patterns = $m.Patterns
        }
    }

    $jsonContent | ConvertTo-Json | Set-Content -Path $jsonPath
    Write-Host "`n✓ Saved JSON summary: $jsonPath" -ForegroundColor Green

    # Generate HTML report
    $htmlPath = "$OutputDir/report.html"
    $htmlContent = @"
<!DOCTYPE html>
<html>
<head>
    <title>Domain Disease Detection Results</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; background: #f5f5f5; }
        h1 { color: #333; border-bottom: 3px solid #0066cc; padding-bottom: 10px; }
        table { border-collapse: collapse; width: 100%; background: white; margin: 20px 0; }
        th { background: #0066cc; color: white; padding: 12px; text-align: left; }
        td { padding: 12px; border-bottom: 1px solid #ddd; }
        tr:hover { background: #f9f9f9; }
        .critical { color: #d32f2f; font-weight: bold; }
        .high { color: #f57c00; font-weight: bold; }
        .medium { color: #fbc02d; font-weight: bold; }
        .low { color: #388e3c; font-weight: bold; }
        .summary { background: #e3f2fd; padding: 15px; border-radius: 5px; margin: 20px 0; }
        .metric { display: inline-block; margin: 10px 20px; }
    </style>
</head>
<body>
    <h1>Domain-Agnostic Disease Detection: Test Results</h1>
    <p><strong>Generated:</strong> $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")</p>

    <div class="summary">
        <h2>Summary Statistics</h2>
        <div class="metric"><strong>Mean Risk Score:</strong> $([Math]::Round($meanScore, 3))</div>
        <div class="metric"><strong>Critical Issues:</strong> <span class="critical">$criticalCount</span></div>
        <div class="metric"><strong>High Issues:</strong> <span class="high">$highCount</span></div>
        <div class="metric"><strong>Medium Issues:</strong> <span class="medium">$mediumCount</span></div>
        <div class="metric"><strong>Low Issues:</strong> <span class="low">$lowCount</span></div>
    </div>

    <h2>Domain Results</h2>
    <table>
        <tr>
            <th>Domain</th>
            <th>Severity</th>
            <th>Risk Score</th>
            <th>Patterns</th>
        </tr>
"@

    foreach ($m in $metrics) {
        $severityClass = $m.Severity.ToLower()
        $htmlContent += @"
        <tr>
            <td>$($m.Domain)</td>
            <td class="$severityClass">$($m.Severity)</td>
            <td>$($m.Score)</td>
            <td>$($m.Patterns)</td>
        </tr>
"@
    }

    $htmlContent += @"
    </table>

    <h2>Recommendations</h2>
    <ul>
"@

    if ($criticalCount -gt 0) {
        $htmlContent += "<li><strong>CRITICAL:</strong> Immediate intervention required for $criticalCount domain(s)</li>`n"
    }
    if ($highCount -gt 0) {
        $htmlContent += "<li><strong>HIGH:</strong> Urgent remediation needed within 24-48 hours for $highCount domain(s)</li>`n"
    }
    if ($mediumCount -gt 0) {
        $htmlContent += "<li><strong>MEDIUM:</strong> Plan remediation within 1-2 weeks for $mediumCount domain(s)</li>`n"
    }

    $htmlContent += @"
    </ul>

    <hr>
    <p style="color: #999; font-size: 12px;">Domain-Agnostic Disease Detection Framework v1.0</p>
</body>
</html>
"@

    Set-Content -Path $htmlPath -Value $htmlContent
    Write-Host "`n✓ Generated HTML report: $htmlPath" -ForegroundColor Green
}

Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Analysis Complete                                           ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host "`n📁 Output files:" -ForegroundColor Cyan
Write-Host "  • $csvPath" -ForegroundColor Green
Write-Host "  • $jsonPath" -ForegroundColor Green
Write-Host "  • $htmlPath" -ForegroundColor Green

Write-Host "`n✓ Analysis script completed successfully!" -ForegroundColor Green
