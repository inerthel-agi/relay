param([string]$Url, [string]$Out, [int]$Width = 1440, [int]$Height = 900, [int]$Budget = 0)
# Headless Chrome screenshot of a local page. Short profile path: the default one is too long for Chrome.
$profile = Join-Path $env:TEMP "relay-site-chrome"
$args = @("--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
  "--user-data-dir=$profile", "--window-size=$Width,$Height", "--screenshot=$Out")
if ($Budget -gt 0) { $args += "--virtual-time-budget=$Budget" }
$args += $Url
$p = Start-Process -FilePath "C:\Program Files\Google\Chrome\Application\chrome.exe" -ArgumentList $args -Wait -PassThru -NoNewWindow -RedirectStandardError (Join-Path $env:TEMP "relay-site-chrome.log")
if (-not (Test-Path $Out)) { throw "No screenshot written for $Url (exit $($p.ExitCode))" }
