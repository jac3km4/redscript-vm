use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use enum_as_inner::EnumAsInner;
use gc_arena::lock::{GcRefLock, RefLock};
use gc_arena::{Collect, Gc, Mutation};
use itertools::{Either, Itertools};
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::definition::{Class, Field};

use crate::index_map::IndexMap;
use crate::interop::{FromVM, IntoVM};
use crate::metadata::{Metadata, TypeId};

#[derive(Debug, Clone, Collect, EnumAsInner)]
#[collect(no_drop)]
pub enum Value<'gc> {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Bool(bool),
    EnumVal(i64),
    PackedStruct(PackedStruct),
    BoxedStruct(GcRefLock<'gc, IndexMap<Value<'gc>>>),
    Obj(Obj<'gc>),
    Str(Gc<'gc, Box<str>>),
    InternStr(StringType, VMIndex),
    Array(GcRefLock<'gc, Vec<Value<'gc>>>),
    Pinned(GcRefLock<'gc, Value<'gc>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Collect)]
#[collect(require_static)]
pub enum StringType {
    String,
    Name,
    TweakDbId,
    Resource,
}

impl<'gc> Value<'gc> {
    #[inline]
    pub fn unpinned(&self) -> impl Deref<Target = Self> + '_ {
        match self {
            Value::Pinned(cell) => Either::Left(cell.borrow()),
            other => Either::Right(other),
        }
    }

    #[inline]
    pub fn pin(&mut self, mc: &Mutation<'gc>) {
        if !self.is_pinned() {
            let pinned = Value::Pinned(Gc::new(mc, RefLock::new(self.clone())));
            *self = pinned;
        }
    }

    #[inline]
    pub fn copied(&self, mc: &Mutation<'gc>) -> Self {
        match self {
            Value::BoxedStruct(str) => Value::BoxedStruct(Gc::new(mc, str.as_ref().clone())),
            other => other.clone(),
        }
    }

    pub fn to_string(&self, pool: &ConstantPool) -> String {
        fn aggregate_to_string(fields: &IndexMap<Value<'_>>, pool: &ConstantPool) -> String {
            let formatted = fields
                .iter::<Field>()
                .map(|(idx, val)| format!("{}: {}", pool.def_name(idx).unwrap(), val.to_string(pool)))
                .format(", ");
            format!("{{{formatted}}}")
        }

        match self {
            Value::I8(i) => i.to_string(),
            Value::I16(i) => i.to_string(),
            Value::I32(i) => i.to_string(),
            Value::I64(i) => i.to_string(),
            Value::U8(i) => i.to_string(),
            Value::U16(i) => i.to_string(),
            Value::U32(i) => i.to_string(),
            Value::U64(i) => i.to_string(),
            Value::F32(i) => i.to_string(),
            Value::F64(i) => i.to_string(),
            Value::Bool(i) => i.to_string(),
            Value::EnumVal(i) => i.to_string(),
            Value::PackedStruct(_) => todo!(),
            Value::BoxedStruct(struct_) => aggregate_to_string(&struct_.borrow(), pool),
            Value::Obj(Obj::Null) => "null".to_string(),
            Value::Obj(Obj::Instance(inst)) => aggregate_to_string(&inst.borrow().fields, pool),
            Value::Str(str) => str.as_ref().clone().into_string(),
            Value::InternStr(StringType::String, idx) => pool.strings.get(idx.to_pool()).unwrap().deref().to_owned(),
            Value::InternStr(StringType::Name, idx) => pool.names.get(idx.to_pool()).unwrap().deref().to_owned(),
            Value::InternStr(StringType::TweakDbId, idx) => {
                pool.tweakdb_ids.get(idx.to_pool()).unwrap().as_ref().to_owned()
            }
            Value::InternStr(StringType::Resource, idx) => {
                pool.resources.get(idx.to_pool()).unwrap().as_ref().to_owned()
            }
            Value::Array(arr) => {
                let arr = arr.borrow();
                let formatted = arr.iter().map(|val| val.to_string(pool)).format(", ");
                format!("[{formatted}]")
            }
            Value::Pinned(v) => v.borrow().to_string(pool),
        }
    }

    pub fn equals(&self, other: &Self) -> bool {
        match (&*self.unpinned(), &*other.unpinned()) {
            (Value::I8(lhs), Value::I8(rhs)) => lhs == rhs,
            (Value::I16(lhs), Value::I16(rhs)) => lhs == rhs,
            (Value::I32(lhs), Value::I32(rhs)) => lhs == rhs,
            (Value::I64(lhs), Value::I64(rhs)) => lhs == rhs,
            (Value::U8(lhs), Value::U8(rhs)) => lhs == rhs,
            (Value::U16(lhs), Value::U16(rhs)) => lhs == rhs,
            (Value::U32(lhs), Value::U32(rhs)) => lhs == rhs,
            (Value::U64(lhs), Value::U64(rhs)) => lhs == rhs,
            (Value::F32(lhs), Value::F32(rhs)) => lhs == rhs,
            (Value::F64(lhs), Value::F64(rhs)) => lhs == rhs,
            (Value::Bool(lhs), Value::Bool(rhs)) => lhs == rhs,
            (Value::EnumVal(lhs), Value::EnumVal(rhs)) => lhs == rhs,
            (Value::Str(lhs), Value::Str(rhs)) => *lhs == *rhs,
            (Value::InternStr(ltyp, lidx), Value::InternStr(rtyp, ridx)) => ltyp == rtyp && lidx == ridx,
            _ => false,
        }
    }

    pub fn has_type(&self, typ: &TypeId) -> bool {
        match (self, typ) {
            (Value::I8(_), TypeId::I8)
            | (Value::I16(_), TypeId::I16)
            | (Value::I32(_), TypeId::I32)
            | (Value::I64(_), TypeId::I64)
            | (Value::U8(_), TypeId::U8)
            | (Value::U16(_), TypeId::U16)
            | (Value::U32(_), TypeId::U32)
            | (Value::U64(_), TypeId::U64)
            | (Value::F32(_), TypeId::F32)
            | (Value::F64(_), TypeId::F64)
            | (Value::Bool(_), TypeId::Bool)
            | (Value::EnumVal(_), TypeId::Enum(_))
            | (Value::BoxedStruct(_) | Value::PackedStruct(_), TypeId::Struct(_))
            | (Value::Obj(Obj::Null), TypeId::Ref(_) | TypeId::WRef(_))
            | (Value::Str(_) | Value::InternStr(StringType::String, _), TypeId::String)
            | (Value::InternStr(StringType::Name, _), TypeId::CName)
            | (Value::InternStr(StringType::TweakDbId, _), TypeId::TweakDbId)
            | (Value::InternStr(StringType::Resource, _), TypeId::ResRef)
            | (Value::Array(_), TypeId::Array(_)) => true,
            (Value::Obj(Obj::Instance(cell)), TypeId::Ref(class) | TypeId::WRef(class)) => {
                cell.borrow().tag.to_pool() == *class
            }
            (Value::Pinned(val), _) => val.borrow().has_type(typ),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Collect, EnumAsInner)]
#[collect(no_drop)]
pub enum Obj<'gc> {
    Null,
    Instance(GcRefLock<'gc, Instance<'gc>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Collect)]
