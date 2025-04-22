use dbus_crossroads::{Context, IfaceBuilder};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{dbus::{PropertyMethodOps, PropertyWrapper}, kernel::{
    self, Module, PrimitiveParameter
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

pub struct EbcState {
    //default_hint:
    //direct_mode:
    //delay_a: PWrap<KPParam<i32>>,
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
            //default_hint:
            //direct_mode:
            bw_threshold: PWrap::new(
                module.primitive_parameter("bw_threshold"),
                "BwThreshold",
                true,
                MOps::Disabled, MOps::Enabled("threshold")
            ),
            delay_a: PWrap::new(
                module.primitive_parameter("delay_a"),
                "BwThreshold",
                true,
                MOps::Disabled, MOps::Enabled("threshold")
            ),
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
