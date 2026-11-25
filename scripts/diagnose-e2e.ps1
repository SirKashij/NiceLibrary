# TEMPORARY: diagnoses the WebDriver session failure on the CI runner.
$ErrorActionPreference = 'Continue'
$app = (Resolve-Path $env:NICELIBRARY_APP).Path

'=== Installed WebView2 runtime folders'
Get-ChildItem 'C:\Program Files (x86)\Microsoft\EdgeWebView\Application' -ErrorAction SilentlyContinue | Select-Object Name
'=== Preinstalled EdgeDriver'
& "$env:EDGEWEBDRIVER\msedgedriver.exe" --version

function Stop-OwnWebView {
    Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" | Where-Object CommandLine -Match 'webview-exe-name=nicelibrary' | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
}

function Show-WebView {
    Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -NotMatch '--type=' -and $_.CommandLine -Match 'webview-exe-name=nicelibrary' } |
        ForEach-Object { "webview2 main: $($_.ExecutablePath)"; ($_.CommandLine -split ' --') | ForEach-Object { "  --$_" } }
}

'=== Policies'
foreach ($k in 'HKLM:\SOFTWARE\Policies\Microsoft\Edge', 'HKCU:\SOFTWARE\Policies\Microsoft\Edge') { "$k"; Get-ItemProperty $k -ErrorAction SilentlyContinue | Out-String; Get-ChildItem $k -Recurse -ErrorAction SilentlyContinue | ForEach-Object { $_.Name; $_ | Get-ItemProperty | Out-String } }
'=== A: app started directly with remote debugging on 9222'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
$p = Start-Process -FilePath $app -PassThru
Start-Sleep -Seconds 10
Show-WebView
try { "port 9222: $((Invoke-WebRequest http://127.0.0.1:9222/json/version -UseBasicParsing -TimeoutSec 5).StatusCode)" } catch { "port 9222: $_" }
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Stop-OwnWebView
Remove-Item env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
Start-Sleep -Seconds 2

function Test-Session($driverExe, $label) {
    "=== $label"
    $log = Join-Path $PWD "$label.log"
    $d = Start-Process $driverExe -ArgumentList '--port=9515', '--verbose', "--log-path=$log" -PassThru
    Start-Sleep -Seconds 3
    $body = @{ capabilities = @{ alwaysMatch = @{ browserName = 'webview2'; 'ms:edgeOptions' = @{ binary = $app; webviewOptions = @{} } } } } | ConvertTo-Json -Depth 6
    $job = Start-Job { param($b) try { (Invoke-RestMethod -Method Post -Uri http://127.0.0.1:9515/session -Body $b -ContentType 'application/json' -TimeoutSec 90).value.sessionId } catch { "failed: $($_.ErrorDetails.Message)" } } -ArgumentList $body
    Start-Sleep -Seconds 15
    Show-WebView
    $result = $job | Wait-Job | Receive-Job
    "session: $result"
    Stop-Process -Id $d.Id -Force -ErrorAction SilentlyContinue
    Get-Process nicelibrary -ErrorAction SilentlyContinue | Stop-Process -Force; Stop-OwnWebView
    Start-Sleep -Seconds 2
}

Test-Session (Resolve-Path target\e2e\msedgedriver.exe).Path 'B-driver-matching-webview2'
Test-Session "$env:EDGEWEBDRIVER\msedgedriver.exe" 'C-preinstalled-edgedriver'
