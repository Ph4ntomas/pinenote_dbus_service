use std::fmt::Display;
use std::str::FromStr;
use std::{fs::OpenOptions, marker::PhantomData};
use std::io::{self, BufRead, BufReader, Write};

use num_enum::TryFromPrimitive;

pub enum Error {
    IoError(io::Error),
    ParseError,
    ConvertError
}

///
/// SysFS Module representation
///
/// This class is meant to build a quick binding to the sysFS representation of
/// a kernel module. The *_parameter method allows to instantiate a type-safe
/// representation of a given parameter.
///
/// # Example
/// ```rust
/// let foo = Module::new("foo");
/// let bar = foo.primitive_parameter("bar")
///
/// // read /sys/module/foo/parameters/bar
/// let value = bar.read();
/// // write /sys/module/foo/parameters/bar
/// bar.write(value + 1);
/// ```
///
pub struct Module {
    name: String
}

impl Module {
    /// Create a new Module which reside at `/sys/module/{name}`
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

    pub fn generic_parameter<T>(&self, parameter: impl Into<String>) -> GenericParameter<T>
    where
        T: TryFromKernelParam + Into<T::KRepr>,
        T::KRepr: FromStr + Display
    {
        GenericParameter::new(&self.name, parameter)
    }
}

///
/// Common part for every module parameters.
///
#[doc(hidden)]
struct ParameterCommon {
    module: String,
    parameter: String,
    path: String,
}

impl ParameterCommon {
    ///
    /// Initialize a new `ParameterCommon` structure.
    ///
    /// The parameter will have its path set to `/sys/module/{name}/parameters/{parameter}`
    ///
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

    /// Access the module name
    fn module(&self) -> &str { &self.module }
    /// Access the parameter name
    fn parameter(&self) -> &str { &self.parameter }
    /// Access the parameter path
    fn path(&self) -> &str { &self.path }
}

///
/// Convert a parameter from its kernel representation.
///
/// `TryFromKernelParam`'s [`try_from_kernel`] is used implicitely in [ModuleParam]'s
/// blanket implementation for [GenericParameter].
///
/// [`try_from_kernel`]: TryFromKernelParam::try_from_kernel
///
pub trait TryFromKernelParam
where Self: Sized
{
    /// The associated kernel representation.
    type KRepr;
    /// The associated error which can be returned from parsing.
    type Error;

    /// Parses a value to a return value of this type.
    ///
    /// If parsing succeeds, return the value inside [`Ok`], otherwise when the
    /// conversion failed, return an error specific to the type, inside [`Err`].
    /// The error type is specific to the implementation of the trait.
    ///
    /// # Examples
    ///
    /// TODO: Add example
    fn try_from_kernel(value: Self::KRepr) -> Result<Self, Self::Error>;
}

/// Parameter whose internal representation is a primitive type.
///
/// This type is used to access parameter whose value doesn't have any special
/// Rust representation, and that can be easily parsed via the [FromStr] traits.
///
/// Note, since boolean values can sometime be represented by Y or N, it's better
/// to use [BoolParameter].
///
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

/// Parameter whose internal representation is a boolean.
///
/// `BoolParameter` are used to binds to boolean module parameter.
/// This specialization is needed since some module uses "Y" or "N" to represent
/// booleans, which makes FromStr::from_str return an error.
///
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

/// Parameter whose rust representation is an enumeration implementing [`num_enum::TryFromPrimitive`]
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

/// Parameter with arbitrary rust representation.
pub struct GenericParameter<Type> where
Type: TryFromKernelParam + Into<Type::KRepr>
{
    common: ParameterCommon,
    phantom: PhantomData<Type>
}

impl<Type> GenericParameter<Type> where
Type: TryFromKernelParam + Into<Type::KRepr>,
Type::KRepr: FromStr + Display
{
    pub fn new(module: impl Into<String>, parameter: impl Into<String>) -> Self {
        let common = ParameterCommon::new(module, parameter);

        Self {
            common,
            phantom: PhantomData{}
        }
    }

    pub fn module(&self) -> &str { self.common.module() }
    pub fn parameter(&self) -> &str { self.common.parameter() }
    pub fn path(&self) -> &str { self.common.path() }
}

