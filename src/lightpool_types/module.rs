#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Module(u8);

impl Module {
    pub const SYSTEM: Module = Module(0x00);
    pub const ACCOUNT: Module = Module(0x01);
    pub const TOKEN: Module = Module(0x02);
    pub const SPOT: Module = Module(0x03);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn value(&self) -> u8 {
        self.0
    }

    pub const fn as_u8(&self) -> u8 {
        self.0
    }
}

impl From<u8> for Module {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<Module> for u8 {
    fn from(module: Module) -> Self {
        module.0
    }
}

impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module({})", self.0)
    }
}
