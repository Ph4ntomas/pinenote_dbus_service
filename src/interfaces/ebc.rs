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

#[derive(Default)]
pub struct EbcContext {}

#[derive(Default)]
pub struct ServiceContext{
    pub ebc_context: EbcContext
}

impl EbcContext {
    pub fn build_v1(builder: &mut IfaceBuilder<ServiceContext>) {
        let _auto_refresh_changed = builder.signal::<( ), _>("AutoRefreshChanged", ()).msg_fn();
        let _bw_mode_changed = builder.signal::<( ), _>("BwModeChanged", ()).msg_fn();
        let _bw_dither_invert_changed = builder.signal::<( ), _>("BwDitherInvertChanged", ()).msg_fn();
        let _dclk_select_changed = builder.signal::<( ), _>("DclkSelectChanged", ()).msg_fn();
        let _waveform_changed = builder.signal::<( ), _>("WaveformChanged", ()).msg_fn();
        let _no_off_screen_changed = builder.signal::<( ), _>("NoOffScreenChanged", ()).msg_fn();
        let _requested_quality_or_performance_mode = builder.signal::<(u8, ), _>("RequestedQualityOrPerformance", ("requested_mode", )).msg_fn();
        let _delay_a_changed = builder.signal::<( ), _>("DelayAChanged", ()).msg_fn();
        let _globre_convert_before_changed = builder.signal::<( ), _>("GlobreConvertBeforeChanged", ()).msg_fn();

        //let default_waveform_changed = builder.property("DefaultWaveform").changed_msg_fn();

        builder.property("default_waveform")
            .get(|_, svc_ctx| { svc_ctx.ebc_context.get_default_waveform() })
            .set(|_, svc_ctx, value| { svc_ctx.ebc_context.set_default_waveform(value) })
            .deprecated();

        builder.property("DefaultWaveform")
            .emits_changed_true()
            .get(|_, svc_ctx| { svc_ctx.ebc_context.get_default_waveform() })
            .set(|_, svc_ctx, value| {
                svc_ctx.ebc_context.set_default_waveform(value)
            });

        builder.method("TriggerGlobalRefresh",
            (), // In args
            (), // Out args
            move |_, svc_ctx, ()| { svc_ctx.ebc_context.trigger_global_refresh() }
        );


        let split_area_limit_changed = builder
            .signal::<( ), _>("SplitAreaLimitChanged", ())
            .msg_fn();

        let split_area_limit_cb = builder.property("SplitAreaLimit")
            .emits_changed_true()
            .get(|_, svc_ctx| { svc_ctx.ebc_context.get_split_area_limit() })
            .changed_msg_fn()
        ;

        builder.method("GetSplitAreaLimit",
            (),
            ( "split_limit", ),
            move |_: &mut Context, svc_ctx, ()| {
                svc_ctx.ebc_context.get_split_area_limit().map(|r| {(r,)})
            }
        );

        builder.method("SetSplitAreaLimit",
            ( "split_limit", ),
            (),
            move |ctx, svc_ctx, (split_limit, )| {
                svc_ctx.ebc_context.set_split_area_limit(split_limit);
                let msg = split_area_limit_changed(ctx.path(), &());
                ctx.push_msg(msg);
                split_area_limit_cb(ctx.path(), &split_limit).map(|msg| ctx.push_msg(msg));
                Ok(())
            }
        );
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

    fn trigger_global_refresh(&self) -> Result<(), MethodErr> {
        ebc_ioctl::trigger_global_refresh();
        Ok(())
    }

    fn get_split_area_limit(&self) -> Result<u32, MethodErr> {
        let ret = sys_handler::get_split_area_limit();
        Ok(ret)
    }

    fn set_split_area_limit(&self, limit: u32) {
        sys_handler::set_split_area_limit(limit);
    }
}
