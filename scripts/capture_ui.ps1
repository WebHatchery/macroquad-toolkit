# Generic screenshot-capture wrapper for games using macroquad_toolkit::capture.
#
# Builds the game, then runs its exe once for the complete scene manifest with
# PREFIX_CAPTURE_* env vars set, and sanity-checks each PNG. Package name, exe path, and env-var
# prefix are derived from `cargo metadata`, so most games can call this with no
# arguments from their own directory:
#
#   & ..\macroquad-toolkit\scripts\capture_ui.ps1 -Scenes gameplay,map
#
# Or via a one-line per-game wrapper script. Override -Prefix / -ExeName only
# if the game's env-var prefix doesn't match its package name.
#
# One game process owns one window for the whole run. Start-Process creates that
# window hidden unless -Visible is passed, while the toolkit also applies its
# capture focus guard before the macroquad window appears.
#
# -Release builds and captures with the optimised binary. Default is debug,
# which is fine for most games; reach for -Release when a scene is heavy enough
# that an unoptimised build cannot render it in reasonable time. Toybox is the
# worked example: ~4500 loose toys, where a debug capture took over 25 minutes
# without producing a PNG and a release one finishes in seconds.

param(
    [string]$GameDir = (Get-Location).Path,
    [string]$Prefix,
    [string]$ExeName,
    [string[]]$Scenes = @("gameplay"),
    [int]$Frames = 150,
    [int]$WindowWidth = 0,
    [int]$WindowHeight = 0,
    [string]$OutputDir = "docs\verification",
    [int]$MinBytes = 40000,
    [switch]$SkipBuild,
    [switch]$Release,
    # Captures run with the game window hidden (macroquad_toolkit::capture::headless).
    # -Visible puts it back on the desktop, for when a capture comes out wrong
    # and you want to watch the scene play out.
    [switch]$Visible,
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath (Join-Path $GameDir "Cargo.toml"))) {
    throw "No Cargo.toml in '$GameDir' - run from a game directory or pass -GameDir."
}

Push-Location $GameDir
try {
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    # In a workspace, metadata lists every member; pick the one that owns GameDir.
    $manifest = (Resolve-Path (Join-Path $GameDir "Cargo.toml")).Path
    $package = $metadata.packages | Where-Object { $_.manifest_path -eq $manifest } | Select-Object -First 1
    if (-not $package) { throw "No package with manifest $manifest in cargo metadata." }
    if (-not $ExeName) { $ExeName = $package.name }
    if (-not $Prefix) { $Prefix = ($package.name -replace "-", "_").ToUpperInvariant() }
    $profileDir = if ($Release) { "release" } else { "debug" }
    $exe = Join-Path $metadata.target_directory "$profileDir\$ExeName.exe"

    if (-not $SkipBuild) {
        Write-Host "Building $($package.name) ($profileDir)..."
        if ($Release) { cargo build --release } else { cargo build }
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed." }
    }
    if (-not (Test-Path -LiteralPath $exe)) { throw "Missing executable: $exe" }

    $outDir = Join-Path $GameDir $OutputDir
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null

    $captures = foreach ($scene in $Scenes) {
        # A scene name is a game's own addressing scheme, not a filename. Games
        # that route by "area:rock_fields:0:day" hit an InvalidFilename panic on
        # Windows the moment that name reaches Join-Path, and the script then
        # reports "capture failed" for what is really an unwritable path. The
        # file gets a sanitised name; the game still receives the scene verbatim.
        $safe = [regex]::Replace($scene, '[^A-Za-z0-9._+-]', '_')
        $path = Join-Path $outDir ("ui_{0}.png" -f $safe)
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }

        [pscustomobject]@{ Scene = $scene; Path = $path }
    }

    $manifestPath = Join-Path $outDir (".capture_manifest_{0}.tsv" -f $PID)
    $manifestRows = $captures | ForEach-Object { "{0}`t{1}" -f $_.Scene, $_.Path }
    Set-Content -LiteralPath $manifestPath -Value $manifestRows -Encoding utf8

    Set-Item -Path "Env:${Prefix}_CAPTURE_MANIFEST" -Value $manifestPath
    Set-Item -Path "Env:${Prefix}_CAPTURE_FRAMES" -Value "$Frames"
    if ($WindowWidth -gt 0) { Set-Item -Path "Env:${Prefix}_WINDOW_WIDTH" -Value "$WindowWidth" }
    if ($WindowHeight -gt 0) { Set-Item -Path "Env:${Prefix}_WINDOW_HEIGHT" -Value "$WindowHeight" }
    Set-Item -Path "Env:${Prefix}_HEADLESS" -Value $(if ($Visible) { "0" } else { "1" })
    $stdoutPath = Join-Path $outDir (".capture_stdout_{0}.log" -f $PID)
    $stderrPath = Join-Path $outDir (".capture_stderr_{0}.log" -f $PID)
    try {
        $startArgs = @{
            FilePath = $exe
            PassThru = $true
            RedirectStandardOutput = $stdoutPath
            RedirectStandardError = $stderrPath
        }
        if (-not $Visible) { $startArgs.WindowStyle = "Hidden" }
        $proc = Start-Process @startArgs
        Write-Host ("Capturing {0} scenes in one process (PID {1})..." -f $captures.Count, $proc.Id)
        if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
            $proc.Kill()
            throw ("Capture batch did not exit within $TimeoutSeconds s. " +
                "Most likely the env-var prefix is wrong: this run used " +
                "'$Prefix', derived from the package name. Check what the " +
                "game passes to CaptureConfig::all_from_env and pass -Prefix to match.")
        }
        if ($proc.ExitCode -ne 0) {
            $details = @(
                if (Test-Path -LiteralPath $stdoutPath) { Get-Content -LiteralPath $stdoutPath -Tail 40 }
                if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Tail 40 }
            ) -join "`n"
            throw "Capture process exited with code $($proc.ExitCode).`n$details"
        }
    }
    finally {
        Remove-Item "Env:${Prefix}_CAPTURE_MANIFEST", "Env:${Prefix}_CAPTURE_FRAMES", "Env:${Prefix}_HEADLESS", "Env:${Prefix}_WINDOW_WIDTH", "Env:${Prefix}_WINDOW_HEIGHT" -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $manifestPath -Force -ErrorAction SilentlyContinue
        if ($proc -and $proc.ExitCode -eq 0) {
            Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
        }
    }

    foreach ($capture in $captures) {
        $path = $capture.Path
        if (-not (Test-Path -LiteralPath $path)) { throw "Capture failed: $path not created." }
        $bytes = (Get-Item -LiteralPath $path).Length
        if ($bytes -lt $MinBytes) { throw "Capture failed: $path is only $bytes bytes (likely blank/black)." }
        Write-Host ("Captured {0} ({1} bytes)" -f $path, $bytes)
    }
}
finally {
    Pop-Location
}
