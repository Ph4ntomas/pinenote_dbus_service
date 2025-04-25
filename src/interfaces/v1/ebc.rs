use std::fmt::Display;

use dbus::MethodErr;
use dbus_crossroads::{Context, IfaceBuilder};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{dbus::PropertyMethodOps, kernel::{
    self, ioctl::{drm::rockchip_ebc, IoctlError}, Module, ModuleParam, TryFromKernelParam
}};

use crate::dbus::PropertyWrapper as PWrap;
use kernel::PrimitiveParameter as KPParam;
use kernel::BoolParameter as KBParam;
use kernel::EnumParameter as KEParam;
use kernel::GenericParameter as KGParam;

//FIXME: Move this to some HW specific module
const SCREEN_HEIGHT: usize = 1404;
const SCREEN_WIDTH: usize = 1872;

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
const HINT_BIT_DEPTH_SHIFT : u8 = 4;
const HINT_BIT_DEPTH_MASK : u8 = 3 << HINT_BIT_DEPTH_SHIFT;

#[derive(TryFromPrimitive, IntoPrimitive, Clone)]
#[repr(u8)]
pub enum HintConvertMode {
    Threshold = 0,
    Dither = 1
}
const HINT_CONVERT_SHIFT : u8 = 6;
const HINT_CONVERT_MASK : u8 = 1 << HINT_CONVERT_SHIFT;

#[derive(Clone)]
struct PixelHints {
    bit_depth: HintBitDepth,
    convert_mode: HintConvertMode,
    redraw: bool
}

const HINT_REDRAW_SHIFT : u8 = 7;
const HINT_REDRAW_MASK : u8 = 1 << 7;

enum PixelHintsError {
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
    fn try_from_part(depth: u8, convert_mode: u8, redraw: bool) -> Result<Self, PixelHintsError> {
        let bit_depth = HintBitDepth::try_from_primitive(depth)
            .map_err(|_| PixelHintsError::BadBitDepth)?;
        let convert_mode = HintConvertMode::try_from_primitive(convert_mode)
            .map_err(|_| PixelHintsError::BadConvertMode)?;

        Ok(Self { bit_depth, convert_mode, redraw })
    }
}

impl From<PixelHints> for u8 {
    fn from(value: PixelHints) -> Self {
        let bit_depth = (value.bit_depth as u8) << HINT_BIT_DEPTH_SHIFT;
        let convert = (value.convert_mode as u8) << HINT_CONVERT_SHIFT;
        let redraw = (value.redraw as u8) << HINT_REDRAW_SHIFT;

        bit_depth | convert | redraw
    }
}

impl TryFrom<u8> for PixelHints {
    type Error = kernel::Error; // TODO: Change this;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let bit_depth = HintBitDepth::try_from_primitive((value & HINT_BIT_DEPTH_MASK) >> HINT_BIT_DEPTH_SHIFT)
            .map_err(|_| Self::Error::ConvertError)?;
        let convert_mode = HintConvertMode::try_from_primitive((value & HINT_CONVERT_MASK) >> HINT_CONVERT_SHIFT)
            .map_err(|_| Self::Error::ConvertError)?;
        let redraw = ((value & HINT_REDRAW_MASK) >> HINT_REDRAW_SHIFT) == 1;

        Ok(Self {
            bit_depth,
            convert_mode,
            redraw
        })
    }
}

impl TryFromKernelParam for PixelHints {
    type KRepr = u8;
    type Error = <Self as TryFrom<Self::KRepr>>::Error;

    fn try_from_kernel(value: Self::KRepr) -> Result<Self, Self::Error> {
        Self::try_from(value)
    }
}

impl crate::dbus::Property for KGParam<PixelHints> {
    type DBusRepr = (u8, u8, bool);

    fn get(&self) -> Result<Self::DBusRepr, dbus::MethodErr> {
        let hint = self.read().map_err(dbus::MethodErr::from)?;

        Ok((hint.bit_depth.into(), hint.convert_mode.into(), hint.redraw))
    }

