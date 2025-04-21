use std::fmt::Display;
use std::str::FromStr;
use std::{fs::OpenOptions, marker::PhantomData};
use std::io::{self, BufRead, BufReader, Write};

use num_enum::TryFromPrimitive;

pub mod enums {
    use num_enum::{IntoPrimitive, TryFromPrimitive};

    #[derive(TryFromPrimitive, IntoPrimitive, Clone)]
    #[repr(u8)]
    pub enum Waveform {
        Reset,
        A2,
        DU,
        DU4,
        GC16,
        GCC16,
        GL16,
        GLR16,
        GLD16,
    }

    #[derive(TryFromPrimitive, IntoPrimitive, Clone)]
    #[repr(u8)]
    pub enum BwMode {
        Gray16,
        BWDither,
        BWTheshold,
        Gray4
    }
}

pub enum Error {
    IoError(io::Error),
    ParseError,
    ConvertError
}

pub struct Module {
    name: String
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name
        }
    }

    pub fn primitive_parameter<T>(&self, parameter: impl Into<String>) -> PrimitiveParameter<T>
    where T: FromStr + Display
    {
        PrimitiveParameter::new(&self.name, parameter)
    }

    pub fn enum_parameter<T>(&self, parameter: impl Into<String>) -> EnumParameter<T>
    where
        T: TryFromPrimitive + Into<T::Primitive>,
        T::Primitive: FromStr + Display
    {
        EnumParameter::new(&self.name, parameter)
    }

    pub fn bool_parameter(&self, parameter: impl Into<String>) -> BoolParameter {
        BoolParameter::new(&self.name, parameter)
    }
}

struct ParameterCommon {
    module: String,
    parameter: String,
    path: String,
}

impl ParameterCommon {
    fn new(module: impl Into<String>, parameter: impl Into<String>) -> Self {
        let module = module.into();
        let parameter = parameter.into();
        let path = format!("/sys/module/{module}/parameters/{parameter}");

        Self {
            module,
            parameter,
            path
        }
    }

    fn module(&self) -> &str { &self.module }
    fn parameter(&self) -> &str { &self.parameter }
    fn path(&self) -> &str { &self.path }
}

pub struct PrimitiveParameter<Repr> {
    common: ParameterCommon,
    phantom: PhantomData<Repr>
}

impl<Repr> PrimitiveParameter<Repr> {
    pub fn new(module: impl Into<String>, parameter: impl Into<String>) -> Self {
        let common = ParameterCommon::new(module, parameter);

        Self {
            common,
            phantom: PhantomData::<Repr>{}
        }
    }

    pub fn module(&self) -> &str { self.common.module() }
    pub fn parameter(&self) -> &str { self.common.parameter() }
    pub fn path(&self) -> &str { self.common.path() }
}

pub struct BoolParameter {
    common: ParameterCommon,
}

impl BoolParameter {
    pub fn new(module: impl Into<String>, parameter: impl Into<String>) -> Self {
        Self {
            common: ParameterCommon::new(module, parameter)
        }
    }

    pub fn module(&self) -> &str { self.common.module() }
    pub fn parameter(&self) -> &str { self.common.parameter() }
    pub fn path(&self) -> &str { self.common.path() }
}

pub struct EnumParameter<Enum> where
Enum: TryFromPrimitive + Into<Enum::Primitive>
{
    common: ParameterCommon,
    phantom: PhantomData<Enum>
}

impl<Enum> EnumParameter<Enum> where
Enum: TryFromPrimitive + Into<Enum::Primitive>,
Enum::Primitive: FromStr + Display
{
    pub fn new(module: impl Into<String>, parameter: impl Into<String>) -> Self {
        let common = ParameterCommon::new(module, parameter);

        Self {
            common,
            phantom: PhantomData::<Enum>{}
        }
    }

    pub fn module(&self) -> &str { self.common.module() }
    pub fn parameter(&self) -> &str { self.common.parameter() }
    pub fn path(&self) -> &str { self.common.path() }
}

pub trait ModuleParamBase {
    fn get_path(&self) -> String;

    fn read_raw(&self) -> Result<String, Error> {
        let path = self.get_path();
        let file_result = OpenOptions::new().read(true).open(&path);

        match file_result {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut buf = String::new();
                let mut num_bytes = 1;

                while num_bytes > 0 {
                    num_bytes = reader.read_line(&mut buf).map_err(|e| { Error::IoError(e) })?;
                }

                return Ok(buf.trim_end().to_string());
            },
            Err(e) => {
                eprintln!("Error while opening file {} for reading: error {}", path, e);
                return Err(Error::IoError(e))
            }
        }
    }

    fn write_raw(&self, value: String) -> Result<(), Error> {
        let path = self.get_path();
        eprintln!("Writing to {}: {}", path, value);

        OpenOptions::new()
            .write(true)
            .open(&path)
            .and_then(|mut f| { write!(f, "{}", value) })
            .map_err(|e| Error::IoError(e))
    }

}

impl<T> ModuleParamBase for PrimitiveParameter<T> {
    fn get_path(&self) -> String {
        self.path().to_string()
    }
}

impl<T> ModuleParamBase for EnumParameter<T> where
    T: TryFromPrimitive + Into<T::Primitive>,
    T::Primitive: FromStr + Display
{
    fn get_path(&self) -> String {
        self.path().to_string()
    }
}

impl ModuleParamBase for BoolParameter {
    fn get_path(&self) -> String {
        self.path().to_string()
    }
}

pub trait ModuleParam<T> : ModuleParamBase {
    type Repr;

    fn read(&self) -> Result<T, Error>;
    fn write(&self, value: T) -> Result<T, Error>;
}

impl<T> ModuleParam<T> for PrimitiveParameter<T> where
T: FromStr + Display
{
    type Repr = T;

    fn read(&self) -> Result<T, Error> {
        self.read_raw()
            .and_then(|v| v.parse::<T>().map_err(|_| Error::ParseError ))
    }

    fn write(&self, value: T) -> Result<T, Error> {
        self.write_raw(format!("{value}"))?;
        Ok(value)
    }
}

impl<T> ModuleParam<T> for EnumParameter<T> where
T: TryFromPrimitive + Into<T::Primitive> + Clone,
T::Primitive: FromStr + Display
{
    type Repr = T::Primitive;

    fn read(&self) -> Result<T, Error> {
        self.read_raw()
            .and_then(|v| v.parse::<Self::Repr>().map_err(|_| Error::ParseError ))
            .and_then(|v| T::try_from_primitive(v).map_err(|_| Error::ConvertError ))
    }

    fn write(&self, value:T) -> Result<T, Error> {
        let prim = value.clone().into();
        self.write_raw(format!("{prim}"))?;
        Ok(value)
    }
}

impl ModuleParam<bool> for BoolParameter {
    type Repr = bool;

    fn read(&self) -> Result<bool, Error> {
        let repr = self.read_raw()?;

        match repr.trim() {
            "Y" | "1" => Ok(true),
            "N" | "0" => Ok(false),
            _ => Err(Error::ParseError)
        }
    }

    fn write(&self, value: bool) -> Result<bool, Error> {
        self.write_raw(format!("{}", value as u8))?;
        Ok(value)
    }
}

