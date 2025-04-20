use dbus::MethodErr;
//use sys_handler;
use dbus_crossroads::{Context, IfaceBuilder};
use num_enum::TryFromPrimitive;

use crate::{ebc_ioctl, sys_handler};

pub struct NulCtx {}

///
/// DRM Waveform
///
#[derive(TryFromPrimitive)]
#[repr(u8)]
enum Waveform {
    Reset,
    A2,
    DU,
    DU4,
    GC16,
    GCC16,
    GL16,
    GLR16,
    GLD16,
    MAX
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum BwMode {
    Gray16,
    BWDither,
    BWTheshold,
    Gray4
}

#[derive(Default)]
pub struct EbcState {}

#[derive(Default)]
pub struct State {
    pub ebc: EbcState
}

impl EbcState {
    pub fn build_v1(builder: &mut IfaceBuilder<State>) {
        let _bw_dither_invert_changed = builder.signal::<( ), _>("BwDitherInvertChanged", ()).msg_fn();
        let _dclk_select_changed = builder.signal::<( ), _>("DclkSelectChanged", ()).msg_fn();
        let _no_off_screen_changed = builder.signal::<( ), _>("NoOffScreenChanged", ()).msg_fn();
        let _requested_quality_or_performance_mode = builder.signal::<(u8, ), _>("RequestedQualityOrPerformance", ("requested_mode", )).msg_fn();
        let _delay_a_changed = builder.signal::<( ), _>("DelayAChanged", ()).msg_fn();
        let _globre_convert_before_changed = builder.signal::<( ), _>("GlobreConvertBeforeChanged", ()).msg_fn();

        let prop_autorefresh_changed = builder.property("AutoRefresh")
            .emits_changed_true()
            .get(|_, state| { state.ebc.get_auto_refresh() })
            .set(|_, state, value| { state.ebc.set_auto_refresh(value) })
            .changed_msg_fn();
        let auto_refresh_changed = builder.signal::<( ), _>("AutoRefreshChanged", ()).msg_fn();

        builder.method("GetAutoRefresh",
            (), ( "state_auto_refresh", ),
            move | _, state, () | {
                state.ebc.get_auto_refresh().map(|v| { (v, ) })
            }).deprecated();

        builder.method("GetAutorefresh",
            (), ( "state_auto_refresh", ),
            move | _, state, () | {
                state.ebc.get_auto_refresh().map(|v| { (v, ) })
            }).deprecated();

        builder.method("SetAutoRefresh",
            ("state_auto_refresh", ), (),
            move | ctx, state, (val,) | {
                let _r = state.ebc.set_auto_refresh(val)?;
                let msg = auto_refresh_changed(ctx.path(), &());
                ctx.push_msg(msg);

                prop_autorefresh_changed(ctx.path(), &val).map(|msg| ctx.push_msg(msg));

                Ok(())
            });

        let prop_bw_mode_changed = builder.property("BwMode")
            .emits_changed_true()
            .get(|_, state| { state.ebc.get_bw_mode() })
            .set(|_, state, value| { state.ebc.set_bw_mode(value) })
            .changed_msg_fn();
        let bw_mode_changed = builder.signal::<(), _>("BwModeChanged", ()).msg_fn();

        builder.method("GetBwMode",
            (), ( "current_mode", ),
            move | _, state, () | {
                state.ebc.get_bw_mode().map(|v| { (v,) })
            }).deprecated();

        builder.method("SetBwMode",
            ("new_mode",), (),
            move | ctx, state, ( value, ) | {
                let r = state.ebc.set_bw_mode(value)?;
                let msg = bw_mode_changed(ctx.path(), &());
                ctx.push_msg(msg);

                prop_bw_mode_changed(ctx.path(), &value).map(|msg| ctx.push_msg(msg));
                Ok(())
            });

        // DefaultWaveform
        let waveform_changed = builder.signal::<(), _>("WaveformChanged", ()).msg_fn();
        let prop_wf_changed = builder.property("DefaultWaveform")
            .emits_changed_true()
            .get(|_, state| { state.ebc.get_default_waveform() })
            .set(|_, state, value| { state.ebc.set_default_waveform(value) })
            .changed_msg_fn();

        builder.property("default_waveform")
            .get(|_, state| { state.ebc.get_default_waveform() })
            .set(|_, state, value| { state.ebc.set_default_waveform(value) })
            .deprecated();

        builder.method("GetDefaultWaveform",
            (), ("current_waveform", ),
            move |_, state, ()| {
                state.ebc.get_default_waveform().map(|v| { (v, ) })
            }
            )
            .deprecated();

        builder.method("SetDefaultWaveform",
            ("waveform", ),
            (),
            move |ctx, state, (waveform, ): (u8, )| {
                state.ebc.set_default_waveform(waveform)?;

                let signal_msg = waveform_changed(ctx.path(), &());
                ctx.push_msg(signal_msg);

                prop_wf_changed(ctx.path(), &waveform).map(|msg| ctx.push_msg(msg));
                Ok(())
            }
        );

        let split_area_limit_cb = builder.property("SplitAreaLimit")
            .emits_changed_true()
            .get(|_, state| { state.ebc.get_split_area_limit() })
            .changed_msg_fn()
        ;
        let split_area_limit_changed = builder.signal::<( ), _>("SplitAreaLimitChanged", ()).msg_fn();

        builder.method("GetSplitAreaLimit",
            (),
            ( "split_limit", ),
            move |_: &mut Context, state, ()| {
                state.ebc.get_split_area_limit().map(|r| {(r,)})
            }
        ).deprecated();

        builder.method("SetSplitAreaLimit",
            ( "split_limit", ),
            (),
            move |ctx, state, (split_limit, )| {
                state.ebc.set_split_area_limit(split_limit);
                let msg = split_area_limit_changed(ctx.path(), &());
                ctx.push_msg(msg);
                split_area_limit_cb(ctx.path(), &split_limit).map(|msg| ctx.push_msg(msg));
                Ok(())
            }
        );

        builder.method("TriggerGlobalRefresh",
            (), // In args
            (), // Out args
            move |_, state, ()| { state.ebc.trigger_global_refresh() }
        );
    }

