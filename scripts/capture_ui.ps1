# Generic screenshot-capture wrapper for games using macroquad_toolkit::capture.
#
# Builds the game, then runs its exe once per scene with PREFIX_CAPTURE_* env
# vars set, and sanity-checks each PNG. Package name, exe path, and env-var
# prefix are derived from `cargo metadata`, so most games can call this with no
# arguments from their own directory:
#
#   & ..\macroquad-toolkit\scripts\capture_ui.ps1 -Scenes gameplay,map
#
# Or via a one-line per-game wrapper script. Override -Prefix / -ExeName only
# if the game's env-var prefix doesn't match its package name.
#
# The game window stays off the desktop for the whole run: the toolkit hides it
# as it is created, and the frames are read out of the back buffer, which does
# not need the window to be on screen. Pass -Visible to watch instead.
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

    foreach ($scene in $Scenes) {
        # A scene name is a game's own addressing scheme, not a filename. Games
        # that route by "area:rock_fields:0:day" hit an InvalidFilename panic on
        # Windows the moment that name reaches Join-Path, and the script then
        # reports "capture failed" for what is really an unwritable path. The
        # file gets a sanitised name; the game still receives the scene verbatim.
        $safe = [regex]::Replace($scene, '[^A-Za-z0-9._+-]', '_')
        $path = Join-Path $outDir ("ui_{0}.png" -f $safe)
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force }

        Set-Item -Path "Env:${Prefix}_CAPTURE_PATH" -Value $path
        Set-Item -Path "Env:${Prefix}_CAPTURE_SCENE" -Value $scene
        Set-Item -Path "Env:${Prefix}_CAPTURE_FRAMES" -Value "$Frames"
        Set-Item -Path "Env:${Prefix}_HEADLESS" -Value $(if ($Visible) { "0" } else { "1" })
        try {
            # Bounded wait. A game whose capture prefix does not match the one
            # it reads sees no PREFIX_CAPTURE_* vars at all, falls through to
            # its normal main loop, and runs forever with no window to close —
            # so an unbounded wait here hangs the caller with no clue why.
            $proc = Start-Process -FilePath $exe -PassThru -NoNewWindow
            if (-not $proc.WaitForExit($TimeoutSeconds * 1000)) {
                $proc.Kill()
                throw ("Capture '$scene' did not exit within $TimeoutSeconds s. " +
                    "Most likely the env-var prefix is wrong: this run used " +
                    "'$Prefix', derived from the package name. Check what the " +
                    "game passes to CaptureConfig::from_env and pass -Prefix to match.")
            }
        }
        finally {
            Remove-Item "Env:${Prefix}_CAPTURE_PATH", "Env:${Prefix}_CAPTURE_SCENE", "Env:${Prefix}_CAPTURE_FRAMES", "Env:${Prefix}_HEADLESS" -ErrorAction SilentlyContinue
        }

        if (-not (Test-Path -LiteralPath $path)) { throw "Capture failed: $path not created." }
        $bytes = (Get-Item -LiteralPath $path).Length
        if ($bytes -lt $MinBytes) { throw "Capture failed: $path is only $bytes bytes (likely blank/black)." }
        Write-Host ("Captured {0} ({1} bytes)" -f $path, $bytes)
    }
}
finally {
    Pop-Location
}
