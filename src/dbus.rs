use std::{fmt::Display, str::FromStr};
use dbus::MethodErr;
use dbus_crossroads::{ Context, IfaceBuilder };
use num_enum::TryFromPrimitive;

use crate::kernel::{
    BoolParameter,
    PrimitiveParameter,
    EnumParameter,
    ModuleParam,
    self
};

impl From<kernel::Error> for MethodErr {
    fn from(value: kernel::Error) -> Self {
        match value {
            kernel::Error::IoError(_) => MethodErr::failed("Internal Error"),
            kernel::Error::ParseError => MethodErr::failed("Internal Error"),
            kernel::Error::ConvertError => MethodErr::invalid_arg("Bad parameter type")
        }
    }
}

trait Property {
    type DBusRepr;

    fn get(&self) -> Result<Self::DBusRepr, MethodErr>;
    fn set(&mut self, value: Self::DBusRepr) -> Result<Self::DBusRepr, MethodErr>;
}

impl Property for BoolParameter {
    type DBusRepr = <BoolParameter as ModuleParam<bool>>::Repr;

    fn get(&self) -> Result<Self::DBusRepr, MethodErr> {
        self.read().map_err(MethodErr::from)
    }

    fn set(&mut self, value: bool) -> Result<bool, MethodErr> {
        self.write(value).map_err(MethodErr::from)
    }
}

impl<T> Property for PrimitiveParameter<T> where
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

impl<T> Property for EnumParameter<T> where
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

pub enum PropertyMethodOps {
    Enabled(&'static str),
    Deprecated(&'static str),
    Disabled
}

pub struct PropertyWrapper<T>
    where T: Property
{
    name: String,
    property: T,
    emit_changed: bool,
    getter_ops: PropertyMethodOps,
    setter_ops: PropertyMethodOps,
    pub setter_hooks: Vec<Box<dyn Fn(&mut Context, &T::DBusRepr) -> Option<dbus::Message> + Send>>
}

impl<Prop> PropertyWrapper<Prop> where
Prop: Property,
Prop::DBusRepr: dbus::arg::Arg + dbus::arg::RefArg + dbus::arg::Append + for <'x> dbus::arg::Get<'x> + 'static
{
    pub fn new(
        property: Prop, name: impl Into<String>, emit_changed: bool,
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
