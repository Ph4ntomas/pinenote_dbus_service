const IOCTL_MAGIC: u8 = 'd' as u8;
const COMMAND_BASE: u8 = 0x40;

pub mod rockchip_ebc {
    use crate::kernel::ioctl::IoctlError;

    pub const SCREEN_WIDTH: usize = 1872;
    pub const SCREEN_HEIGHT: usize = 1404;
    pub const FRAMEBUFFER_SZ_4BPP: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 2;
    // FIXME: The current driver has a bug and do not use the height to compute the number of
    // pixels.
    pub const PIXEL_NUM: usize = SCREEN_WIDTH * SCREEN_WIDTH;

    pub mod details {
        use nix::ioctl_readwrite;

        use crate::kernel::ioctl::drm::{ IOCTL_MAGIC, COMMAND_BASE };

        pub fn get_phase_size(direct_mode: bool) -> usize {
            if direct_mode {
                super::SCREEN_WIDTH / 4 * super::SCREEN_HEIGHT
            } else {
                super::SCREEN_WIDTH * super::SCREEN_HEIGHT
            }
        }

        #[repr(C)]
        pub struct GlobalRefreshPayload {
            pub trigger: bool,
        }

        const GLOBAL_REFRESH_IOCTL_NR: u8 = 0x00;
        ioctl_readwrite!(global_refresh_iowr, IOCTL_MAGIC, COMMAND_BASE + GLOBAL_REFRESH_IOCTL_NR, GlobalRefreshPayload);

        #[repr(C)]
        pub struct OffScreenContentPayload {
            pub info1: u8,
            pub content: *mut u8, // Pointer to a 1404 x 1872 / 2 array. (1404 x 1872, 4bpp)
        }
        const OFF_SCREEN_CONTENT_IOCTL_NR: u8 = 0x01;
        ioctl_readwrite!(off_screen_iowr, IOCTL_MAGIC, COMMAND_BASE + OFF_SCREEN_CONTENT_IOCTL_NR, OffScreenContentPayload);

        #[repr(C)]
        pub struct ExtractFBsPayload {
            pub next_prev: *mut u8,
            pub hints: *mut u8,
            pub prelim_target: *mut u8,
            pub phase1: *mut u8,
            pub phase2: *mut u8,
            pub fnum_inner: *mut u8,
            pub fnum_outer: *mut u8,
        }

        const EXTRACT_FBS_IOCTL_NR: u8 = 0x2;
        ioctl_readwrite!(extract_fbs_iowr, IOCTL_MAGIC, COMMAND_BASE + EXTRACT_FBS_IOCTL_NR, ExtractFBsPayload);

        // IOCTL_NR = 0x03
        #[repr(C)]
        pub struct RectHintsPayload {
            pub num_rects: u32, // 20 MAX
            pub set_default_hints: bool,
            pub rect_hints: [super::RectHint; 20]
        }

        const RECT_HINTS_IOCTL_NR: u8 = 0x03;
        ioctl_readwrite!(rect_hints_iowr, IOCTL_MAGIC, COMMAND_BASE + RECT_HINTS_IOCTL_NR, RectHintsPayload);

        // IOCTL_NR = 0x04
        #[repr(C)]
        pub struct FastModePayload {
            pub fast_mode: u8, // this is just a boolean
        }

        const FAST_MODE_IOCTL_NR: u8 = 0x04;
        ioctl_readwrite!(fast_mode_iowr, IOCTL_MAGIC, COMMAND_BASE + FAST_MODE_IOCTL_NR, FastModePayload);

    }

    pub unsafe fn trigger_global_refresh(raw_fd: std::os::fd::RawFd) -> Result<(), IoctlError> {
        let mut payload = details::GlobalRefreshPayload { trigger: true };
        details::global_refresh_iowr(raw_fd, &mut payload).map_err(IoctlError::from)?;
        Ok(())
    }


