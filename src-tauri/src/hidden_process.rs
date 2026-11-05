use std::process::Command;

/// Windows `CREATE_NO_WINDOW`: helper tools (FFmpeg, yt-dlp, reg.exe…) run
/// without flashing a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Windows `BELOW_NORMAL_PRIORITY_CLASS`: a long helper task gives way to the
/// game and OBS while streaming.
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

pub(crate) fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Same as `hide_window`, at a lower priority than the user's programs.
pub(crate) fn hide_window_low_priority(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
    #[cfg(not(windows))]
    let _ = command;
}
