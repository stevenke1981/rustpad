param(
    [Parameter(Mandatory)][string]$Baseline,
    [Parameter(Mandatory)][string]$Current,
    [Parameter(Mandatory)][string]$Output,
    [int]$Rounds = 6
)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Output) { throw 'Output must be a new directory' }
$versions = @{ baseline=(Resolve-Path -LiteralPath $Baseline).Path; current=(Resolve-Path -LiteralPath $Current).Path }
New-Item -ItemType Directory -Path $Output | Out-Null
$outPath = (Resolve-Path -LiteralPath $Output).Path
$rows = @()
$index = 0
for ($round = 0; $round -lt $Rounds; $round++) {
    foreach ($version in @('baseline', 'current', 'current', 'baseline')) {
        $report = Join-Path $outPath "startup-$index.json"
        $process = Start-Process -FilePath $versions[$version] -ArgumentList @('--startup-probe', ('"'+$report+'"')) -WindowStyle Hidden -PassThru
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        $peak = 0L
        while (-not $process.HasExited) {
            if ($timer.Elapsed.TotalSeconds -gt 30) { $process.Kill(); throw 'Owned probe timed out' }
            try { $process.Refresh(); $peak = [Math]::Max($peak, $process.PeakWorkingSet64) } catch {}
            Start-Sleep -Milliseconds 10
        }
        if ($process.ExitCode -ne 0) { throw "Probe failed: $($process.ExitCode)" }
        $result = Get-Content -LiteralPath $report -Raw -Encoding utf8 | ConvertFrom-Json
        $rows += [PSCustomObject]@{round=$round; version=$version; ms=$result.main_to_first_framebuffer_ms; peak_bytes=$peak}
        $index++
    }
}
$rows | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $outPath 'samples.json')
$summary = foreach ($version in @('baseline', 'current')) {
    $selected = @($rows | Where-Object version -eq $version)
    $times = @($selected.ms | Sort-Object)
    $peaks = @($selected.peak_bytes | Sort-Object)
    $middle = [int]($times.Count / 2)
    [PSCustomObject]@{
        version=$version; n=$times.Count; exe_bytes=(Get-Item -LiteralPath $versions[$version]).Length
        exe_sha256=(Get-FileHash -LiteralPath $versions[$version] -Algorithm SHA256).Hash
        median_ms=($times[$middle-1]+$times[$middle])/2; min_ms=$times[0]; max_ms=$times[-1]
        sampled_peak_median_bytes=($peaks[$middle-1]+$peaks[$middle])/2
    }
}
$summary | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $outPath 'summary.json')
$summary | Format-Table
