#[derive(Default, Clone)]
#[repr(C)]
pub struct Rect {
    pub x1: i32, // Horizontal starting coordinate (inclusive)
    pub y1: i32, // Vertical starting coordinate (inclusive)
    pub x2: i32, // Horizontal stopping coordinate (exclusive)
    pub y2: i32, // Vertical stopping coordinate (exclusive)
}

pub mod rockchip_ebc {
    #[repr(C)]
    pub struct GlobalRefresh {
        pub trigger: bool,
    }

    #[repr(C)]
    pub struct OffScreenContent {
        pub info1: u8,
        pub content: *mut u8, // Pointer to a 1404 x 1872 / 2 array. (1404 x 1872, 4bpp)
    }

    #[repr(C)]
    pub struct ExtractFBs {
        pub next_prev: *mut u8,
        pub hints: *mut u8,
        pub prelim_target: *mut u8,
        pub phase1: *mut u8,
        pub phase2: *mut u8,
        pub fnum_inner: *mut u8,
        pub fnum_outer: *mut u8,
    }

    #[derive(Default, Clone)]
    #[repr(C)]
    pub struct RectHint {
        pub hints: u8,
        pub padding: [u8; 7],
        pub rect: super::Rect,
    }

    #[repr(C)]
    pub struct RectHints {
        pub set_default_hints: u8,
        pub default_hints: u8,
        pub padding: [u8; 2],
        pub num_rects: u32,
        pub rect_hints: *const RectHint
    }

    #[repr(C)]
    pub struct FastMode {
        pub fast_mode: u8, // this is just a boolean
    }
}
