use dbus::MethodErr;
use dbus_crossroads::{Context, IfaceBuilder};

use crate::{dbus::PropertyMethodOps, kernel::{
    self, ioctl::IoctlError,
    module::rockchip_ebc::{
        DitheringMethod, PixelHintsError,
        RectHint, RectHints, ScreenRect},
    Module, ModuleParam
}};

use crate::dbus::PropertyWrapper as PWrap;
use kernel::PrimitiveParameter as KPParam;
use kernel::BoolParameter as KBParam;
use kernel::EnumParameter as KEParam;
use kernel::GenericParameter as KGParam;
use kernel::module::rockchip_ebc::PixelHints;

impl crate::dbus::Property for KGParam<PixelHints> {
    type DBusRepr = (u8, u8, bool);

    fn get(&self) -> Result<Self::DBusRepr, dbus::MethodErr> {
        let hint = self.read().map_err(dbus::MethodErr::from)?;

        Ok((hint.bit_depth().into(), hint.convert_mode().into(), hint.redraw()))
    }

    fn set(&mut self, (bit_depth, convert_mode, redraw): Self::DBusRepr) -> Result<Self::DBusRepr, dbus::MethodErr> {
        let hint = PixelHints::try_from_part(bit_depth, convert_mode, redraw)
            .map_err(|e| match e {
                    PixelHintsError::BadBitDepth => MethodErr::invalid_arg(&format!(
                        "{bit_depth} is not a valid bit depth."
                    )),
                    PixelHintsError::BadConvertMode => MethodErr::invalid_arg(&format!(
                        "{convert_mode} is not a valid conversion mode.")),
                })?;
        self.write(hint).map_err(dbus::MethodErr::from)?;

        Ok((bit_depth, convert_mode, redraw))
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
            |ctx, s, hints| s.set_default_hints(ctx, hints)
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

    fn do_set_default_hint(&self, hints: RectHints) -> Result<(), MethodErr> {
        match self.ebc_ioctl.set_hints(hints.into()) {
            Err(e) => {
                eprintln!("{e}");
                Err(MethodErr::failed("Internal Error"))
            },
            _ => Ok(())
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn set_hints(&self, _ctx: &mut Context, default: bool, hints: Vec<((u8, u8, bool), (i32, i32, i32, i32))>) -> Result<(), MethodErr> {
        let hints: Vec<RectHint> = hints.into_iter()
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

            Ok(RectHint::new(hint, r))
        }).collect::<Result<Vec<_>, MethodErr>>()?;

        let hints = RectHints::new(default, hints);

        self.do_set_default_hint(hints)
    }

    pub fn set_default_hints(&mut self, ctx: &mut Context, hints: (u8, u8, bool)) -> Result<(), MethodErr> {
        self.default_hint.setter(ctx, hints)?;

        let hints = RectHints::new(true, Vec::new());

        self.do_set_default_hint(hints)
    }

    pub fn set_fast_mode(&mut self, _ctx: &mut Context, fast: bool) -> Result<(), MethodErr> {
        self.ebc_ioctl.set_fast_mode(fast).map_err(Self::ioctl_internal_error)
    }
}

impl Default for EbcState {
    fn default() -> Self {
        Self::new()
    }
}
