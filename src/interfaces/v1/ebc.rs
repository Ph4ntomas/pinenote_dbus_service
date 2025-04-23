use dbus_crossroads::{Context, IfaceBuilder};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{dbus::{PropertyMethodOps, PropertyWrapper}, kernel::{
    self, GenericParameter, Module, PrimitiveParameter, TryFromKernelParam, ModuleParam
}};

use crate::dbus::PropertyWrapper as PWrap;
use kernel::PrimitiveParameter as KPParam;
use kernel::BoolParameter as KBParam;
use kernel::EnumParameter as KEParam;

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

impl crate::dbus::Property for GenericParameter<PixelHints> {
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

pub struct EbcState {
    default_hint: PWrap<GenericParameter<PixelHints>>,
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
                MOps::Disabled, MOps::Enabled("hints")
            ),
            delay_a: PWrap::new(
                module.primitive_parameter("delay_a"),
                "BwThreshold",
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
                "EarlyCancellatioAdditionalFrames",
                true,
                MOps::Disabled,
                MOps::Enabled("num_frames")
            ),
            limit_fb_blit: PWrap::new(
                module.primitive_parameter("limit_fb_blit"),
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
            )
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


        //let auto_reresh_changed_sig = builder.signal::<(), _>("AutoRefreshChanged", ()).msg_fn();
        //let bw_mode_changed_sig = builder.signal::<(), _>("BwModeChanged", ()).msg_fn();
        //let waveform_changed_sig = builder.signal::<(), _>("WaveformChanged", ()).msg_fn();

        //self.auto_refresh.build(builder, |s| &mut s.auto_refresh);
        //self.auto_refresh.setter_hooks.push(Box::new(move |ctx, _| Some(auto_reresh_changed_sig(ctx.path(), &()))));

        //self.bw_mode.build(builder, |s| &mut s.bw_mode);
        //self.bw_mode.setter_hooks.push(Box::new(move |ctx, _| Some(bw_mode_changed_sig(ctx.path(), &()))));

        //self.default_waveform.build(builder, |s| &mut s.default_waveform);
        //self.default_waveform.setter_hooks.push(Box::new(move |ctx, _| Some(waveform_changed_sig(ctx.path(), &()))));
    }
}
