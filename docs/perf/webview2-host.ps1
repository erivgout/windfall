# Hosts one borderless WebView2 control over the whole primary screen, with
# remote debugging on, so run-canvas-bench.mjs can drive it with --cdp.
#
# This measures the system web view itself, the one Tauri uses on Windows,
# without needing a build of the app.
#
# It needs the WebView2 SDK assemblies, which are not in this repository.
# Download the Microsoft.Web.WebView2 package from nuget.org, unzip it into
# an empty folder, and pass that folder as -Sdk. Run with Windows PowerShell
# 5.1 (powershell.exe), which loads the .NET Framework build:
#
#   powershell -ExecutionPolicy Bypass -File webview2-host.ps1 -Sdk C:\tmp\webview2-sdk
#   node run-canvas-bench.mjs --cdp http://127.0.0.1:9333 --out results-webview2.json
#
# Close it with Alt+F4 or by stopping the process.

param(
  [Parameter(Mandatory = $true)][string]$Sdk,
  [string]$Url = "about:blank",
  [int]$Port = 9333,
  [string]$UserData = (Join-Path $env:TEMP "wf-bench-webview2-profile"),
  [string]$ExtraArguments = ""
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -Path (Join-Path $Sdk "lib\net462\Microsoft.Web.WebView2.Core.dll")
Add-Type -Path (Join-Path $Sdk "lib\net462\Microsoft.Web.WebView2.WinForms.dll")

# The managed assemblies load WebView2Loader.dll by name.
$env:PATH = (Join-Path $Sdk "runtimes\win-x64\native") + ";" + $env:PATH
$env:WEBVIEW2_USER_DATA_FOLDER = $UserData
# The same anti-throttling switches the Edge runs use, so a covered window
# keeps animating. None of them changes how fast drawing is.
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = (
  "--remote-debugging-port=$Port " +
  "--disable-backgrounding-occluded-windows " +
  "--disable-renderer-backgrounding " +
  "--disable-background-timer-throttling " +
  $ExtraArguments
).Trim()

$form = New-Object System.Windows.Forms.Form
$form.Text = "Windfall WebView2 benchmark host"
$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::None
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
$form.Bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$form.TopMost = $true

$webView = New-Object Microsoft.Web.WebView2.WinForms.WebView2
$webView.Dock = [System.Windows.Forms.DockStyle]::Fill
$form.Controls.Add($webView)
$webView.Source = [Uri]$Url

[System.Windows.Forms.Application]::Run($form)
