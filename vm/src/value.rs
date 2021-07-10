use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use enum_as_inner::EnumAsInner;
use gc_arena::{Collect, Gc, GcCell, MutationContext};
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::definition::Class;

use crate::index_map::IndexMap;
use crate::interop::{FromVM, IntoVM};
use crate::metadata::Metadata;

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
    BoxedStruct(GcCell<'gc, IndexMap<Value<'gc>>>),
    Obj(Obj<'gc>),
    Str(Gc<'gc, String>),
    InternStr(StringType, VMIndex),
    Array(GcCell<'gc, Vec<Value<'gc>>>),
    Pinned(GcCell<'gc, Value<'gc>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Collect)]
#[collect(no_drop)]
pub enum StringType {
    String,
    Name,
    TweakDbId,
    Resource,
}

impl<'gc> Value<'gc> {
    #[inline]
    pub fn unpinned(self) -> Value<'gc> {
        match self {
            Value::Pinned(cell) => cell.read().clone(),
            other => other,
        }
    }

    #[inline]
    pub fn is_pinned(&self) -> bool {
        matches!(self, Value::Pinned(_))
    }

    pub fn to_string(&self, pool: &ConstantPool) -> String {
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
            Value::BoxedStruct(_) => todo!(),
            Value::Obj(_) => todo!(),
            Value::Str(str) => str.deref().to_owned(),
            Value::InternStr(StringType::String, idx) => pool.names.get(idx.to_pool()).unwrap().deref().to_owned(),
            Value::InternStr(StringType::Name, idx) => pool.names.get(idx.to_pool()).unwrap().deref().to_owned(),
            Value::InternStr(StringType::TweakDbId, idx) => {
                pool.tweakdb_ids.get(idx.to_pool()).unwrap().deref().to_owned()
            }
            Value::InternStr(StringType::Resource, idx) => {
                pool.resources.get(idx.to_pool()).unwrap().deref().to_owned()
            }
            Value::Array(_) => todo!(),
            Value::Pinned(v) => v.read().to_string(pool),
        }
    }

    pub fn equals(self, other: Value) -> bool {
        match (self.unpinned(), other.unpinned()) {
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
            (Value::Str(lhs), Value::Str(rhs)) => lhs.as_str() == rhs.as_str(),
            (Value::InternStr(ltyp, lidx), Value::InternStr(rtyp, ridx)) => ltyp == rtyp && lidx == ridx,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub struct PackedStruct([u8; 0xf]);

#[derive(Debug, Clone, Collect, EnumAsInner)]
#[collect(no_drop)]
pub enum Obj<'gc> {
    Null,
    Instance(GcCell<'gc, Instance<'gc>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Collect)]
#[collect(no_drop)]
pub struct VMIndex(pub u32);

impl VMIndex {
    pub const ZERO: VMIndex = VMIndex(0);

    pub fn to_pool<A>(self) -> PoolIndex<A> {
        PoolIndex::new(self.0)
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
    pub fn new<'ctx, 'pool>(
        idx: PoolIndex<Class>,
        metadata: &mut Metadata<'pool>,
        mc: MutationContext<'gc, 'ctx>,
    ) -> Self {
        let mut current = idx;
        let mut fields = IndexMap::new();
        while !current.is_undefined() {
            let class = metadata.pool().class(current).unwrap();
            for field_idx in &class.fields {
                let field = metadata.pool().field(*field_idx).unwrap();
                let typ = metadata.get_type(field.type_).unwrap();
                fields.put(*field_idx, typ.default_value(mc, metadata.pool()));
            }
            current = metadata.pool().class(current).unwrap().base;
        }
        let vtable = metadata.get_vtable(idx).unwrap();

        Self {
            tag: VMIndex(idx.index),
            fields,
            vtable,
        }
    }
}

impl<'gc> FromVM<'gc> for i32 {
    fn from_vm<'pool>(val: Value<'gc>, _pool: &'pool ConstantPool) -> Result<Self, &'static str> {
        match val.unpinned() {
            Value::I32(i) => Ok(i),
            _ => Err("Invalid argument, expected i32"),
        }
    }
}

impl<'gc> FromVM<'gc> for f32 {
    fn from_vm<'pool>(val: Value<'gc>, _pool: &'pool ConstantPool) -> Result<Self, &'static str> {
        match val.unpinned() {
            Value::F32(i) => Ok(i),
            _ => Err("Invalid argument, expected f32"),
        }
    }
}

impl<'gc> FromVM<'gc> for bool {
    fn from_vm<'pool>(val: Value<'gc>, _pool: &'pool ConstantPool) -> Result<Self, &'static str> {
        match val.unpinned() {
            Value::Bool(i) => Ok(i),
            _ => Err("Invalid argument, expected bool"),
        }
    }
}

impl<'gc> FromVM<'gc> for String {
    fn from_vm<'pool>(val: Value<'gc>, _pool: &'pool ConstantPool) -> Result<Self, &'static str> {
        match val.unpinned() {
            Value::Str(i) => Ok(i.deref().clone()),
            _ => Err("Invalid argument, expected String"),
        }
    }
}

impl<'gc> IntoVM<'gc> for i32 {
    #[inline]
    fn into_vm<'ctx>(self, _mc: MutationContext<'gc, 'ctx>) -> Value<'gc> {
        Value::I32(self)
    }
}

impl<'gc> IntoVM<'gc> for f32 {
    #[inline]
    fn into_vm<'ctx>(self, _mc: MutationContext<'gc, 'ctx>) -> Value<'gc> {
        Value::F32(self)
    }
}

impl<'gc> IntoVM<'gc> for bool {
    #[inline]
    fn into_vm<'ctx>(self, _mc: MutationContext<'gc, 'ctx>) -> Value<'gc> {
        Value::Bool(self)
    }
}

impl<'gc> IntoVM<'gc> for String {
    #[inline]
    fn into_vm<'ctx>(self, mc: MutationContext<'gc, 'ctx>) -> Value<'gc> {
        Value::Str(Gc::allocate(mc, self))
    }
}
