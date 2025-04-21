use std::{fmt::Display, str::FromStr};

use dbus::MethodErr;
//use sys_handler;
use dbus_crossroads::{Context, IfaceBuilder};
use num_enum::TryFromPrimitive;

use crate::kernel::{
        self,
        enums::{BwMode, Waveform},
        BoolParameter,
        EnumParameter,
        Module,
        ModuleParam,
        PrimitiveParameter
    };

pub struct State {
    pub ebc: EbcState
}

impl From<kernel::Error> for MethodErr {
    fn from(value: kernel::Error) -> Self {
        match value {
            kernel::Error::IoError(_) => MethodErr::failed("Internal Error"),
            kernel::Error::ParseError => MethodErr::failed("Internal Error"),
            kernel::Error::ConvertError => MethodErr::invalid_arg("Bad parameter type")
        }
    }
}

trait DbusProperty {
    type DBusRepr;

    fn get(&self) -> Result<Self::DBusRepr, MethodErr>;
    fn set(&mut self, value: Self::DBusRepr) -> Result<Self::DBusRepr, MethodErr>;
}

impl DbusProperty for BoolParameter {
    type DBusRepr = <BoolParameter as ModuleParam<bool>>::Repr;

    fn get(&self) -> Result<Self::DBusRepr, MethodErr> {
        self.read().map_err(MethodErr::from)
    }

    fn set(&mut self, value: bool) -> Result<bool, MethodErr> {
        self.write(value).map_err(MethodErr::from)
    }
}

impl<T> DbusProperty for PrimitiveParameter<T> where
T: FromStr + Display
{
    type DBusRepr = <PrimitiveParameter<T> as ModuleParam<T>>::Repr;

    fn get(&self) -> Result<Self::DBusRepr, MethodErr> {
        self.read().map_err(MethodErr::from)
    }

    fn set(&mut self, value: Self::DBusRepr) -> Result<Self::DBusRepr, MethodErr> {
        self.write(value).map_err(MethodErr::from)
    }
}

impl<T> DbusProperty for EnumParameter<T> where
T: TryFromPrimitive + Into<T::Primitive> + Clone,
T::Primitive: FromStr + Display
{
    type DBusRepr = T::Primitive;

    fn get(&self) -> Result<Self::DBusRepr, MethodErr> {
        self.read()
            .map(T::into)
            .map_err(MethodErr::from)
    }

    fn set(&mut self, value: Self::DBusRepr) -> Result<Self::DBusRepr, MethodErr> {
        let val = T::try_from_primitive(value).map_err(|_|
            MethodErr::invalid_arg(&format!("Bad parameter")))?;

        self.write(val).map(T::into).map_err(MethodErr::from)
    }
}

//trait DbusPropertyWrapper<Prop: DbusProperty> {
    //fn property(&self) -> &T;
    //fn property_mut(&mut self) -> &mut T;
    //fn emit_changed(&self) -> bool;
    //fn setter_hooks(&self) -> &Vec<Box<dyn Fn(&mut Context, &Prop::DBusRepr) -> Option<dbus::Message> + Send>>;

    //fn get(&self) -> Result<T::DBusRepr, MethodErr> {
        //self.property().get_()
    //}

    //fn set(&mut self, value: T::DBusRepr) -> Result<Option<T::DBusRepr>, MethodErr> {
        //self.property_mut()
            //.set_(value)
            //.map_err(MethodErr::from)
            //.map(|v| if self.emit_changed() { Some(v) } else { None })
    //}

    //fn getter(&self) -> Result<(T::DBusRepr, ), MethodErr> {
        //self.get().map(|v| (v,))
    //}

    //fn setter(&mut self, ctx: &mut Context, value: T::DBusRepr) -> Result<(), MethodErr> {
        //let value = self.property_mut().set(value)?;

        //let msgs : Vec<_>  = self.setter_hooks().iter()
            //.flat_map(|f| f(ctx, &value)).collect();

        //msgs.into_iter().for_each(|msg| ctx.push_msg(msg));

        //Ok(())
    //}
