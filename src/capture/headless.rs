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

/// Hide this process's game window right now. Returns whether its Macroquad
/// window was found; `false` on a platform without an implementation.
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

/// Resize the process-owned Macroquad window without changing its z-order,
/// position, or activation state. Hidden windows are eligible.
#[cfg(target_os = "windows")]
pub(crate) fn resize_window_client(
    width: i32,
    height: i32,
    hidden: bool,
) -> Result<String, String> {
    windows::resize_client(width, height, hidden)
}

#[cfg(target_os = "windows")]
pub(crate) fn window_size_report() -> Result<String, String> {
    windows::window_size_report()
}

/// Minimal Win32 bindings. A few `extern` declarations against DLLs the process
/// already links (miniquad itself uses `user32`) is less weight than pulling in
/// `winapi`/`windows-sys` for this.
#[cfg(target_os = "windows")]
mod windows {
    use std::ffi::c_void;

    type Hwnd = *mut c_void;
    type Hhook = *mut c_void;
    type EnumProc = unsafe extern "system" fn(Hwnd, isize) -> i32;
    type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    const SW_HIDE: i32 = 0;
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_NOOWNERZORDER: u32 = 0x0200;
    const SWP_NOSENDCHANGING: u32 = 0x0400;
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
        fn ShowWindow(hwnd: Hwnd, command: i32) -> i32;
        fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn AdjustWindowRectEx(rect: *mut Rect, style: u32, menu: i32, ex_style: u32) -> i32;
        fn SetWindowPos(
            hwnd: Hwnd,
            insert_after: Hwnd,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
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
        fn GetLastError() -> u32;
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
        let found = find_window();
        if found.is_null() {
            return false;
        }
        unsafe {
            // Cross-thread SW_HIDE is legal: it posts to the owning thread,
            // which is inside miniquad's message pump every frame.
            ShowWindow(found, SW_HIDE);
        }
        true
    }

    pub fn resize_client(width: i32, height: i32, hidden: bool) -> Result<String, String> {
        if width <= 0 || height <= 0 {
            return Err(format!("invalid native client size {width}x{height}"));
        }
        let hwnd = find_window();
        if hwnd.is_null() {
            return Err("no process-owned MINIQUADAPP window was found".to_owned());
        }
        let before = describe_window(hwnd)?;

        unsafe {
            let mut bounds = Rect {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if AdjustWindowRectEx(&mut bounds, style, 0, ex_style) == 0 {
                return Err(format!(
                    "AdjustWindowRectEx failed for the capture window: {}",
                    describe_window(hwnd).unwrap_or_else(|error| error)
                ));
            }
            let outer_width = bounds
                .right
                .checked_sub(bounds.left)
                .ok_or_else(|| "capture window width overflowed".to_owned())?;
            let outer_height = bounds
                .bottom
                .checked_sub(bounds.top)
                .ok_or_else(|| "capture window height overflowed".to_owned())?;
            if outer_width <= 0 || outer_height <= 0 {
                return Err(format!(
                    "AdjustWindowRectEx produced invalid outer size {outer_width}x{outer_height}; {before}"
                ));
            }
            let mut flags = SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
            if hidden {
                // Default window processing applies the monitor max-track limit
                // during WM_WINDOWPOSCHANGING. Headless capture must keep the
                // requested client dimensions even when its window is hidden.
                flags |= SWP_NOSENDCHANGING;
            }
            if SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                outer_width,
                outer_height,
                flags,
            ) == 0
            {
                let error = GetLastError();
                return Err(format!(
                    "SetWindowPos failed with Win32 error {error}; target outer {outer_width}x{outer_height}; before {before}"
                ));
            }
            let after = describe_window(hwnd)?;
            Ok(format!(
                "target client {width}x{height}, outer {outer_width}x{outer_height}, flags 0x{flags:04X}; before [{before}], immediately after [{after}]"
            ))
        }
    }

    pub fn window_size_report() -> Result<String, String> {
        let hwnd = find_window();
        if hwnd.is_null() {
            return Err("no process-owned MINIQUADAPP window was found".to_owned());
        }
        describe_window(hwnd)
    }

    fn describe_window(hwnd: Hwnd) -> Result<String, String> {
        unsafe {
            let mut outer = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut outer) == 0 {
                return Err(format!(
                    "GetWindowRect failed with Win32 error {}",
                    GetLastError()
                ));
            }
            let mut client = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut client) == 0 {
                return Err(format!(
                    "GetClientRect failed with Win32 error {}",
                    GetLastError()
                ));
            }
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            Ok(format!(
                "hwnd {hwnd:p}, style 0x{style:08X}, exstyle 0x{ex_style:08X}, outer {}x{} at ({},{}), client {}x{}",
                outer.right - outer.left,
                outer.bottom - outer.top,
                outer.left,
                outer.top,
                client.right - client.left,
                client.bottom - client.top
            ))
        }
    }

    fn find_window() -> Hwnd {
        let mut found: Hwnd = std::ptr::null_mut();
        unsafe {
            EnumWindows(on_window, &mut found as *mut Hwnd as isize);
        }
        found
    }

    /// `EnumWindows` callback: stop at the first process-owned miniquad
    /// window, including hidden windows, handing it back through `lparam`.
    unsafe extern "system" fn on_window(hwnd: Hwnd, lparam: isize) -> i32 {
        let mut owner = 0u32;
        GetWindowThreadProcessId(hwnd, &mut owner);
        if owner != GetCurrentProcessId() {
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
