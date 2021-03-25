use std::error;
use std::fmt;
use std::str::FromStr;

use rustpython_parser::ast::{Expression, ExpressionType, Number, StatementType, StringGroup};
use rustpython_parser::error::ParseError;
use rustpython_parser::parser;

/// Represents an error during parsing
#[derive(Debug)]
pub enum Error {
    /// Python source code syntax error
    SyntaxError(ParseError),
    /// missing build_time_vars variable
    MissingBuildTimeVars,
    /// missing required key in configuration
    KeyError(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SyntaxError(err) => err.fmt(f),
            Error::MissingBuildTimeVars => write!(f, "missing build_time_vars variable"),
            Error::KeyError(key) => write!(f, "missing required key {}", key),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::SyntaxError(err) => Some(err),
            Error::MissingBuildTimeVars => None,
            Error::KeyError(_) => None,
        }
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Self::SyntaxError(err)
    }
}

/// Python configuration information
#[derive(Debug, Clone)]
pub struct PythonConfig {
    sys_config_data: SysConfigData,
}

impl PythonConfig {
    /// Parse from `_sysconfigdata.py` content
    pub fn parse(src: &str) -> Result<Self, Error> {
        let sys_config_data = SysConfigData::parse(src)?;
        Ok(Self { sys_config_data })
    }

    /// Returns Python version
    pub fn version(&self) -> &str {
        &self.sys_config_data.build_time_vars.version
    }

    /// Returns Python major version
    pub fn version_major(&self) -> u32 {
        let version = self.version();
        version
            .split('.')
            .next()
            .and_then(|x| x.parse::<u32>().ok())
            .unwrap()
    }

    /// Returns Python minor version
    pub fn version_minor(&self) -> u32 {
        let version = self.version();
        version
            .split('.')
            .nth(1)
            .and_then(|x| x.parse::<u32>().ok())
            .unwrap()
    }

    /// Returns the installation prefix of the Python interpreter
    pub fn prefix(&self) -> &str {
        &self.sys_config_data.build_time_vars.prefix
    }

    /// Returns the executable path prefix for the Python interpreter
    pub fn exec_prefix(&self) -> &str {
        &self.sys_config_data.build_time_vars.exec_prefix
    }

    /// C compilation flags
    pub fn cflags(&self) -> &str {
        &self.sys_config_data.build_time_vars.cflags
    }

    /// Returns linker flags required for linking this Python
    /// distribution. All libraries / frameworks have the appropriate `-l`
    /// or `-framework` prefixes.
    pub fn libs(&self) -> &str {
        &self.sys_config_data.build_time_vars.libs
    }

    /// Returns linker flags required for creating
    /// a shared library for this Python distribution. All libraries / frameworks
    /// have the appropriate `-L`, `-l`, or `-framework` prefixes.
    pub fn ldflags(&self) -> &str {
        &self.sys_config_data.build_time_vars.ldflags
    }

    /// Returns the file extension for this distribution's library
    pub fn ext_suffix(&self) -> &str {
        &self.sys_config_data.build_time_vars.ext_suffix
    }

    /// The ABI flags specified when building this Python distribution
    pub fn abiflags(&self) -> &str {
        &self.sys_config_data.build_time_vars.abiflags
    }

    /// The location of the distribution's `python3-config` script
    pub fn config_dir(&self) -> &str {
        &self.sys_config_data.build_time_vars.config_dir
    }

    /// Returns the C headers include directory
    pub fn include_dir(&self) -> &str {
        &self.sys_config_data.build_time_vars.include_dir
    }

    /// Returns library directory
    pub fn lib_dir(&self) -> &str {
        &self.sys_config_data.build_time_vars.lib_dir
    }

    /// Returns ld version
    pub fn ld_version(&self) -> &str {
        &self.sys_config_data.build_time_vars.ld_version
    }

    /// Returns SOABI
    pub fn soabi(&self) -> &str {
        &self.sys_config_data.build_time_vars.soabi
    }

    /// Returns shared library suffix
    pub fn shlib_suffix(&self) -> &str {
        &self.sys_config_data.build_time_vars.shlib_suffix
    }

    /// Returns whether this distribution is built with `--enable-shared`
    pub fn enable_shared(&self) -> bool {
        self.sys_config_data.build_time_vars.py_enable_shared
    }

    /// Returns whether this distribution is built with `Py_DEBUG`
    pub fn debug(&self) -> bool {
        self.sys_config_data.build_time_vars.py_debug
    }

    /// Returns whether this distribution is built with `Py_REF_DEBUG`
    pub fn ref_debug(&self) -> bool {
        self.sys_config_data.build_time_vars.py_ref_debug
    }

    /// Returns whether this distribution is built with thread
    pub fn with_thread(&self) -> bool {
        self.sys_config_data.build_time_vars.with_thread
    }

    /// Returns pointer size (size of C `void*`) of this distribution
    pub fn pointer_size(&self) -> u32 {
        self.sys_config_data.build_time_vars.size_of_void_p
    }
}

#[derive(Debug, Clone)]
struct SysConfigData {
    pub build_time_vars: BuildTimeVars,
}

#[derive(Debug, Clone, Default)]
struct BuildTimeVars {
    pub abiflags: String,
    pub count_allocs: bool,
    pub cflags: String,
    pub config_dir: String,
    pub ext_suffix: String,
    pub exec_prefix: String,
    pub include_dir: String,
    pub lib_dir: String,
    pub libs: String,
    pub ldflags: String,
    pub ld_version: String,
    pub prefix: String,
    pub py_debug: bool,
    pub py_ref_debug: bool,
    pub py_trace_refs: bool,
    pub py_enable_shared: bool,
    pub soabi: String,
    pub shlib_suffix: String,
    pub size_of_void_p: u32,
    pub with_thread: bool,
    pub version: String,
}