    pub unsafe fn set_off_screen(raw_fd: std::os::fd::RawFd, content: &mut [u8; FRAMEBUFFER_SZ_4BPP]) -> Result<(), nix::errno::Errno> {
        let mut payload = details::OffScreenContentPayload {
            info1: 0,
            content: content.as_mut_ptr(),
        };

        details::off_screen_iowr(raw_fd, &mut payload)?;
        Ok(())
    }

    // When in direct mode (the default), phase1 and phase2 only use a quarter of the size.
    pub struct ExtractFBs {
        pub next_prev: Vec<u8>,
        pub hints: Vec<u8>,
        pub prelim_target: Vec<u8>,
        pub phase1: Vec<u8>,
        pub phase2: Vec<u8>,
    }

    impl ExtractFBs {
        pub unsafe fn extract_fbs(raw_fd: std::os::fd::RawFd) -> Result<ExtractFBs, nix::errno::Errno> {
            let phase_capacity = details::get_phase_size(true);

            let mut next_prev: Vec<u8> = vec![0; PIXEL_NUM];
            let mut hints: Vec<u8> = vec![0; PIXEL_NUM];
            let mut prelim_target: Vec<u8> = vec![0; PIXEL_NUM];
            let mut phase1: Vec<u8> = vec![0; phase_capacity];
            let mut phase2: Vec<u8> = vec![0; phase_capacity];
            let mut fnum_inner: Vec<u8> = Vec::new();
            let mut fnum_outer: Vec<u8> = Vec::new();

            let mut payload = details::ExtractFBsPayload {
                next_prev: next_prev.as_mut_ptr(),
                hints: hints.as_mut_ptr(),
                prelim_target: prelim_target.as_mut_ptr(),
                phase1: phase1.as_mut_ptr(),
                phase2: phase2.as_mut_ptr(),
                fnum_inner: fnum_inner.as_mut_ptr(),
                fnum_outer: fnum_outer.as_mut_ptr()
            };

            details::extract_fbs_iowr(raw_fd, &mut payload)?;

            Ok(Self {
                next_prev, hints, prelim_target,
                phase1, phase2,
            })
        }
    }

    #[derive(Default, Clone)]
    #[repr(C)]
    pub struct DrmRect {
        pub x1: i32, // Horizontal starting coordinate (inclusive)
        pub y1: i32, // Vertical starting coordinate (inclusive)
        pub x2: i32, // Horizontal stopping coordinate (exclusive)
        pub y2: i32, // Vertical stopping coordinate (exclusive)
    }

    #[derive(Default, Clone)]
    #[repr(C)]
    pub struct RectHint {
        pub hints: u8,
        pub rect: DrmRect,
    }

    pub struct RectHints {
        pub set_default_hints: bool,
        pub rect_hints: Vec<RectHint>
    }

    impl RectHints {
        const MAX_HINT: usize = 20;

        pub unsafe fn set_rect_hints(self, raw_fd: std::os::fd::RawFd) -> Result<(), nix::errno::Errno> {
            let mut rect_hints: [RectHint; Self::MAX_HINT] = Default::default();

            for (i, hint) in self.rect_hints.iter().enumerate().take(20) {
                rect_hints[i] = hint.clone()
            }

            let mut payload = details::RectHintsPayload {
                num_rects: usize::min(Self::MAX_HINT, self.rect_hints.len()) as u32,
                set_default_hints: self.set_default_hints,
                rect_hints,
            };

            details::rect_hints_iowr(raw_fd, &mut payload)?;

            Ok(())
        }
    }


    pub unsafe fn set_fast_mode(raw_fd: std::os::fd::RawFd, fast: bool) -> Result<(), nix::errno::Errno> {
        let mut payload = details::FastModePayload {
            fast_mode: fast as u8,
        };

        details::fast_mode_iowr(raw_fd, &mut payload)?;

        Ok(())
    }
}
