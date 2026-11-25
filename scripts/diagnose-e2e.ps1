# TEMPORARY: diagnoses the WebDriver session failure on the CI runner.
$ErrorActionPreference = 'Continue'
$base = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients'
"Edge browser pv: $((Get-ItemProperty "$base\{56EB18F8-B008-4CBD-B6D2-8C97FE7E9062}" -ErrorAction SilentlyContinue).pv)"
"WebView2 pv:     $((Get-ItemProperty "$base\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue).pv)"
"msedge.exe:      $((Get-Item 'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe' -ErrorAction SilentlyContinue).VersionInfo.ProductVersion)"
"driver:          $(& target\e2e\msedgedriver.exe --version)"
"EDGEWEBDRIVER:   $env:EDGEWEBDRIVER"
"WEBVIEW2 env:    $(Get-ChildItem env: | Where-Object Name -Like 'WEBVIEW2*' | Out-String)"
"user:            $(whoami)  session: $((Get-Process -Id $PID).SessionId)"

$app = (Resolve-Path $env:NICELIBRARY_APP).Path
$log = Join-Path $PWD 'msedgedriver.log'
$driver = Start-Process target\e2e\msedgedriver.exe -ArgumentList '--port=9515', '--verbose', "--log-path=$log" -PassThru
Start-Sleep -Seconds 3
$body = @{ capabilities = @{ alwaysMatch = @{ browserName = 'webview2'; 'ms:edgeOptions' = @{ binary = $app; webviewOptions = @{} } } } } | ConvertTo-Json -Depth 6
try {
    $r = Invoke-RestMethod -Method Post -Uri http://127.0.0.1:9515/session -Body $body -ContentType 'application/json' -TimeoutSec 120
    "session OK: $($r.value.sessionId)"
    Invoke-RestMethod -Method Delete -Uri "http://127.0.0.1:9515/session/$($r.value.sessionId)" | Out-Null
} catch {
    "session failed: $_"
}
Get-Process | Where-Object Name -Match 'msedgewebview2' | Select-Object -First 1 Path, @{ n = 'Version'; e = { $_.MainModule.FileVersionInfo.ProductVersion } } | Format-List
Stop-Process -Id $driver.Id -Force
Get-Process nicelibrary -ErrorAction SilentlyContinue | Stop-Process -Force
'--- msedgedriver.log (filtered) ---'
Get-Content $log | Select-String -Pattern 'Launching|DevTools|SEVERE|WARNING|ERROR|webview|exited|command line' | Select-Object -First 60