    fn get_auto_refresh(&self) -> Result<bool, MethodErr> {
        let ret = sys_handler::get_auto_refresh();

        Ok(ret)
    }

    fn set_auto_refresh(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        sys_handler::set_auto_refresh(value);

        Ok(Some(value))
    }

    fn get_bw_dither_invert(&self) -> Result<bool, MethodErr> {
        let ret = sys_handler::get_bw_dither_invert();

        Ok(ret)
    }

    fn set_bw_dither_invert(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        sys_handler::set_bw_dither_invert(value);

        Ok(Some(value))
    }

    fn get_bw_mode(&self) -> Result<u8, MethodErr> {
        let ret = sys_handler::get_bw_mode();

        Ok(ret)
    }

    fn set_bw_mode(&mut self, value: u8) -> Result<Option<u8>, MethodErr> {
        let bw_mode = BwMode::try_from(value).map_err(|_| {
            MethodErr::invalid_arg(&format!("{value} is not a valide mode."))
        })?;

        // TODO: Make this function report error ?
        sys_handler::set_bw_mode(bw_mode as u8);

        Ok(Some(value))
    }

    fn get_dclk_select(&self) -> Result<i16, MethodErr> {
        let r = sys_handler::get_dclk_select();

        Ok(r)
    }

    fn set_dclk_select(&mut self, value: i16) -> Result<Option<i16>, MethodErr> {
        sys_handler::set_dclk_select(value);

        Ok(Some(value))
    }

    fn get_default_waveform(&self) -> Result<u8, MethodErr> {
        Ok(sys_handler::get_default_waveform())
    }

    /// Set the default waveform to be used by the driver
    fn set_default_waveform(&self, value: u8) -> Result<Option<u8>, MethodErr> {
        let waveform = Waveform::try_from(value).map_err(|_| {
            MethodErr::invalid_arg(&format!("{value} is not a valid Wafeform identifier"))
        })?;

        // TODO: Make this function report errors or reimpl.
        sys_handler::set_default_waveform(waveform as u8);

        Ok(Some(value))
    }

    fn get_globre_convert_before(&self) -> Result<bool, MethodErr> {
        let r = sys_handler::get_globre_convert_before();

        Ok(r)
    }

    fn set_globre_convert_before(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        sys_handler::set_globre_convert_before(value);

        Ok(Some(value))
    }

    fn get_no_off_screen(&self) -> Result<bool, MethodErr> {
        Ok(sys_handler::get_no_off_screen())
    }

    fn set_no_off_screen(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        sys_handler::set_no_off_screen(value);

        Ok(Some(value))
    }

    fn get_split_area_limit(&self) -> Result<u32, MethodErr> {
        let ret = sys_handler::get_split_area_limit();
        Ok(ret)
    }

    fn set_split_area_limit(&self, limit: u32) {
        sys_handler::set_split_area_limit(limit);
    }

    fn trigger_global_refresh(&self) -> Result<(), MethodErr> {
        ebc_ioctl::trigger_global_refresh();
        Ok(())
    }
}
