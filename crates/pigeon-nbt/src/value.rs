use indexmap::IndexMap;
use std::collections::BTreeMap;

/// NBT tag IDs per the NBT specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NbtTag {
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
    LongArray = 12,
}

impl NbtTag {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::End),
            1 => Some(Self::Byte),
            2 => Some(Self::Short),
            3 => Some(Self::Int),
            4 => Some(Self::Long),
            5 => Some(Self::Float),
            6 => Some(Self::Double),
            7 => Some(Self::ByteArray),
            8 => Some(Self::String),
            9 => Some(Self::List),
            10 => Some(Self::Compound),
            11 => Some(Self::IntArray),
            12 => Some(Self::LongArray),
            _ => None,
        }
    }
}

/// A single NBT value carrying its tag type.
#[derive(Debug, Clone, PartialEq)]
pub enum NbtValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(NbtList),
    Compound(NbtCompound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl NbtValue {
    pub fn tag(&self) -> NbtTag {
        match self {
            Self::Byte(_) => NbtTag::Byte,
            Self::Short(_) => NbtTag::Short,
            Self::Int(_) => NbtTag::Int,
            Self::Long(_) => NbtTag::Long,
            Self::Float(_) => NbtTag::Float,
            Self::Double(_) => NbtTag::Double,
            Self::ByteArray(_) => NbtTag::ByteArray,
            Self::String(_) => NbtTag::String,
            Self::List(_) => NbtTag::List,
            Self::Compound(_) => NbtTag::Compound,
            Self::IntArray(_) => NbtTag::IntArray,
            Self::LongArray(_) => NbtTag::LongArray,
        }
    }
}

/// A homogeneous list of NBT values (all same tag type).
#[derive(Debug, Clone, PartialEq)]
pub struct NbtList {
    pub element_type: NbtTag,
    pub elements: Vec<NbtValue>,
}

impl NbtList {
    pub fn new(element_type: NbtTag) -> Self {
        Self {
            element_type,
            elements: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// A compound tag preserving insertion order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NbtCompound {
    pub entries: IndexMap<String, NbtValue>,
}

impl NbtCompound {
    pub fn new() -> Self {
        Self {
            entries: IndexMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, value: NbtValue) {
        self.entries.insert(name.into(), value);
    }

    pub fn get(&self, name: &str) -> Option<&NbtValue> {
        self.entries.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut NbtValue> {
        self.entries.get_mut(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<NbtValue> {
        self.entries.shift_remove(name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &NbtValue)> {
        self.entries.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }
}

impl FromIterator<(String, NbtValue)> for NbtCompound {
    fn from_iter<I: IntoIterator<Item = (String, NbtValue)>>(iter: I) -> Self {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}

/// Top-level NBT root, always a single named compound.
#[derive(Debug, Clone, PartialEq)]
pub struct Nbt {
    pub name: String,
    pub root: NbtCompound,
}

impl Nbt {
    pub fn new(name: impl Into<String>, root: NbtCompound) -> Self {
        Self {
            name: name.into(),
            root,
        }
    }

    pub fn unnamed(root: NbtCompound) -> Self {
        Self {
            name: String::new(),
            root,
        }
    }
}

/// Temporary legacy alias kept so that downstream code referencing
/// `BTreeMap`-based compounds continues to type-check during the migration.
#[deprecated(note = "use NbtCompound instead")]
pub type LegacyCompound = BTreeMap<String, NbtValue>;