//}

pub enum PropertyMethodOps {
    Enabled(&'static str),
    Deprecated(&'static str),
    Disabled
}

struct PropertyWrapper<T>
    where T: DbusProperty
{
    name: String,
    property: T,
    emit_changed: bool,
    getter_ops: PropertyMethodOps,
    setter_ops: PropertyMethodOps,
    setter_hooks: Vec<Box<dyn Fn(&mut Context, &T::DBusRepr) -> Option<dbus::Message> + Send>>
}

//impl<T: DbusProperty> DbusPropertyWrapper<T> for PropertyWrapper<T> {
    //fn property(&self) -> &T { &self.property }
    //fn property_mut(&mut self) -> &mut T { &mut self.property }
    //fn emit_changed(&self) -> bool { self.emit_changed }
    //fn setter_hooks(&self) -> &Vec<Box<dyn Fn(&mut Context, &<T as DbusProperty>::DBusRepr) -> Option<dbus::Message> + Send>> {
        //&self.setter_hooks
    //}
//}

//trait DbusPropertyBuilder<Prop> : DbusPropertyWrapper<Prop> where
//Prop: DbusProperty,
//Prop::DBusRepr: dbus::arg::Arg + dbus::arg::RefArg + dbus::arg::Append + for <'x> dbus::arg::Get<'x> + 'static
//{
    //fn name(&self) -> String;

    //fn build<State, T>(&mut self, builder: &mut IfaceBuilder<State>, access: T) where
        //T: Fn(&mut State) -> &mut Self + Clone + Send + 'static,
        //State: Send
    //{
        //let get_access = access.clone();
        //let set_access = access.clone();

        //let prop = builder.property(&self.name())
            //.get(move |_, state| get_access(state).get())
            //.set(move |_, state, value| set_access(state).set(value))
        //;

        ////if self.emit_changed() {
            ////let m = prop
                ////.emits_changed_true()
                ////.changed_msg_fn();

            ////self.setter_hooks().push(Box::new(move |ctx: &mut Context, val| {
                ////m(ctx.path(), val)
            ////}));
        ////} else {
            ////prop.emits_changed_false();
        ////}

        ////match &self.getter_ops {
            ////PropertyMethodOps::Enabled(arg_name) | PropertyMethodOps::Deprecated(arg_name) => {
                ////let get_access = access.clone();

                ////let method = builder.method(format!("Get{}", &self.name),
                    ////(), (*arg_name, ),
                    ////move | _, state, () | get_access(state).getter()
                ////);

                ////if let PropertyMethodOps::Deprecated(_) = &self.getter_ops {
                    ////method.deprecated();
                ////}
            ////},
            ////_ => {}
        ////}

        ////match &self.setter_ops {
            ////PropertyMethodOps::Enabled(arg_name) | PropertyMethodOps::Deprecated(arg_name) => {
                ////let set_access = access.clone();

                ////let method = builder.method(format!("Set{}", &self.name),
                    ////(*arg_name,), (),
                    ////move | ctx, state, (value,) | set_access(state).setter(ctx, value)
                    ////);

                ////if let PropertyMethodOps::Deprecated(_) = &self.getter_ops {
                    ////method.deprecated();
                ////}
            ////},
            ////_ => {}
        ////}
    //}
//}

impl<Prop> PropertyWrapper<Prop> where
Prop: DbusProperty,
Prop::DBusRepr: dbus::arg::Arg + dbus::arg::RefArg + dbus::arg::Append + for <'x> dbus::arg::Get<'x> + 'static
{
    pub fn new(
        name: impl Into<String>, property: Prop, emit_changed: bool,
        getter_ops: PropertyMethodOps, setter_ops: PropertyMethodOps
    ) -> Self {
        let name = name.into();

        Self {
            name,
            property,
            emit_changed,
            getter_ops,
            setter_ops,
            setter_hooks: Vec::new()
        }
    }

    pub fn build<State, T>(&mut self, builder: &mut IfaceBuilder<State>, access: T) where
        T: Fn(&mut State) -> &mut Self + Clone + Send + 'static,
        State: Send
    {
        let get_access = access.clone();
        let set_access = access.clone();

        let prop = builder.property(&self.name)
            .get(move |_, state| get_access(state).get())
            .set(move |_, state, value| set_access(state).set(value))
        ;

        if self.emit_changed {
            let m = prop
                .emits_changed_true()
                .changed_msg_fn();

            self.setter_hooks.push(Box::new(move |ctx: &mut Context, val| {
                m(ctx.path(), val)
            }));
        } else {
            prop.emits_changed_false();
        }

        match &self.getter_ops {
            PropertyMethodOps::Enabled(arg_name) | PropertyMethodOps::Deprecated(arg_name) => {
                let get_access = access.clone();

                let method = builder.method(format!("Get{}", &self.name),
                    (), (*arg_name, ),
                    move | _, state, () | get_access(state).getter()
                );

                if let PropertyMethodOps::Deprecated(_) = &self.getter_ops {
                    method.deprecated();
                }
            },
            _ => {}
        }

        match &self.setter_ops {
            PropertyMethodOps::Enabled(arg_name) | PropertyMethodOps::Deprecated(arg_name) => {
                let set_access = access.clone();

                let method = builder.method(format!("Set{}", &self.name),
                    (*arg_name,), (),
                    move | ctx, state, (value,) | set_access(state).setter(ctx, value)
                    );

                if let PropertyMethodOps::Deprecated(_) = &self.getter_ops {
                    method.deprecated();
                }
            },
            _ => {}
        }
    }

    fn get(&self) -> Result<Prop::DBusRepr, MethodErr> {
        self.property.get()
    }

    fn set(&mut self, value: Prop::DBusRepr) -> Result<Option<Prop::DBusRepr>, MethodErr> {
        self.property
            .set(value)
            .map_err(MethodErr::from)
            .map(|v| if self.emit_changed { Some(v) } else { None })
    }

    fn getter(&self) -> Result<(Prop::DBusRepr, ), MethodErr> {
        self.get().map(|v| (v,))
    }

    fn setter(&mut self, ctx: &mut Context, value: Prop::DBusRepr) -> Result<(), MethodErr> {
        let value = self.property.set(value)?;

        let msgs : Vec<_>  = self.setter_hooks.iter()
            .flat_map(|f| f(ctx, &value)).collect();

        msgs.into_iter().for_each(|msg| ctx.push_msg(msg));

        Ok(())
    }
}

//impl PropertyWrapper<BoolParameter> {
    //pub fn new(
        //name: impl Into<String>, property: BoolParameter, emit_changed: bool,
        //getter_ops: PropertyMethodOps, setter_ops: PropertyMethodOps
    //) -> Self {
        //let name = name.into();

        //Self {
            //name,
            //property,
            //emit_changed,
            //getter_ops,
            //setter_ops,
            //setter_hooks: Vec::new()
        //}
    //}

    //pub fn build<State, T>(&mut self, builder: &mut IfaceBuilder<State>, access: T) where
        //T: Fn(&mut State) -> &mut Self + Clone + Send + 'static,
        //State: Send
    //{
        //let get_access = access.clone();
        //let set_access = access.clone();

        //let prop = builder.property(&self.name)
            //.get(move |_, state| get_access(state).get())
            //.set(move |_, state, value| set_access(state).set(value))
        //;

        //if self.emit_changed {
            //let m = prop
                //.emits_changed_true()
                //.changed_msg_fn();

            //self.setter_hooks.push(Box::new(move |ctx: &mut Context, val| {
                //m(ctx.path(), val)
            //}));
        //} else {
            //prop.emits_changed_false();
        //}

        //match &self.getter_ops {
            //PropertyMethodOps::Enabled(arg_name) | PropertyMethodOps::Deprecated(arg_name) => {
                //let get_access = access.clone();

                //let method = builder.method(format!("Get{}", &self.name),
                    //(), (*arg_name, ),
                    //move | _, state, () | get_access(state).getter()
                //);

                //if let PropertyMethodOps::Deprecated(_) = &self.getter_ops {
                    //method.deprecated();
                //}
            //},
            //_ => {}
        //}

        //match &self.setter_ops {
            //PropertyMethodOps::Enabled(arg_name) | PropertyMethodOps::Deprecated(arg_name) => {
                //let set_access = access.clone();

                //let method = builder.method(format!("Set{}", &self.name),
                    //(*arg_name,), (),
                    //move | ctx, state, (value,) | set_access(state).setter(ctx, value)
                    //);

                //if let PropertyMethodOps::Deprecated(_) = &self.getter_ops {
                    //method.deprecated();
                //}
            //},
            //_ => {}
        //}
    //}
//}

pub struct EbcState {
    auto_refresh: PropertyWrapper<BoolParameter>,
    bw_mode: PropertyWrapper<EnumParameter<BwMode>>,
    default_waveform: PropertyWrapper<EnumParameter<Waveform>>,
}


impl EbcState {
    pub fn new() -> Self {
        let module = Module::new("rockchip_ebc");

        let auto_refresh = PropertyWrapper::new("AutoRefresh",
                module.bool_parameter("auto_refresh"),
                true,
                PropertyMethodOps::Deprecated("state_auto_refresh"),
                PropertyMethodOps::Enabled("state_auto_refres")
            );

        let bw_mode = PropertyWrapper::new("BwMode",
                module.enum_parameter("bw_mode"),
                true,
                PropertyMethodOps::Deprecated("bw_mode"),
                PropertyMethodOps::Enabled("new_mode")
            );

        let default_waveform = PropertyWrapper::new("DefaultWaveform",
                module.enum_parameter("default_waveform"),
                true,
                PropertyMethodOps::Deprecated("current_waveform"),
                PropertyMethodOps::Enabled("waveform")
            );

        Self {
            auto_refresh,
            bw_mode,
            default_waveform
        }
    }

    pub fn build_v2(&mut self, builder: &mut IfaceBuilder<State>) {
        let auto_reresh_changed_sig = builder.signal::<(), _>("AutoRefreshChanged", ()).msg_fn();
        let bw_mode_changed_sig = builder.signal::<(), _>("BwModeChanged", ()).msg_fn();
        let waveform_changed_sig = builder.signal::<(), _>("WaveformChanged", ()).msg_fn();

        self.auto_refresh.build(builder, |s: &mut State| &mut s.ebc.auto_refresh);
        self.auto_refresh.setter_hooks.push(Box::new(move |ctx, _| Some(auto_reresh_changed_sig(ctx.path(), &()))));

        self.bw_mode.build(builder, |s| &mut s.ebc.bw_mode);
        self.bw_mode.setter_hooks.push(Box::new(move |ctx, _| Some(bw_mode_changed_sig(ctx.path(), &()))));

        self.default_waveform.build(builder, |s| &mut s.ebc.default_waveform);
        self.default_waveform.setter_hooks.push(Box::new(move |ctx, _| Some(waveform_changed_sig(ctx.path(), &()))));
    }

    //pub fn build_v1(builder: &mut IfaceBuilder<State>) {
        //let _bw_dither_invert_changed = builder.signal::<( ), _>("BwDitherInvertChanged", ()).msg_fn();
        //let _dclk_select_changed = builder.signal::<( ), _>("DclkSelectChanged", ()).msg_fn();
        //let _no_off_screen_changed = builder.signal::<( ), _>("NoOffScreenChanged", ()).msg_fn();
        //let _requested_quality_or_performance_mode = builder.signal::<(u8, ), _>("RequestedQualityOrPerformance", ("requested_mode", )).msg_fn();
        //let _delay_a_changed = builder.signal::<( ), _>("DelayAChanged", ()).msg_fn();
        //let _globre_convert_before_changed = builder.signal::<( ), _>("GlobreConvertBeforeChanged", ()).msg_fn();


        ////let prop_autorefresh_changed = builder.property("AutoRefresh")
            ////.emits_changed_true()
            ////.get(|_, state| { state.ebc.auto_refresh.get() })
            ////.set(|_, state, value| { state.ebc.auto_refresh.set(value) })
            ////.changed_msg_fn();
        ////let auto_refresh_changed = builder.signal::<( ), _>("AutoRefreshChanged", ()).msg_fn();

        ////builder.method("GetAutoRefresh",
            ////(), ( "state_auto_refresh", ),
            ////move | _, state, () | state.ebc.auto_refresh.getter()
            ////).deprecated();

        ////builder.method("GetAutorefresh",
            ////(), ( "state_auto_refresh", ),
            ////move | _, state, () | state.ebc.auto_refresh.getter()
            ////).deprecated();

        ////builder.method("SetAutoRefresh",
            ////("state_auto_refresh", ), (),
            ////move | ctx, state, (val,) | {
                ////let _r = state.ebc.auto_refresh.setter(val)?;
                ////let msg = auto_refresh_changed(ctx.path(), &());
                ////ctx.push_msg(msg);

                ////prop_autorefresh_changed(ctx.path(), &val).map(|msg| ctx.push_msg(msg));

                ////Ok(())
            ////});

        //let prop_bw_mode_changed = builder.property("BwMode")
            //.emits_changed_true()
            //.get(|_, state| { state.ebc.get_bw_mode() })
            //.set(|_, state, value| { state.ebc.set_bw_mode(value) })
            //.changed_msg_fn();
        //let bw_mode_changed = builder.signal::<(), _>("BwModeChanged", ()).msg_fn();

        //builder.method("GetBwMode",
            //(), ( "current_mode", ),
            //move | _, state, () | {
                //state.ebc.get_bw_mode().map(|v| { (v,) })
            //}).deprecated();

        //builder.method("SetBwMode",
            //("new_mode",), (),
            //move | ctx, state, ( value, ) | {
                //let r = state.ebc.set_bw_mode(value)?;
                //let msg = bw_mode_changed(ctx.path(), &());
                //ctx.push_msg(msg);

                //prop_bw_mode_changed(ctx.path(), &value).map(|msg| ctx.push_msg(msg));
                //Ok(())
            //});

        //// DefaultWaveform
        //let waveform_changed = builder.signal::<(), _>("WaveformChanged", ()).msg_fn();
        //let prop_wf_changed = builder.property("DefaultWaveform")
            //.emits_changed_true()
            //.get(|_, state| { state.ebc.get_default_waveform() })
            //.set(|_, state, value| { state.ebc.set_default_waveform(value) })
            //.changed_msg_fn();

        //builder.property("default_waveform")
            //.get(|_, state| { state.ebc.get_default_waveform() })
            //.set(|_, state, value| { state.ebc.set_default_waveform(value) })
            //.deprecated();

        //builder.method("GetDefaultWaveform",
            //(), ("current_waveform", ),
            //move |_, state, ()| {
                //state.ebc.get_default_waveform().map(|v| { (v, ) })
            //}
            //)
            //.deprecated();

        //builder.method("SetDefaultWaveform",
            //("waveform", ),
            //(),
            //move |ctx, state, (waveform, ): (u8, )| {
                //state.ebc.set_default_waveform(waveform)?;

                //let signal_msg = waveform_changed(ctx.path(), &());
                //ctx.push_msg(signal_msg);

                //prop_wf_changed(ctx.path(), &waveform).map(|msg| ctx.push_msg(msg));
                //Ok(())
            //}
        //);

        //let split_area_limit_cb = builder.property("SplitAreaLimit")
            //.emits_changed_true()
            //.get(|_, state| { state.ebc.get_split_area_limit() })
            //.changed_msg_fn()
        //;
        //let split_area_limit_changed = builder.signal::<( ), _>("SplitAreaLimitChanged", ()).msg_fn();

        //builder.method("GetSplitAreaLimit",
            //(),
            //( "split_limit", ),
            //move |_: &mut Context, state, ()| {
                //state.ebc.get_split_area_limit().map(|r| {(r,)})
            //}
        //).deprecated();

        //builder.method("SetSplitAreaLimit",
            //( "split_limit", ),
            //(),
            //move |ctx, state, (split_limit, )| {
                //state.ebc.set_split_area_limit(split_limit);
                //let msg = split_area_limit_changed(ctx.path(), &());
                //ctx.push_msg(msg);
                //split_area_limit_cb(ctx.path(), &split_limit).map(|msg| ctx.push_msg(msg));
                //Ok(())
            //}
        //);

        //builder.method("TriggerGlobalRefresh",
            //(), // In args
            //(), // Out args
            //move |_, state, ()| { state.ebc.trigger_global_refresh() }
        //);
    //}

    //fn get_auto_refresh(&self) -> Result<bool, MethodErr> {
        //let ret = sys_handler::get_auto_refresh();

        //Ok(ret)
    //}

    //fn set_auto_refresh(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        //sys_handler::set_auto_refresh(value);

        //Ok(Some(value))
    //}

    //fn get_bw_dither_invert(&self) -> Result<bool, MethodErr> {
        //let ret = sys_handler::get_bw_dither_invert();

        //Ok(ret)
    //}

    //fn set_bw_dither_invert(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        //sys_handler::set_bw_dither_invert(value);

        //Ok(Some(value))
    //}

    //fn get_bw_mode(&self) -> Result<u8, MethodErr> {
        //let ret = sys_handler::get_bw_mode();

        //Ok(ret)
    //}

    //fn set_bw_mode(&mut self, value: u8) -> Result<Option<u8>, MethodErr> {
        //let bw_mode = BwMode::try_from(value).map_err(|_| {
            //MethodErr::invalid_arg(&format!("{value} is not a valide mode."))
        //})?;

        //// TODO: Make this function report error ?
        //sys_handler::set_bw_mode(bw_mode as u8);

        //Ok(Some(value))
    //}

    //fn get_dclk_select(&self) -> Result<i16, MethodErr> {
        //let r = sys_handler::get_dclk_select();

        //Ok(r)
    //}

    //fn set_dclk_select(&mut self, value: i16) -> Result<Option<i16>, MethodErr> {
        //sys_handler::set_dclk_select(value);

        //Ok(Some(value))
    //}

    //fn get_default_waveform(&self) -> Result<u8, MethodErr> {

        //Ok(sys_handler::get_default_waveform())
    //}

    ///// Set the default waveform to be used by the driver
    //fn set_default_waveform(&self, value: u8) -> Result<Option<u8>, MethodErr> {
        //let waveform = Waveform::try_from(value).map_err(|_| {
            //MethodErr::invalid_arg(&format!("{value} is not a valid Wafeform identifier"))
        //})?;

        //// TODO: Make this function report errors or reimpl.
        //sys_handler::set_default_waveform(waveform as u8);

        //Ok(Some(value))
    //}

    //fn get_globre_convert_before(&self) -> Result<bool, MethodErr> {
        //let r = sys_handler::get_globre_convert_before();

        //Ok(r)
    //}

    //fn set_globre_convert_before(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        //sys_handler::set_globre_convert_before(value);

        //Ok(Some(value))
    //}

    //fn get_no_off_screen(&self) -> Result<bool, MethodErr> {
        //Ok(sys_handler::get_no_off_screen())
    //}

    //fn set_no_off_screen(&mut self, value: bool) -> Result<Option<bool>, MethodErr> {
        //sys_handler::set_no_off_screen(value);

        //Ok(Some(value))
    //}

    //fn get_split_area_limit(&self) -> Result<u32, MethodErr> {
        //let ret = sys_handler::get_split_area_limit();
        //Ok(ret)
    //}

    //fn set_split_area_limit(&self, limit: u32) {
        //sys_handler::set_split_area_limit(limit);
    //}

    //fn trigger_global_refresh(&self) -> Result<(), MethodErr> {
        //ebc_ioctl::trigger_global_refresh();
        //Ok(())
    //}
}
