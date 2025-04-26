use std::fmt::Display;

use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{
    kernel::{
        ioctl::drm::rockchip_ebc as drm,
        uapi,
    },
    sys::TryFromKernelParam
};

pub const SCREEN_HEIGHT: usize = 1404;
pub const SCREEN_WIDTH: usize = 1872;
pub const FRAMEBUFFER_SZ_4BPP: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 2;
// FIXME: The current driver has a bug and do not use the height to compute the number of
// pixels.
pub const PIXEL_NUM: usize = SCREEN_WIDTH * SCREEN_WIDTH;

#[derive(TryFromPrimitive, IntoPrimitive, Clone)]
#[repr(i32)]
pub enum DClockSelect {
    Auto = -1,
    MHz200 = 0,
    MHz250 = 1,
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone)]
#[repr(u8)]
pub enum DitheringMethod {
    Bayer = 0,
    BlueNoise16 = 1,
    BlueNoise32 = 2,
}

#[derive(TryFromPrimitive, IntoPrimitive, Clone)]
#[repr(u8)]
pub enum HintBitDepth {
    Y1 = 0,
    Y2 = 1,
    Y4 = 2
}
#[derive(TryFromPrimitive, IntoPrimitive, Clone)]
#[repr(u8)]
pub enum HintConvertMode {
    Threshold = 0,
    Dither = 1
}

#[derive(Clone)]
pub struct PixelHints {
    repr: u8,
}

pub enum PixelHintsError {
    BadBitDepth,
    BadConvertMode,
}

impl Display for PixelHintsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadBitDepth => write!(f, "Bad bit depth"),
            Self::BadConvertMode => write!(f, "Bad convert mode")
        }
    }
}

impl PixelHints {
    const BIT_DEPTH_SHIFT : u8 = 4;
    const BIT_DEPTH_MASK : u8 = 3 << Self::BIT_DEPTH_SHIFT;
    const CONVERT_SHIFT : u8 = 6;
    const CONVERT_MASK : u8 = 1 << Self::CONVERT_SHIFT;
    const REDRAW_SHIFT : u8 = 7;
    const REDRAW_MASK : u8 = 1 << Self::REDRAW_SHIFT;

    pub fn try_from_part(depth: u8, convert_mode: u8, redraw: bool) -> Result<Self, PixelHintsError> {
        let bit_depth = HintBitDepth::try_from_primitive(depth)
            .map_err(|_| PixelHintsError::BadBitDepth)?;
        let convert_mode = HintConvertMode::try_from_primitive(convert_mode)
            .map_err(|_| PixelHintsError::BadConvertMode)?;

        let bit_depth = (bit_depth as u8) << Self::BIT_DEPTH_SHIFT;
        let convert_mode = (convert_mode as u8) << Self::CONVERT_SHIFT;
        let redraw = (redraw as u8) << Self::REDRAW_SHIFT;

        let repr = bit_depth | convert_mode | redraw;
        Ok(Self { repr })
    }

    fn extract_bit_depth(repr: u8) -> u8 {
        (repr & Self::BIT_DEPTH_MASK) >> Self::BIT_DEPTH_SHIFT
    }

    fn extract_convert_mode(repr: u8) -> u8 {
        (repr & Self::CONVERT_MASK) >> Self::CONVERT_SHIFT
    }

    fn extract_redraw(repr: u8) -> bool {
        let redraw = (repr & Self::REDRAW_MASK) >> Self::REDRAW_SHIFT;
        redraw != 0
    }

    pub fn bit_depth(&self) -> HintBitDepth {
        let repr = Self::extract_bit_depth(self.repr);
        HintBitDepth::try_from_primitive(repr).unwrap()
    }

    pub fn convert_mode(&self) -> HintConvertMode {
        let repr = Self::extract_convert_mode(self.repr);
        HintConvertMode::try_from_primitive(repr).unwrap()
    }

    pub fn redraw(&self) -> bool {
        Self::extract_redraw(self.repr)
    }
}

impl From<PixelHints> for u8 {
    fn from(value: PixelHints) -> Self {
        value.repr
    }
}

impl TryFrom<u8> for PixelHints {
    type Error = PixelHintsError; // TODO: Change this;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let bit_depth = Self::extract_bit_depth(value);
        let convert = Self::extract_convert_mode(value);
        let redraw = Self::extract_redraw(value);

        Self::try_from_part(bit_depth, convert, redraw)
    }
}

impl TryFromKernelParam for PixelHints {
    type KRepr = u8;
    type Error = <Self as TryFrom<Self::KRepr>>::Error;

    fn try_from_kernel(value: Self::KRepr) -> Result<Self, Self::Error> {
        Self::try_from(value)
    }
}

pub enum RectError {
    BadPos,
    BadHeight,
    BadWidth
}

impl Display for RectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadPos => write!(f, "Bad position"),
            Self::BadHeight => write!(f, "Bad height"),
            Self::BadWidth => write!(f, "Bad width")
        }
    }
}

pub struct ScreenRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl ScreenRect {
    pub fn try_from_part(x: i32, y: i32, width: i32, height: i32) -> Result<Self, RectError> {
        let scr_width = SCREEN_WIDTH as i32;
        let scr_height = SCREEN_HEIGHT as i32;
        if x < 0 || x > scr_width || y < 0 || y > scr_height {
            Err(RectError::BadPos)
        } else if width < 0 || width > scr_width || width < x {
            Err(RectError::BadWidth)
        } else if height < 0 || height > scr_height || height < y {
            Err(RectError::BadHeight)
        } else {
            Ok(Self { x, y, width, height })
        }
    }
}

impl From<ScreenRect> for uapi::drm::Rect {
    fn from(value: ScreenRect) -> Self {
        Self {
            x1: value.x,
            y1: value.y,
            x2: value.width,
            y2: value.height,
        }
    }
}

pub struct RectHint {
    hints: PixelHints,
    rect: ScreenRect,
}

impl RectHint {
    pub fn new(hints: PixelHints, rect: ScreenRect) -> Self {
        Self { hints, rect }
    }
}

impl From<RectHint> for uapi::rockchip_ebc::RectHint {
    fn from(value: RectHint) -> Self {
        Self {
            hints: value.hints.into(),
            rect: value.rect.into()
        }
    }
}

pub struct RectHints {
    set_default: bool,
    rect_hints: Vec<RectHint>
}

impl RectHints {
    pub fn new(set_default: bool, rect_hints: Vec<RectHint>) -> Self {
        Self {
            set_default,
            rect_hints
        }
    }
}

impl From<RectHints> for drm::RectHints {
    fn from(value: RectHints) -> Self {
        drm::RectHints {
            set_default_hints: value.set_default,
            rect_hints: value.rect_hints.into_iter().map(|r| r.into()).collect()
        }
    }
}
