# Downloads the msedgedriver that matches the installed WebView2 runtime.
# tauri-driver needs it to drive the app's WebView2; a driver of another
# version refuses the session. The driver lands in target/e2e/, where the
# end-to-end tests look for it.
# Run from the repository root: pwsh scripts/install-msedgedriver.ps1
$ErrorActionPreference = 'Stop'

$runtimeKey = 'SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$version = $null
foreach ($root in 'HKLM:', 'HKCU:') {
    $item = Get-ItemProperty -Path "$root\$runtimeKey" -Name pv -ErrorAction SilentlyContinue
    if ($item -and $item.pv -and $item.pv -ne '0.0.0.0') {
        $version = $item.pv
        break
    }
}
if (-not $version) {
    throw 'No WebView2 runtime found. Install it from https://developer.microsoft.com/microsoft-edge/webview2/'
}

$dir = Join-Path $PWD 'target\e2e'
$driver = Join-Path $dir 'msedgedriver.exe'
if (Test-Path $driver) {
    $installed = (Get-Item $driver).VersionInfo.ProductVersion
    if ($installed -eq $version) {
        Write-Host "msedgedriver $version is already installed."
        exit 0
    }
}

New-Item -ItemType Directory -Force $dir | Out-Null
$zip = Join-Path $dir 'edgedriver_win64.zip'
Write-Host "Downloading msedgedriver $version"
Invoke-WebRequest -Uri "https://msedgedriver.microsoft.com/$version/edgedriver_win64.zip" -OutFile $zip -UseBasicParsing
Expand-Archive -Path $zip -DestinationPath $dir -Force
Remove-Item $zip
Write-Host "Installed $driver"
