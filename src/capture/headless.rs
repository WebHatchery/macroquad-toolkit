//! Hide the game window during automated runs (screenshot capture, playtest bots).
//!
//! macroquad has no offscreen mode: miniquad must create a real OS window to
//! own the GL context, and it calls `ShowWindow(SW_SHOW)` the moment that
//! context exists. So a capture run or a `--bot` run pops a full game window
//! onto the desktop, takes focus, and sits there for the whole run.
//!
//! This module takes that window back off the desktop. Rendering is unaffected:
//! the frame is still drawn into the back buffer, which the driver owns whether
//! or not the window is mapped, and `get_screen_data()` copies out of the back
//! buffer *before* the swap. Only presentation to the desktop is skipped, so
//! screenshots and bot runs behave exactly as they did with a visible window.
//!
//! # Enabling
//!
//! `PREFIX_HEADLESS` controls it, defaulting to **on whenever capture mode is
//! active** (`PREFIX_CAPTURE_MANIFEST` set):
//!
//! - capture run — headless by default; set `PREFIX_HEADLESS=0` to watch it
//! - bot / normal run — visible by default; set `PREFIX_HEADLESS=1` to hide it
//!
//! # Wiring
//!
//! [`capture_window_conf`](super::capture_window_conf) already calls [`arm`],
//! so games that build their `Conf` through it get this for free. A game with a
//! hand-built `Conf` should call `headless::arm("PREFIX")` from its
//! `window_conf()` — that runs before the window exists, which is the point:
//! [`arm`] leaves a watcher behind that hides the window the instant miniquad
//! shows it, so there is no visible flash.
//!
//! Windows-only. On other native platforms and in wasm builds, arming and
//! hiding are no-ops.

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};

/// True when the process should run without a visible window: `PREFIX_HEADLESS`
/// if set, otherwise on exactly when capture mode is active.
pub fn headless_requested(prefix: &str) -> bool {
    super::env_bool(
        &format!("{prefix}_HEADLESS"),
        super::capture_requested(prefix),
    )
}

/// Arm window hiding for this process, if `PREFIX_HEADLESS` asks for it.
///
/// Safe to call before the window exists — it spawns a watcher thread that
/// hides the window as soon as one appears, and keeps hiding it if something
/// (miniquad's own startup `SW_SHOW`, a fullscreen toggle) shows it again.
/// Repeat calls after the first are ignored.
pub fn arm(prefix: &str) {
    #[cfg(target_os = "windows")]
    {
        static ARMED: AtomicBool = AtomicBool::new(false);

        if !headless_requested(prefix) || ARMED.swap(true, Ordering::SeqCst) {
            return;
        }

        // Miniquad explicitly calls SW_SHOW while creating its GL window. A
        // CBT hook on that same UI thread rejects the activation before Windows
        // can move keyboard focus, while the watcher below keeps the window
        // hidden after creation.
        windows::prevent_activation();

        std::thread::spawn(|| {
            let start = std::time::Instant::now();
            loop {
                hide_window();
                // Tight polling only while the window is being created — the
                // gap between miniquad's SW_SHOW and our SW_HIDE is how long
                // the window is visible, so it wants to be a frame or two.
                // Afterwards this is just a cheap guard against a re-show.
                let interval = if start.elapsed() < std::time::Duration::from_secs(3) {
                    std::time::Duration::from_millis(4)
                } else {
                    std::time::Duration::from_millis(250)
                };
                std::thread::sleep(interval);
            }
        });
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = prefix;
    }
}

/// Hide this process's game window right now. Returns whether a visible window
/// was found and hidden; `false` on a platform without an implementation.
pub fn hide_window() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::hide()
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Minimal Win32 bindings. Four `extern` declarations against DLLs the process
/// already links (miniquad itself uses `user32`) is less weight than pulling in
/// `winapi`/`windows-sys` for this.
#[cfg(target_os = "windows")]
mod windows {
    use std::ffi::c_void;

    type Hwnd = *mut c_void;
    type Hhook = *mut c_void;
    type EnumProc = unsafe extern "system" fn(Hwnd, isize) -> i32;
    type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;

    const SW_HIDE: i32 = 0;
    const WH_CBT: i32 = 5;
    const HCBT_ACTIVATE: i32 = 5;
    /// The class miniquad registers for its game window. Checked so a stray
    /// top-level window of ours (a message box, a driver overlay) is left alone.
    const MINIQUAD_CLASS: &str = "MINIQUADAPP";

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(callback: EnumProc, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut u32) -> u32;
        fn GetClassNameW(hwnd: Hwnd, buffer: *mut u16, max_count: i32) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn ShowWindow(hwnd: Hwnd, command: i32) -> i32;
        fn SetWindowsHookExW(
            hook_id: i32,
            callback: Option<HookProc>,
            module: *mut c_void,
            thread_id: u32,
        ) -> Hhook;
        fn CallNextHookEx(hook: Hhook, code: i32, wparam: usize, lparam: isize) -> isize;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcessId() -> u32;
        fn GetCurrentThreadId() -> u32;
    }

    pub fn prevent_activation() {
        unsafe {
            SetWindowsHookExW(
                WH_CBT,
                Some(on_cbt_event),
                std::ptr::null_mut(),
                GetCurrentThreadId(),
            );
        }
    }

    pub fn hide() -> bool {
        let mut found: Hwnd = std::ptr::null_mut();
        unsafe {
            EnumWindows(on_window, &mut found as *mut Hwnd as isize);
            if found.is_null() {
                return false;
            }
            // Cross-thread SW_HIDE is legal: it posts to the owning thread,
            // which is inside miniquad's message pump every frame.
            ShowWindow(found, SW_HIDE);
        }
        true
    }

    /// `EnumWindows` callback: stop (return 0) at the first visible miniquad
    /// window belonging to this process, handing it back through `lparam`.
    unsafe extern "system" fn on_window(hwnd: Hwnd, lparam: isize) -> i32 {
        let mut owner = 0u32;
        GetWindowThreadProcessId(hwnd, &mut owner);
        if owner != GetCurrentProcessId() || IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut class = [0u16; 64];
        let written = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
        if written <= 0 || String::from_utf16_lossy(&class[..written as usize]) != MINIQUAD_CLASS {
            return 1;
        }

        *(lparam as *mut Hwnd) = hwnd;
        0
    }

    unsafe extern "system" fn on_cbt_event(code: i32, wparam: usize, lparam: isize) -> isize {
        if code == HCBT_ACTIVATE {
            let hwnd = wparam as Hwnd;
            if is_miniquad_window(hwnd) {
                ShowWindow(hwnd, SW_HIDE);
                return 1;
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    unsafe fn is_miniquad_window(hwnd: Hwnd) -> bool {
        let mut class = [0u16; 64];
        let written = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
        written > 0 && String::from_utf16_lossy(&class[..written as usize]) == MINIQUAD_CLASS
    }
}
