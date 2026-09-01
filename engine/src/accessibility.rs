//! macOS Accessibility-permission checks (effectful adapter). `enigo` keystrokes
//! **silently fail** on macOS unless the app is a trusted Accessibility client,
//! so the GUI surfaces this state and lets the user grant it. Trust can also be
//! revoked at runtime — these functions just re-report the current state, never
//! panic. On every other OS there is nothing to grant, so both report `true`.

/// Whether this process is currently a trusted Accessibility client.
/// Always `true` off macOS.
#[cfg(target_os = "macos")]
pub fn is_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

/// Non-macOS: no Accessibility gate exists, so the process is always "trusted".
#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    true
}

/// Ask macOS to prompt the user to grant Accessibility trust: shows the system
/// dialog that deep-links to System Settings and registers this app in the
/// Accessibility list. Returns the trust state at call time (typically still
/// `false` the first time, until the user flips the switch). Always `true` off
/// macOS.
#[cfg(target_os = "macos")]
pub fn request_trust() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

/// Non-macOS: nothing to request; already "trusted".
#[cfg(not(target_os = "macos"))]
pub fn request_trust() -> bool {
    true
}