    fn set(&mut self, value: Self::DBusRepr) -> Result<Self::DBusRepr, dbus::MethodErr> {
        let cl = value.clone();
        let bit_depth = HintBitDepth::try_from_primitive(value.0)
            .map_err(|_| dbus::MethodErr::invalid_arg(
                &format!("{} is not a valid bit depth hint", value.0))
            )?;
        let convert_mode = HintConvertMode::try_from_primitive(value.1)
            .map_err(|_| dbus::MethodErr::invalid_arg(&format!("{} is not a valid conversion mode", value.1)))?;
        let redraw = value.2;

        let hint = PixelHints {
            bit_depth,
            convert_mode,
            redraw
        };

        self.write(hint).map_err(dbus::MethodErr::from)?;

        Ok(cl)
    }
}

enum RectError {
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

// Move this to HW support module
struct ScreenRect {
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

impl Into<rockchip_ebc::DrmRect> for ScreenRect {
    fn into(self) -> rockchip_ebc::DrmRect {
        rockchip_ebc::DrmRect {
            x1: self.x,
            y1: self.y,
            x2: self.width,
            y2: self.height
        }
    }
}

struct RectHint {
    hints: PixelHints,
    rect: ScreenRect,
}

impl RectHint {
    pub fn new(hints: PixelHints, rect: ScreenRect) -> Self {
        Self { hints, rect }
    }
}

impl Into<rockchip_ebc::RectHint> for RectHint {
    fn into(self) -> rockchip_ebc::RectHint {
        rockchip_ebc::RectHint {
            hints: self.hints.into(),
            rect: self.rect.into()
        }
    }
}

pub struct EbcState {
    default_hint: PWrap<KGParam<PixelHints>>,
    //direct_mode:
    bw_threshold: PWrap<KPParam<i32>>,
    delay_a: PWrap<KPParam<i32>>,
    dithering_method: PWrap<KEParam<DitheringMethod>>,
    early_cancellation_addition: PWrap<KPParam<i32>>,
    limit_fb_blit: PWrap<KPParam<i32>>,
    no_off_screen: PWrap<KBParam>,
    redraw_delay: PWrap<KPParam<i32>>,
    refresh_thread_wait_idle: PWrap<KPParam<i32>>,
    shrink_vwindow: PWrap<KBParam>,

    ebc_ioctl: kernel::ioctl::RockchipEbc,
}

impl EbcState {
    pub fn new() -> Self {
        let module = Module::new("rockchip_ebc");
        type MOps = PropertyMethodOps;

        Self {
            bw_threshold: PWrap::new(
                module.primitive_parameter("bw_threshold"),
                "BwThreshold",
                true,
                MOps::Disabled, MOps::Enabled("threshold")
            ),
            default_hint: PWrap::new(
                module.generic_parameter("default_hint"),
                "DefaultHint",
                true,
                MOps::Disabled, MOps::Disabled
            ),
            delay_a: PWrap::new(
                module.primitive_parameter("delay_a"),
                "DelayA",
                true,
                MOps::Disabled, MOps::Enabled("threshold")
            ),
            //direct_mode:
            dithering_method: PWrap::new(
                module.enum_parameter("dithering_method"),
                "DitheringMethod",
                true,
                MOps::Disabled, MOps::Enabled("method")
            ),
            early_cancellation_addition: PWrap::new(
                module.primitive_parameter("early_cancellation_addition"),
                "EarlyCancellationAdditionalFrames",
                true,
                MOps::Disabled,
                MOps::Enabled("num_frames")
            ),
            limit_fb_blit: PWrap::new(
                module.primitive_parameter("limit_fb_blits"),
                "LimitFbBlit",
                true, MOps::Disabled, MOps::Enabled("num_blits")
            ),
            no_off_screen: PWrap::new(
                module.bool_parameter("no_off_screen"),
                "NoOffScreen",
                true, MOps::Disabled, MOps::Enabled("no_off_screen")
            ),
            redraw_delay: PWrap::new(
                module.primitive_parameter("redraw_delay"),
                "RedrawDelay",
                true,
                MOps::Disabled,
                MOps::Enabled("redraw_delay")
            ),
            refresh_thread_wait_idle: PWrap::new(
                module.primitive_parameter("refresh_thread_wait_idle"),
                "RefreshThreadWaitIdle",
                true,
                MOps::Disabled,
                MOps::Enabled("wait_time")
            ),
            shrink_vwindow: PWrap::new(
                module.bool_parameter("shrink_virtual_window"),
                "ShrinkVirtualWindow", true,
                MOps::Disabled, MOps::Enabled("shrink_virtual_window")
            ),

            ebc_ioctl: Default::default(),
        }
    }

