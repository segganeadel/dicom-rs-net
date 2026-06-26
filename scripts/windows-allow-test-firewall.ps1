# Allow inbound TCP on loopback for Rust integration tests (dicom-net, pacs-server, etc.).
# Run once from an elevated PowerShell prompt:
#   Set-ExecutionPolicy -Scope Process Bypass; .\scripts\windows-allow-test-firewall.ps1
#
# Why: `cargo test` builds one .exe per integration test with a content hash in the
# filename. Windows Firewall prompts for each new binary that listens on a port.

$ruleName = "Rust DICOM loopback test servers"

$existing = Get-NetFirewallRule -DisplayName $ruleName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "Rule already exists: $ruleName"
    exit 0
}

# Requires Administrator
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "Run this script in an elevated (Administrator) PowerShell window."
    exit 1
}

New-NetFirewallRule `
    -DisplayName $ruleName `
    -Description "Allow localhost TCP for cargo test binaries (dicom-net integration tests)" `
    -Direction Inbound `
    -Action Allow `
    -Enabled True `
    -Profile Any `
    -Protocol TCP `
    -LocalAddress 127.0.0.1 `
    -RemoteAddress 127.0.0.1 | Out-Null

Write-Host "Created firewall rule: $ruleName"
Write-Host "Loopback TCP inbound is now allowed without per-test-exe prompts."