#[collect(require_static)]
pub struct VMIndex(pub u32);

impl VMIndex {
    pub const ZERO: VMIndex = VMIndex(0);

    #[inline]
    pub fn to_pool<A>(self) -> PoolIndex<A> {
        PoolIndex::new(self.0)
    }
}

impl<A> From<PoolIndex<A>> for VMIndex {
    #[inline]
    fn from(idx: PoolIndex<A>) -> Self {
        VMIndex(idx.into())
    }
}

#[derive(Debug, Collect)]
#[collect(no_drop)]
pub struct Instance<'gc> {
    pub tag: VMIndex,
    pub fields: IndexMap<Value<'gc>>,
    pub vtable: Rc<IndexMap<VMIndex>>,
}

impl<'gc> Instance<'gc> {
    pub fn new(idx: PoolIndex<Class>, meta: &mut Metadata<'_>, mc: &Mutation<'gc>) -> Self {
        let mut current = idx;
        let mut fields = IndexMap::new();
        while !current.is_undefined() {
            let class = meta.pool().class(current).unwrap();
            for field_idx in &class.fields {
                let field = meta.pool().field(*field_idx).unwrap();
                let typ = meta.get_type(field.type_).unwrap();
                fields.put(*field_idx, typ.default_value(mc, meta));
            }
            current = meta.pool().class(current).unwrap().base;
        }
        let vtable = meta.get_vtable(idx).unwrap();

        Self {
            tag: idx.into(),
            fields,
            vtable,
        }
    }
}

#[derive(Debug, Clone, Collect)]
#[collect(require_static)]
pub struct PackedStruct([u8; PackedStruct::MAX_SIZE]);

impl PackedStruct {
    pub const MAX_SIZE: usize = 0xf;
}

macro_rules! impl_prim_conversions {
    ($typ:ty, $constructor:ident) => {
        impl<'gc> IntoVM<'gc> for $typ {
            #[inline]
            fn into_vm<'ctx>(self, _mc: &Mutation<'gc>) -> Value<'gc> {
                Value::$constructor(self)
            }
        }

        impl<'gc> FromVM<'gc> for $typ {
            fn from_vm<'pool>(val: Value<'gc>, _pool: &'pool ConstantPool) -> Result<Self, &'static str> {
                match &*val.unpinned() {
                    Value::$constructor(i) => Ok(*i),
                    _ => Err(concat!("Invalid argument, expected ", stringify!($constructor))),
                }
            }
        }
    };
}

impl_prim_conversions!(i8, I8);
impl_prim_conversions!(i16, I16);
impl_prim_conversions!(i32, I32);
impl_prim_conversions!(i64, I64);
impl_prim_conversions!(u8, U8);
impl_prim_conversions!(u16, U16);
impl_prim_conversions!(u32, U32);
impl_prim_conversions!(u64, U64);
impl_prim_conversions!(f32, F32);
impl_prim_conversions!(f64, F64);
impl_prim_conversions!(bool, Bool);

impl<'gc> FromVM<'gc> for String {
    fn from_vm(val: Value<'gc>, pool: &ConstantPool) -> Result<Self, &'static str> {
        match &*val.unpinned() {
            Value::Str(i) => Ok(i.as_ref().clone().into_string()),
            Value::InternStr(StringType::String, idx) => pool
                .strings
                .get(idx.to_pool())
                .map(|rc| rc.to_string())
                .map_err(|_| "Unknown string constant"),
            _ => Err("Invalid argument, expected String"),
        }
    }
}

impl<'gc> IntoVM<'gc> for String {
    #[inline]
    fn into_vm(self, mc: &Mutation<'gc>) -> Value<'gc> {
        Value::Str(Gc::new(mc, self.into_boxed_str()))
    }
}

impl<'gc> IntoVM<'gc> for &'static str {
    #[inline]
    fn into_vm(self, mc: &Mutation<'gc>) -> Value<'gc> {
        Value::Str(Gc::new(mc, self.into()))
    }
}
