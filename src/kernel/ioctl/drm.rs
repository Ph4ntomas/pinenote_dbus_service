const IOCTL_MAGIC: u8 = 'd' as u8;
const COMMAND_BASE: u8 = 0x40;

pub mod rockchip_ebc {
    use crate::kernel::module::rockchip_ebc::{FRAMEBUFFER_SZ_4BPP, PIXEL_NUM};

    pub(super) mod details {
        use nix::ioctl_readwrite;
        use crate::kernel::{
            ioctl::drm::{ COMMAND_BASE, IOCTL_MAGIC },
            module::rockchip_ebc::{SCREEN_HEIGHT, SCREEN_WIDTH},
        };
        pub use crate::kernel::uapi::rockchip_ebc::{
            ExtractFBs, FastMode,
            GlobalRefresh, OffScreenContent,
            RectHint, RectHints
        };

        pub fn get_phase_size(direct_mode: bool) -> usize {
            if direct_mode {
                SCREEN_WIDTH/ 4 * SCREEN_HEIGHT
            } else {
                SCREEN_WIDTH * SCREEN_HEIGHT
            }
        }

        const GLOBAL_REFRESH_IOCTL_NR: u8 = 0x00;
        ioctl_readwrite!(global_refresh_iowr, IOCTL_MAGIC, COMMAND_BASE + GLOBAL_REFRESH_IOCTL_NR, GlobalRefresh);

        const OFF_SCREEN_CONTENT_IOCTL_NR: u8 = 0x01;
        ioctl_readwrite!(off_screen_iowr, IOCTL_MAGIC, COMMAND_BASE + OFF_SCREEN_CONTENT_IOCTL_NR, OffScreenContent);

        const EXTRACT_FBS_IOCTL_NR: u8 = 0x2;
        ioctl_readwrite!(extract_fbs_iowr, IOCTL_MAGIC, COMMAND_BASE + EXTRACT_FBS_IOCTL_NR, ExtractFBs);

        // IOCTL_NR = 0x03
        const RECT_HINTS_IOCTL_NR: u8 = 0x03;
        ioctl_readwrite!(rect_hints_iowr, IOCTL_MAGIC, COMMAND_BASE + RECT_HINTS_IOCTL_NR, RectHints);

        // IOCTL_NR = 0x04
        const FAST_MODE_IOCTL_NR: u8 = 0x04;
        ioctl_readwrite!(fast_mode_iowr, IOCTL_MAGIC, COMMAND_BASE + FAST_MODE_IOCTL_NR, FastMode);

    }

    pub unsafe fn trigger_global_refresh(raw_fd: std::os::fd::RawFd) -> Result<(), nix::errno::Errno> {
        let mut payload = details::GlobalRefresh{ trigger: true };
        details::global_refresh_iowr(raw_fd, &mut payload)?;
        Ok(())
    }


    pub unsafe fn set_off_screen(raw_fd: std::os::fd::RawFd, content: &mut [u8; FRAMEBUFFER_SZ_4BPP]) -> Result<(), nix::errno::Errno> {
        let mut payload = details::OffScreenContent {
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

            let mut payload = details::ExtractFBs {
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

    pub struct RectHints {
        pub set_default_hints: bool,
        pub rect_hints: Vec<details::RectHint>
    }

    impl RectHints {
        const MAX_RECT: usize = details::RectHints::MAX_RECT;

        pub unsafe fn set_rect_hints(self, raw_fd: std::os::fd::RawFd) -> Result<(), nix::errno::Errno> {
            let mut rect_hints: [details::RectHint; Self::MAX_RECT] = Default::default();

            for (i, hint) in self.rect_hints.iter().enumerate().take(20) {
                rect_hints[i] = hint.clone()
            }

            let mut payload = details::RectHints {
                num_rects: usize::min(Self::MAX_RECT, self.rect_hints.len()) as u32,
                set_default_hints: self.set_default_hints,
                rect_hints,
            };

            details::rect_hints_iowr(raw_fd, &mut payload)?;

            Ok(())
        }
    }


    pub unsafe fn set_fast_mode(raw_fd: std::os::fd::RawFd, fast: bool) -> Result<(), nix::errno::Errno> {
        let mut payload = details::FastMode {
            fast_mode: fast as u8,
        };

        details::fast_mode_iowr(raw_fd, &mut payload)?;

        Ok(())
    }
}
