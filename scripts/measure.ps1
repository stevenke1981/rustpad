param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Output)
$ErrorActionPreference = 'Stop'
$exePath = (Resolve-Path -LiteralPath $Exe).Path
New-Item -ItemType Directory -Force -Path $Output | Out-Null
$outPath = (Resolve-Path -LiteralPath $Output).Path
$rows = @()
for ($i = 0; $i -lt 5; $i++) {
    $report = Join-Path $outPath "startup-$i.json"
    $process = Start-Process -FilePath $exePath -ArgumentList @('--startup-probe', ('"'+$report+'"')) -WindowStyle Hidden -PassThru
    $peak = 0L
    while (-not $process.HasExited) {
        try { $process.Refresh(); $peak = [Math]::Max($peak, $process.PeakWorkingSet64) } catch {}
        Start-Sleep -Milliseconds 10
    }
    if ($process.ExitCode -ne 0) { throw "Probe failed: $($process.ExitCode)" }
    $result = Get-Content -LiteralPath $report -Raw -Encoding utf8 | ConvertFrom-Json
    $rows += [PSCustomObject]@{ms=$result.main_to_first_framebuffer_ms; sampled_startup_peak_bytes=$peak}
}
$benchmark = Join-Path $outPath 'benchmark.json'
$process = Start-Process -FilePath $exePath -ArgumentList @('--benchmark', ('"'+$benchmark+'"')) -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0) { throw 'Benchmark failed' }
$rows | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $outPath 'startup-samples.json')
[PSCustomObject]@{exe_bytes=(Get-Item $exePath).Length; startup_median_ms=($rows.ms | Sort-Object)[2]; sampled_startup_peak_median_bytes=($rows.sampled_startup_peak_bytes | Sort-Object)[2]} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $outPath 'summary.json')
