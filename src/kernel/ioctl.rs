use std::{
    fmt::Display, fs::{File, OpenOptions}, os::{fd::AsRawFd, unix::fs::OpenOptionsExt}
};

use drm::rockchip_ebc::RectHints;
use nix::errno::Errno;

pub mod drm;

#[derive(Debug)]
pub enum IoctlError {
    OpenFailed(std::io::Error),
    InvalidOpOrParam,
    MemoryError,
    BadDeviceOrOp,
    Unexpected(String)
}

impl From<nix::errno::Errno> for IoctlError {
    fn from(errno: nix::errno::Errno) -> Self {
        match errno {
            Errno::EFAULT => Self::MemoryError,
            Errno::EINVAL => Self::InvalidOpOrParam,
            Errno::ENOTTY => Self::BadDeviceOrOp,
            e => Self::Unexpected(format!("{e}"))
        }
    }
}

impl Display for IoctlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenFailed(inner) => write!(f, "{}", inner),
            Self::InvalidOpOrParam => write!(f, "Invalid operation or parameter"),
            Self::MemoryError => write!(f, "Inaccessible memory."),
            Self::BadDeviceOrOp => write!(f, "Bad device or operation."),
            Self::Unexpected(str) => write!(f, "Unexpected Error: {str}")
        }
    }
}

pub(crate) fn open_device(path: &str) -> Result<File, IoctlError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .map_err(IoctlError::OpenFailed)
}

#[derive(Default)]
pub struct RockchipEbc;

impl RockchipEbc {
    const DEVICE: &str = "/dev/dri/by-path/platform-fdec0000.ebc-card";

    pub fn refresh_screen(&self) -> Result<(), IoctlError> {
        let file = open_device(Self::DEVICE)?;

        unsafe {
            drm::rockchip_ebc::trigger_global_refresh(file.as_raw_fd())?;
        }

        Ok(())
    }

    pub fn set_hints(&self, rect_hints: RectHints) -> Result<(), IoctlError> {
        let file = open_device(Self::DEVICE)?;

        unsafe {
            rect_hints.set_rect_hints(file.as_raw_fd())?
        }

        Ok(())
    }

    pub fn set_fast_mode(&self, fast: bool) -> Result<(), IoctlError> {
        let file = open_device(Self::DEVICE)?;

        unsafe {
            drm::rockchip_ebc::set_fast_mode(file.as_raw_fd(), fast)?;
        }

        Ok(())
    }
}
