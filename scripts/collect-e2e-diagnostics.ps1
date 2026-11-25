# Collects what explains a failed end-to-end run: the WebView2, Edge and driver
# versions, the browser arguments that reached the app's WebView2, and a
# verbose msedgedriver log of one session attempt. tauri-driver cannot pass
# --verbose to msedgedriver, so this script drives msedgedriver directly.
# CI runs it when the end-to-end tests fail and uploads the output folder.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/collect-e2e-diagnostics.ps1
#        (the app under test comes from $env:NICELIBRARY_APP)
# It only stops processes it started itself.
$ErrorActionPreference = 'Continue'

$repo = Split-Path $PSScriptRoot -Parent
$out = Join-Path $repo 'target\e2e\diagnostics'
New-Item -ItemType Directory -Force $out | Out-Null
$summary = Join-Path $out 'summary.txt'
$app = (Resolve-Path $env:NICELIBRARY_APP).Path
$driverExe = Join-Path $repo 'target\e2e\msedgedriver.exe'

function Write-Summary([string]$line) { $line | Tee-Object -FilePath $summary -Append }

function Get-OwnWebView {
    # The main WebView2 process of this app; helpers carry --type=.
    Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -match 'webview-exe-name=nicelibrary' -and $_.CommandLine -notmatch '--type=' }
}

$clients = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients'
Write-Summary "app:               $app"
Write-Summary "WebView2 (registry): $((Get-ItemProperty "$clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue).pv)"
Write-Summary "WebView2 (folders):  $((Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -Directory -ErrorAction SilentlyContinue | Where-Object Name -Match '^\d' | ForEach-Object Name) -join ', ')"
Write-Summary "Edge (registry):     $((Get-ItemProperty "$clients\{56EB18F8-B008-4CBD-B6D2-8C97FE7E9062}" -ErrorAction SilentlyContinue).pv)"
Write-Summary "msedgedriver:        $(if (Test-Path $driverExe) { & $driverExe --version } else { 'missing' })"
Write-Summary "user/session:        $(whoami) / $((Get-Process -Id $PID).SessionId)"

if (-not (Test-Path $driverExe)) { exit 0 }

$log = Join-Path $out 'msedgedriver.log'
$driver = Start-Process $driverExe -ArgumentList '--port=9515', '--verbose', "--log-path=$log" -PassThru
Start-Sleep -Seconds 3
$body = @{ capabilities = @{ alwaysMatch = @{ browserName = 'webview2'; 'ms:edgeOptions' = @{ binary = $app; webviewOptions = @{} } } } } | ConvertTo-Json -Depth 6
$session = Start-Job {
    param($b)
    try { "created $((Invoke-RestMethod -Method Post -Uri http://127.0.0.1:9515/session -Body $b -ContentType 'application/json' -TimeoutSec 90).value.sessionId)" }
    catch { "failed: $($_.ErrorDetails.Message)" }
} -ArgumentList $body

Start-Sleep -Seconds 15
Write-Summary '--- WebView2 arguments of the app started by msedgedriver'
Get-OwnWebView | ForEach-Object { ($_.CommandLine -split ' --') | ForEach-Object { Write-Summary "  --$_" } }
Write-Summary "--- session: $($session | Wait-Job | Receive-Job)"

# msedgedriver started the app, so stopping its tree stops the app as well.
taskkill /PID $driver.Id /T /F | Out-Null
Get-OwnWebView | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Write-Summary "Full msedgedriver log: $log"