impl SysConfigData {
    pub fn parse(src: &str) -> Result<Self, Error> {
        let program = parser::parse_program(src)?;
        let mut vars = BuildTimeVars::default();
        for stmt in program.statements {
            if let StatementType::Assign { targets, value } = stmt.node {
                let var_name = targets.get(0).ok_or(Error::MissingBuildTimeVars)?;
                match &var_name.node {
                    ExpressionType::Identifier { name } if name == "build_time_vars" => {}
                    _ => continue,
                }
                if let ExpressionType::Dict { elements } = value.node {
                    for (key, value) in elements {
                        if let Some(key) = key.and_then(|key| get_string(&key)) {
                            match key.as_str() {
                                "ABIFLAGS" => {
                                    vars.abiflags = get_string(&value).unwrap_or_default()
                                }
                                "COUNT_ALLOCS" => vars.count_allocs = get_bool(&value),
                                "CFLAGS" => vars.cflags = get_string(&value).unwrap_or_default(),
                                "LIBPL" => vars.config_dir = get_string(&value).unwrap_or_default(),
                                "EXT_SUFFIX" => {
                                    vars.ext_suffix = get_string(&value).unwrap_or_default()
                                }
                                "exec_prefix" => {
                                    vars.exec_prefix = get_string(&value).unwrap_or_default()
                                }
                                "INCLUDEDIR" => {
                                    vars.include_dir = get_string(&value).unwrap_or_default()
                                }
                                "LIBDIR" => vars.lib_dir = get_string(&value).unwrap_or_default(),
                                "LIBS" => vars.libs = get_string(&value).unwrap_or_default(),
                                "LDFLAGS" => vars.ldflags = get_string(&value).unwrap_or_default(),
                                "LDVERSION" => {
                                    vars.ld_version = get_string(&value).unwrap_or_default()
                                }
                                "prefix" => vars.prefix = get_string(&value).unwrap_or_default(),
                                "Py_DEBUG" => vars.py_debug = get_bool(&value),
                                "Py_ENABLE_SHARED" => vars.py_enable_shared = get_bool(&value),
                                "Py_REF_DEBUG" => vars.py_ref_debug = get_bool(&value),
                                "Py_TRACE_REFS" => vars.py_trace_refs = get_bool(&value),
                                "SOABI" => vars.soabi = get_string(&value).unwrap_or_default(),
                                "SHLIB_SUFFIX" => {
                                    vars.shlib_suffix = get_string(&value).unwrap_or_default()
                                }
                                "SIZEOF_VOID_P" => {
                                    vars.size_of_void_p = get_number(&value)
                                        .ok_or(Error::KeyError("SIZEOF_VOID_P"))?
                                        as u32
                                }
                                "VERSION" => {
                                    vars.version =
                                        get_string(&value).ok_or(Error::KeyError("VERSION"))?
                                }
                                _ => continue,
                            }
                        } else {
                            continue;
                        }
                    }
                }
            }
        }
        if vars.version.is_empty() {
            // no build_time_vars found
            return Err(Error::MissingBuildTimeVars);
        }
        Ok(SysConfigData {
            build_time_vars: vars,
        })
    }
}

fn get_string(expr: &Expression) -> Option<String> {
    match &expr.node {
        ExpressionType::String { value: sg } => match sg {
            StringGroup::Constant { value } => Some(value.to_string()),
            StringGroup::Joined { values } => {
                let mut s = String::new();
                for value in values {
                    if let StringGroup::Constant { value: cs } = value {
                        s.push_str(&cs)
                    }
                }
                Some(s)
            }
            _ => None,
        },
        _ => None,
    }
}

fn get_number(expr: &Expression) -> Option<i32> {
    use num_traits::cast::ToPrimitive;

    match &expr.node {
        ExpressionType::Number { value } => {
            if let Number::Integer { value } = value {
                value.to_i32()
            } else {
                None
            }
        }
        _ => None,
    }
}

fn get_bool(expr: &Expression) -> bool {
    get_number(expr).map(|x| x == 1).unwrap_or(false)
}

impl FromStr for PythonConfig {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, PythonConfig};
    use std::fs;

    #[test]
    fn read_python_sysconfig_data() {
        let src =
            fs::read_to_string("tests/fixtures/cpython38_sysconfigdata__darwin_darwin.py").unwrap();
        let config = PythonConfig::parse(&src).unwrap();
        assert_eq!(config.abiflags(), "");
        assert_eq!(config.soabi(), "cpython-38-darwin");
        assert_eq!(config.version(), "3.8");
        assert_eq!(config.version_major(), 3);
        assert_eq!(config.version_minor(), 8);

        // Test FromStr impl
        let config: PythonConfig = src.parse().unwrap();
        assert_eq!(config.abiflags(), "");
    }

    #[test]
    fn read_invalid_python_sysconfig_data() {
        let config = PythonConfig::parse("i++").unwrap_err();
        assert!(matches!(config, Error::SyntaxError(_)));

        let config = PythonConfig::parse("").unwrap_err();
        assert!(matches!(config, Error::MissingBuildTimeVars));

        let config = PythonConfig::parse("i = 0").unwrap_err();
        assert!(matches!(config, Error::MissingBuildTimeVars));

        let config =
            PythonConfig::parse("build_time_vars = {'VERSION': '3.8', 'SIZEOF_VOID_P': ''}")
                .unwrap_err();
        assert!(matches!(config, Error::KeyError("SIZEOF_VOID_P")));
    }
}