    pub fn build(&mut self, builder: &mut IfaceBuilder<Self>) {
        self.bw_threshold.build(builder, |s| &mut s.bw_threshold);
        self.default_hint.build(builder, |s| &mut s.default_hint);
        self.delay_a.build(builder, |s| &mut s.delay_a);
        self.dithering_method.build(builder, |s| &mut s.dithering_method);
        self.early_cancellation_addition.build(builder, |s| &mut s.early_cancellation_addition);
        self.limit_fb_blit.build(builder, |s| &mut s.limit_fb_blit);
        self.no_off_screen.build(builder, |s| &mut s.no_off_screen);
        self.redraw_delay.build(builder, |s| &mut s.redraw_delay);
        self.refresh_thread_wait_idle.build(builder, |s| &mut s.refresh_thread_wait_idle);
        self.shrink_vwindow.build(builder, |s| &mut s.shrink_vwindow);

        builder.method("SetDefaultHints",
            ( "bit_depth", "convert_mode", "redraw" ), (),
            |ctx, s, v| s.default_hint.setter(ctx, v)
        );

        builder.method("RefreshScreen",
            (), (), |ctx, s, ()| s.refresh_screen(ctx)
        );

        builder.method("SetHints",
            ( "set_default", "hints"), (), | ctx, s, (default, hints) |
            s.set_hints(ctx, default, hints)
        );

        builder.method("SetFastMode",
            ( "fast", ), (), | ctx, s, (fast, ) | s.set_fast_mode(ctx, fast)
        );
    }

    pub fn refresh_screen(&self, _ctx: &mut Context) -> Result<(), MethodErr> {
        let res = self.ebc_ioctl.refresh_screen();

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Ebc1: refresh_screen: {}", e);
                Err(MethodErr::failed("Internal Error"))
            }
        }
    }

    fn ioctl_internal_error(error: IoctlError) -> MethodErr {
        eprintln!("{error}");
        MethodErr::failed("Internal Error")
    }

    fn do_set_default_hint(&self, hints: rockchip_ebc::RectHints) -> Result<(), MethodErr> {
        match self.ebc_ioctl.set_hints(hints) {
            Err(e) => {
                eprintln!("{e}");
                Err(MethodErr::failed("Internal Error"))
            },
            _ => Ok(())
        }
    }

    pub fn set_hints(&self, _ctx: &mut Context, default: bool, hints: Vec<((u8, u8, bool), (i32, i32, i32, i32))>) -> Result<(), MethodErr> {
        let hints: Vec<rockchip_ebc::RectHint> = hints.into_iter()
            .enumerate()
            .map(|(i, ((depth, convert, redraw), (x, y, width, height)))| {
            let hint = PixelHints::try_from_part(depth, convert, redraw)
                .map_err(|e| {
                    dbus::MethodErr::invalid_arg(&format!("Rect {i}: {e}"))
                })?;
            let r = ScreenRect::try_from_part(x, y, width, height)
                .map_err(|e| {
                    dbus::MethodErr::invalid_arg(&format!("Rect {i}: Bad rectangle({x}, {y}, {width} {height}): {e}"))
                })?;

            Ok(RectHint::new(hint, r).into())
        }).collect::<Result<Vec<_>, MethodErr>>()?;

        let hints = rockchip_ebc::RectHints {
            set_default_hints: default,
            rect_hints: hints
        };

        self.do_set_default_hint(hints)
    }

    pub fn set_default_hints(&mut self, ctx: &mut Context, hints: (u8, u8, bool)) -> Result<(), MethodErr> {
        self.default_hint.setter(ctx, hints)?;

        let hints = rockchip_ebc::RectHints {
            set_default_hints: true,
            rect_hints: Vec::new()
        };

        self.do_set_default_hint(hints)
    }

    pub fn set_fast_mode(&mut self, _ctx: &mut Context, fast: bool) -> Result<(), MethodErr> {
        self.ebc_ioctl.set_fast_mode(fast).map_err(Self::ioctl_internal_error)
    }
}
