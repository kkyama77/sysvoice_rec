<#
.SYNOPSIS
  Installs Rust toolchain on Windows using rustup and adds common components.

.DESCRIPTION
  This script downloads and runs the official rustup installer (windows executable),
  sets up the chosen default toolchain, and installs common components such as
  rustfmt, clippy, and rust-src. It is written to be idempotent and will skip
  steps that are already satisfied unless you pass -Force.

.PARAMETER DefaultToolchain
  The Rust toolchain to install / set as default. Default is 'stable'.

.PARAMETER Components
  Additional rustup components to add. Default: 'rustfmt','clippy','rust-src'.

.PARAMETER Force
  Re-run installer even if rustup is already installed. Also forces re-install of components.

.PARAMETER SkipPathUpdate
  If specified, the script will not try to persistently update the PATH environment
  (it will still update the PATH for the current process so commands work immediately).

.EXAMPLE
  .\install_rust.ps1
  Installs the stable toolchain and the default components.

.EXAMPLE
  .\install_rust.ps1 -DefaultToolchain nightly -Force
  Installs rustup (re-runs installer), sets nightly default, and installs components.

.NOTES
  - This script targets Windows PowerShell / PowerShell Core on Windows.
  - Visual Studio Build Tools (or equivalent C/C++ toolchain) may be required to build some crates (MSVC).
  - You may need to re-open terminals after the installer modifies environment variables.
#>

param(
    [string]$DefaultToolchain = "stable",
    [string[]]$Components = @("rustfmt","clippy","rust-src"),
    [switch]$Force,
    [switch]$SkipPathUpdate
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $ts = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
    Write-Host "[$ts] [$Level] $Message"
}

function Ensure-RunningOnWindows {
    if ($env:OS -notlike "Windows_NT") {
        Write-Log "This script is intended to run on Windows." "ERROR"
        throw "Unsupported OS"
    }
}

function Get-RustupPath {
    return (Get-Command rustup -ErrorAction SilentlyContinue)?.Source
}

function Ensure-TempFile {
    param([string]$Prefix = "rustup")
    $tmp = [IO.Path]::Combine([IO.Path]::GetTempPath(), "$Prefix-$([Guid]::NewGuid().ToString()).exe")
    return $tmp
}

function Download-File {
    param(
        [Parameter(Mandatory=$true)][string]$Uri,
        [Parameter(Mandatory=$true)][string]$Out
    )
    Write-Log "Downloading $Uri -> $Out"
    try {
        # Use Invoke-WebRequest which is available on Windows PowerShell and PowerShell Core on Windows
        Invoke-WebRequest -Uri $Uri -OutFile $Out -UseBasicParsing -TimeoutSec 120
    } catch {
        Write-Log "Failed to download $Uri : $($_.Exception.Message)" "ERROR"
        throw
    }
}

function Run-Installer {
    param([string]$InstallerPath, [string[]]$Args)
    $startInfo = @{
        FilePath = $InstallerPath
        ArgumentList = $Args
        Wait = $true
        NoNewWindow = $true
    }

    Write-Log "Running installer: $InstallerPath $($Args -join ' ')"
    $proc = Start-Process @startInfo -PassThru
    if ($proc.ExitCode -ne 0) {
        throw "Installer exited with code $($proc.ExitCode)"
    }
}

function Ensure-Rustup {
    param([string]$DefaultToolchain, [switch]$Force)
    $rustupPath = Get-RustupPath
    if ($rustupPath -and -not $Force) {
        Write-Log "rustup already present at: $rustupPath"
        return $false
    }

    $installer = Ensure-TempFile -Prefix "rustup-init"
    # Official rustup windows installer endpoint - will redirect to the appropriate executable
    $installerUri = "https://win.rustup.rs/"

    Download-File -Uri $installerUri -Out $installer

    # run with -y to accept defaults and set the specified default toolchain
    $args = @("-y", "--default-toolchain", $DefaultToolchain)

    try {
        Run-Installer -InstallerPath $installer -Args $args
        Write-Log "rustup installer completed."
        return $true
    } finally {
        if (Test-Path $installer) {
            Remove-Item $installer -Force -ErrorAction SilentlyContinue
        }
    }
}

function Update-Path-InSessionAndPersist {
    if (-not $SkipPathUpdate) {
        $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
        if (Test-Path $cargoBin) {
            if (-not ($env:PATH -split ";" | Where-Object { $_ -eq $cargoBin })) {
                Write-Log "Adding $cargoBin to PATH for current session."
                $env:PATH = "$cargoBin;$env:PATH"
            } else {
                Write-Log "$cargoBin already on PATH (session)."
            }

            # Persist to USER environment variable (so new terminals see it)
            try {
                $currentUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
                if ($currentUserPath -notlike "*$cargoBin*") {
                    Write-Log "Persisting $cargoBin to user PATH."
                    $newUserPath = if ([string]::IsNullOrEmpty($currentUserPath)) { $cargoBin } else { "$currentUserPath;$cargoBin" }
                    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
                    Write-Log "User PATH updated. You may need to restart terminals for changes to take effect."
                } else {
                    Write-Log "$cargoBin already present in user PATH."
                }
            } catch {
                Write-Log "Failed to persist PATH: $($_.Exception.Message)" "WARN"
            }
        } else {
            Write-Log "Expected cargo bin path not found: $cargoBin" "WARN"
        }
    } else {
        Write-Log "Skipping persistent PATH update as requested."
    }
}

function Ensure-ToolchainAndComponents {
    param(
        [string]$DefaultToolchain,
        [string[]]$Components,
        [switch]$Force
    )

    # Ensure rustup is in the PATH in this session
    $rustupPath = Get-RustupPath
    if (-not $rustupPath) {
        Write-Log "rustup not found in PATH after installer. Trying to refresh session PATH."
        $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
        if (Test-Path $cargoBin) {
            $env:PATH = "$cargoBin;$env:PATH"
            $rustupPath = Get-RustupPath
        }
    }

    if (-not $rustupPath) {
        throw "rustup not available in PATH. Ensure installation succeeded and restart your shell."
    }

    Write-Log "Updating rustup and installing toolchain [$DefaultToolchain]."
    & rustup self update

    # Install / update the default toolchain
    & rustup install $DefaultToolchain
    & rustup default $DefaultToolchain

    # Install components
    foreach ($component in $Components) {
        try {
            if ($Force) {
                Write-Log "Adding (force) component: $component"
            } else {
                Write-Log "Ensuring component installed: $component"
            }
            & rustup component add $component --toolchain $DefaultToolchain
        } catch {
            Write-Log "Failed to add component $component: $($_.Exception.Message)" "WARN"
        }
    }

    Write-Log "Toolchain and components setup complete."
}

try {
    Ensure-RunningOnWindows

    Write-Log "Starting Rust installation (DefaultToolchain=$DefaultToolchain)."

    $installed = Ensure-Rustup -DefaultToolchain $DefaultToolchain -Force:$Force
    if ($installed) {
        Write-Log "rustup was installed by this script."
    }

    Update-Path-InSessionAndPersist -SkipPathUpdate:$SkipPathUpdate

    Ensure-ToolchainAndComponents -DefaultToolchain $DefaultToolchain -Components $Components -Force:$Force

    Write-Log "Rust installation finished successfully." "INFO"
    Write-Log "Run 'rustc --version' and 'cargo --version' to verify."

} catch {
    Write-Log "Installation failed: $($_.Exception.Message)" "ERROR"
    exit 1
}
