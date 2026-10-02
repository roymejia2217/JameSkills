param(
    [switch]$Check,
    [switch]$PrintInstallPlan,
    [switch]$Help
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ($Help -or (-not $Check -and -not $PrintInstallPlan)) {
    Write-Output 'Usage: setup-windows.ps1 -Check | -PrintInstallPlan | -Help'
    if (-not $Help) { exit 2 }
    exit 0
}
if ($Check -and $PrintInstallPlan) {
    Write-Output 'Choose only one of -Check or -PrintInstallPlan.'
    exit 2
}

$kitSource = 'https://gpui-kit.com/docs/installation'
if ($PrintInstallPlan) {
    [ordered]@{
        schema_version = 1
        platform = 'windows'
        manual_steps = @(
            'Install Visual Studio 2022 Build Tools with Desktop development with C++ and a Windows 10/11 SDK.',
            'Install CMake and add it to PATH.',
            'Install Rust 1.95.0 for x86_64-pc-windows-msvc with rustfmt and clippy using rustup.'
        )
        command_preview = 'rustup toolchain install 1.95.0-x86_64-pc-windows-msvc --component rustfmt --component clippy'
        sources = @(
            $kitSource,
            'https://learn.microsoft.com/visualstudio/install/workload-component-id-vs-build-tools',
            'https://learn.microsoft.com/windows/apps/windows-app-sdk/downloads',
            'https://rustup.rs/'
        )
    } | ConvertTo-Json -Depth 5 -Compress
    exit 0
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    [ordered]@{
        schema_version = 1
        platform = 'windows'
        checks = @([ordered]@{
            id = 'host'
            status = 'unsupported'
            summary = 'Windows requirements cannot be observed from this operating system'
            source = $kitSource
        })
    } | ConvertTo-Json -Depth 4 -Compress
    exit 1
}

$checks = @()
$requiredMissing = $false
function Add-Check([string]$Id, [string]$Status, [string]$Summary, [string]$Source, [bool]$Required) {
    $script:checks += [ordered]@{ id = $Id; status = $Status; summary = $Summary; source = $Source }
    if ($Required -and $Status -ne 'pass') { $script:requiredMissing = $true }
}

$rustc = Get-Command rustc -ErrorAction SilentlyContinue
if ($null -ne $rustc) {
    $rustVersion = (& rustc --version 2>$null | Out-String).Trim()
    if ($rustVersion -match '^rustc 1\.95\.0\b') {
        Add-Check 'rust' 'pass' 'Rust 1.95.0 is active on PATH' 'https://doc.rust-lang.org/1.95.0/' $true
    } else {
        Add-Check 'rust' 'missing' 'Rust 1.95.0 is not active on PATH' 'https://rustup.rs/' $true
    }
} else {
    Add-Check 'rust' 'missing' 'rustc is not on PATH' 'https://rustup.rs/' $true
}

$targetList = @()
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $targetList = @(& rustup target list --installed 2>$null)
}
$activeToolchain = ''
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $activeToolchain = (& rustup show active-toolchain 2>$null | Out-String).Trim()
}
if ($activeToolchain -match '^1\.95\.0-x86_64-pc-windows-msvc\b') {
    Add-Check 'rust-msvc-toolchain' 'pass' 'Rust MSVC toolchain is active' 'https://rustup.rs/' $true
} else {
    Add-Check 'rust-msvc-toolchain' 'missing' 'Pinned Rust MSVC toolchain is not active' 'https://rustup.rs/' $true
}
if ($targetList -contains 'x86_64-pc-windows-msvc') {
    Add-Check 'rust-msvc-target' 'pass' 'MSVC target is installed' 'https://rustup.rs/' $true
} else {
    Add-Check 'rust-msvc-target' 'missing' 'x86_64-pc-windows-msvc target is not installed' 'https://rustup.rs/' $true
}

if (Get-Command cmake -ErrorAction SilentlyContinue) {
    Add-Check 'cmake' 'pass' 'CMake is on PATH' $kitSource $true
} else {
    Add-Check 'cmake' 'missing' 'CMake is not on PATH' $kitSource $true
}

$programFilesX86 = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFilesX86)
$vswhere = Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsInstallation = $null
if (Test-Path -LiteralPath $vswhere) {
    $vsInstallation = (& $vswhere -latest -version '[17.0,18.0)' -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null | Out-String).Trim()
}
if (-not [string]::IsNullOrWhiteSpace($vsInstallation)) {
    $compilerPattern = Join-Path $vsInstallation 'VC\Tools\MSVC\*\bin\Hostx64\x64\cl.exe'
    $compilerFound = @(Get-ChildItem -Path $compilerPattern -ErrorAction SilentlyContinue).Count -gt 0
    if ($compilerFound) {
        Add-Check 'visual-studio-cpp' 'pass' 'Visual Studio 2022 C++ compiler was detected' 'https://learn.microsoft.com/visualstudio/install/workload-component-id-vs-build-tools' $true
    } else {
        Add-Check 'visual-studio-cpp' 'missing' 'Visual Studio C++ compiler was not detected' 'https://learn.microsoft.com/visualstudio/install/workload-component-id-vs-build-tools' $true
    }
} else {
    Add-Check 'visual-studio-cpp' 'missing' 'Visual Studio 2022 C++ build tools were not detected' 'https://learn.microsoft.com/visualstudio/install/workload-component-id-vs-build-tools' $true
}

$sdkRoot = Join-Path $programFilesX86 'Windows Kits\10\Include'
$sdkVersions = @()
if (Test-Path -LiteralPath $sdkRoot) {
    $sdkVersions = @(Get-ChildItem -LiteralPath $sdkRoot -Directory -ErrorAction SilentlyContinue)
}
if ($sdkVersions.Count -gt 0) {
    Add-Check 'windows-sdk' 'pass' 'Windows SDK headers were detected' 'https://learn.microsoft.com/windows/apps/windows-app-sdk/downloads' $true
} else {
    Add-Check 'windows-sdk' 'missing' 'Windows 10/11 SDK headers were not detected' 'https://learn.microsoft.com/windows/apps/windows-app-sdk/downloads' $true
}

Add-Check 'display' 'unknown' 'Display backend requires a native window smoke test' $kitSource $false
Add-Check 'gpu' 'unknown' 'GPU backend requires a native renderer smoke test' $kitSource $false

[ordered]@{
    schema_version = 1
    platform = 'windows'
    architecture = [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString().ToLowerInvariant()
    checks = $checks
} | ConvertTo-Json -Depth 5 -Compress

if ($requiredMissing) { exit 1 }
exit 0
