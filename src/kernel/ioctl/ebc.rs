use std::{fs::{File, OpenOptions}, os::unix::fs::OpenOptionsExt};

use nix::ioctl_readwrite;

const DRM_IOCTL_MAGIC: u8 = 'd';
const DRM_COMMAND_BASE: u8 = 0x40;


#[repr(C)]
struct TriggerGlobalRefresh {
    trigger_global_refresh: bool,
}

impl TriggerGlobalRefresh {
    const IOCTL_NR: u8 = 0x00;

    ioctl_readwrite!(ioctl, DRM_IOCTL_MAGIC, DRM_COMMAND_BASE + Self::IOCTL_NR, Self);
}

#[repr(C)]
struct OffScreen {
    info1: u64,
    screen_content: *mut u8, // Pointer to a 1404 x 1872 / 2 array. (1404 x 1872, 4bpp)
}

impl OffScreen {
    const IOCTL_NR: u8 = 0x01;

    ioctl_readwrite!(ioctl, DRM_IOCTL_MAGIC, DRM_COMMAND_BASE + Self::IOCTL_NR, Self);
}

// TODO: Find out max size for each buffers!
// IOCTL_NR = 0x02
#[repr(C)]
struct ExtractFBs {
    next_prev: *mut u8,
    hints: *mut u8,
    prelim_targer: *mut u8,
    phase1: *mut u8,
    phase2: *mut u8,
    fnum_inner: *mut u8,
    fnum_outer: *mut u8,
}

impl ExtractFBs {
    const IOCTL_NR: u8 = 0x2;

    //ioctl_readwrite!(ioctl, DRM_IOCTL_MAGIC, DRM_COMMAND_BASE + Self::IOCTL_NR, Self)
}

#[repr(C)]
struct DrmRect {
    x1: i32, // Horizontal starting coordinate (inclusive)
    y1: i32, // Vertical starting coordinate (inclusive)
    x2: i32, // Horizontal stopping coordinate (exclusive)
    y2: i32, // Vertical stopping coordinate (exclusive)
}

#[repr(C)]
struct RectHint {
    hints: u8,
    rect: DrmRect,
}

// IOCTL_NR = 0x03
#[repr(C)]
struct RectHints {
    num_rects: u32, // 20 MAX ???
    set_default_hints: bool,
    rect_hints: [RectHint; 20]
}

// IOCTL_NR = 0x04
#[repr(C)]
struct FastMode {
    fast_mode: u8, // this is just a boolean
}