/// Common trait for module parameters.
///
/// Implementation must provide a [`get_path`] function, that returns the path to
/// the parameter file in SysFS, as well as a [`read`] and [`write`] function
///
/// [`get_path`]: ModuleParam<T>::read
/// [`read`]: ModuleParam<T>::read
/// [`write`]: ModuleParam<T>::write
///
pub trait ModuleParam<T> {
    type Repr;

    /// Returns the path to the file to read or write.
    fn get_path(&self) -> &str;

    /// Read from the parameter file and returns a String.
    fn read_raw(&self) -> Result<String, Error> {
        let path = self.get_path();
        let file_result = OpenOptions::new().read(true).open(path);

        match file_result {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut buf = String::new();
                let mut num_bytes = 1;

                while num_bytes > 0 {
                    num_bytes = reader.read_line(&mut buf).map_err(|e| { Error::IoError(e) })?;
                }

                Ok(buf.trim_end().to_string())
            },
            Err(e) => {
                eprintln!("Error while opening file {} for reading: error {}", path, e);
                Err(Error::IoError(e))
            }
        }
    }

    /// Write a string to the parameter file.
    fn write_raw(&self, value: impl Into<String>) -> Result<(), Error> {
        let path = self.get_path();

        OpenOptions::new()
            .write(true)
            .open(path)
            .and_then(|mut f| { write!(f, "{}", value.into()) })
            .map_err(Error::IoError)
    }

    /// Read from the parameter file and returns a parsed value.
    fn read(&self) -> Result<T, Error>;
    /// Convert the value to a string, and write it to the file.
    fn write(&self, value: T) -> Result<(), Error>;
}

impl<T> ModuleParam<T> for PrimitiveParameter<T> where
T: FromStr + Display
{
    type Repr = T;

    fn get_path(&self) -> &str {
        self.path()
    }

    fn read(&self) -> Result<T, Error> {
        self.read_raw()
            .and_then(|v| v.parse::<T>().map_err(|_| Error::ParseError ))
    }

    fn write(&self, value: T) -> Result<(), Error> {
        self.write_raw(format!("{value}"))?;
        Ok(())
    }
}

impl<T> ModuleParam<T> for EnumParameter<T> where
T: TryFromPrimitive + Into<T::Primitive>,
T::Primitive: FromStr + Display
{
    type Repr = T::Primitive;

    fn get_path(&self) -> &str{
        self.path()
    }

    fn read(&self) -> Result<T, Error> {
        self.read_raw()
            .and_then(|v| v.parse::<Self::Repr>().map_err(|_| Error::ParseError ))
            .and_then(|v| T::try_from_primitive(v).map_err(|_| Error::ConvertError ))
    }

    fn write(&self, value:T) -> Result<(), Error> {
        let prim = value.into();
        self.write_raw(format!("{prim}"))?;
        Ok(())
    }
}

impl<T> ModuleParam<T> for GenericParameter<T> where
T: TryFromKernelParam + Into<T::KRepr>,
T::KRepr: FromStr + Display
{
    type Repr = T::KRepr;

    fn get_path(&self) -> &str {
        self.path()
    }

    fn read(&self) -> Result<T, Error> {
        self.read_raw()
            .and_then(|v| v.parse::<Self::Repr>().map_err(|_| Error::ParseError ))
            .and_then(|v| T::try_from_kernel(v).map_err(|_| Error::ConvertError ))
    }

    fn write(&self, value:T) -> Result<(), Error> {
        let prim = value.into();
        self.write_raw(format!("{prim}"))?;
        Ok(())
    }
}

impl ModuleParam<bool> for BoolParameter {
    type Repr = bool;

    fn get_path(&self) -> &str {
        self.path()
    }

    fn read(&self) -> Result<bool, Error> {
        let repr = self.read_raw()?;

        match repr.trim() {
            "Y" | "1" => Ok(true),
            "N" | "0" => Ok(false),
            _ => Err(Error::ParseError)
        }
    }

    fn write(&self, value: bool) -> Result<(), Error> {
        self.write_raw(format!("{}", value as u8))?;
        Ok(())
    }
}
