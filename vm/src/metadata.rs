use std::collections::HashMap;
use std::rc::Rc;

use gc_arena::lock::RefLock;
use gc_arena::{Gc, Mutation};
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::definition::{AnyDefinition, Class, Enum, Function, Type};
use redscript::Ref;

use crate::index_map::IndexMap;
use crate::interop::{IntoVMFunction, VMFunction};
use crate::value::{Obj, StringType, VMIndex, Value};

pub struct Metadata<'pool> {
    pool: &'pool ConstantPool,
    symbols: Symbols,
    types: IndexMap<TypeId>,
    function_meta: IndexMap<FunctionMetadata>,
    class_meta: IndexMap<ClassMetadata>,
}

impl<'pool> Metadata<'pool> {
    pub fn new(pool: &'pool ConstantPool) -> Self {
        let symbols = Symbols::new(pool);
        let mut types = IndexMap::new();
        let mut function_meta = IndexMap::new();
        let mut class_meta = IndexMap::new();

        for (idx, def) in pool.definitions() {
            match def.value {
                AnyDefinition::Type(_) => {
                    let id = TypeId::from(idx.cast(), pool, &symbols).expect("should resolve types");
                    types.put(idx, id);
                }
                AnyDefinition::Function(_) => {
                    function_meta.put(idx, FunctionMetadata::default());
                }
                AnyDefinition::Class(ref class) => {
                    if !class.flags.is_struct() {
                        class_meta.put(idx, ClassMetadata::default());
                    }
                }
                _ => {}
            }
        }

        Self {
            pool,
            symbols,
            types,
            function_meta,
            class_meta,
        }
    }

    #[inline]
    pub fn pool(&self) -> &'pool ConstantPool {
        self.pool
    }

    #[inline]
    pub fn get_type(&self, idx: PoolIndex<Type>) -> Option<&TypeId> {
        self.types.get(idx)
    }

    #[inline]
    pub fn get_class(&self, name: &str) -> Option<PoolIndex<Class>> {
        self.symbols.classes.get(name).copied()
    }

    #[inline]
    pub fn get_function(&self, name: &str) -> Option<PoolIndex<Function>> {
        self.symbols.functions.get(name).copied()
    }

    #[inline]
    pub fn get_native(&self, idx: PoolIndex<Function>) -> Option<&VMFunction> {
        self.function_meta.get(idx)?.native.as_ref().map(AsRef::as_ref)
    }

    #[inline]
    pub fn get_code_offsets(&mut self, idx: PoolIndex<Function>) -> Option<Rc<[u16]>> {
        let meta = self.function_meta.get_mut(idx)?;
        let fun = self.pool.function(idx).ok()?;
        Some(meta.get_offsets(fun))
    }

    #[inline]
    pub fn get_vtable(&mut self, idx: PoolIndex<Class>) -> Option<Rc<IndexMap<VMIndex>>> {
        let meta = self.class_meta.get_mut(idx)?;
        meta.get_vtable(idx, self.pool)
    }

    pub fn register_native<F: IntoVMFunction<A, R>, A, R>(&mut self, name: &str, function: F) -> Option<()> {
        self.set_native_function(name, function.into_vm_function())
    }

    fn set_native_function(&mut self, name: &str, function: Box<VMFunction>) -> Option<()> {
        let idx = self.get_function(name)?;
        let meta = self.function_meta.get_mut(idx)?;
        meta.native = Some(function);
        Some(())
    }

    pub fn is_instance_of(&self, instance: PoolIndex<Class>, of: PoolIndex<Class>) -> bool {
        let mut expected = of;
        loop {
            let class = self.pool.class(expected).expect("should resolve classes");
            if instance == expected {
                break true;
            } else if class.base.is_undefined() {
                break false;
            };
            expected = class.base;
        }
    }
}

struct Symbols {
    functions: HashMap<Ref<str>, PoolIndex<Function>>,
    classes: HashMap<Ref<str>, PoolIndex<Class>>,
    enums: HashMap<Ref<str>, PoolIndex<Enum>>,
}

impl Symbols {
    fn new(pool: &ConstantPool) -> Self {
        let mut functions = HashMap::new();
        let mut classes = HashMap::new();
        let mut enums = HashMap::new();

        for (idx, def) in pool.roots() {
            match def.value {
                AnyDefinition::Class(_) => {
                    classes.insert(pool.names.get(def.name).unwrap(), idx.cast());
                }
                AnyDefinition::Enum(_) => {
                    enums.insert(pool.names.get(def.name).unwrap(), idx.cast());
                }
                AnyDefinition::Function(_) => {
                    functions.insert(pool.names.get(def.name).unwrap(), idx.cast());
                }
                _ => {}
            }
        }

        Symbols {
            functions,
            classes,
            enums,
        }
    }
}

#[derive(Debug, Default)]
struct ClassMetadata {
    vtable: Option<Rc<IndexMap<VMIndex>>>,
}

impl ClassMetadata {
    fn get_vtable(&mut self, idx: PoolIndex<Class>, pool: &ConstantPool) -> Option<Rc<IndexMap<VMIndex>>> {
        match &self.vtable {
            Some(rc) => Some(rc.clone()),
            None => {
                let mut current = idx;
                let mut bases = vec![];
                while !current.is_undefined() {
                    bases.push(current);
                    current = pool.class(current).ok()?.base;
                }

                let mut vtable = IndexMap::new();
                for class_idx in bases.into_iter() {
                    let class = pool.class(class_idx).ok()?;
                    for fun_idx in &class.functions {
                        let def = pool.definition(*fun_idx).ok()?;
                        let fun = pool.function(*fun_idx).ok()?;
                        if !fun.flags.is_final() && !fun.flags.is_static() {
                            vtable.put(def.name, (*fun_idx).into());
                        }
                    }
                }
                let rc = Rc::new(vtable);
                self.vtable = Some(rc.clone());
                Some(rc)
            }
        }
    }
}

#[derive(Default)]
struct FunctionMetadata {
    offsets: Option<Rc<[u16]>>,
    native: Option<Box<VMFunction>>,
}

impl FunctionMetadata {
    fn get_offsets(&mut self, function: &Function) -> Rc<[u16]> {
        match &self.offsets {
            Some(offsets) => offsets.clone(),
            None => {
                let code = &function.code;
                let offsets: Rc<[u16]> = code.iter().map(|(loc, _)| loc.value).collect();
                self.offsets = Some(offsets.clone());
                offsets
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeId {
    I64,
    I32,
    I16,
    I8,
    U64,
    U32,
    U16,
    U8,
    F64,
    F32,
    Bool,
    String,
    CName,
    TweakDbId,
    ResRef,
    Variant,
    NodeRef,
    CRUID,
    Ref(PoolIndex<Class>),
    WRef(PoolIndex<Class>),
    ScriptRef(Box<TypeId>),
    Enum(PoolIndex<Enum>),
    Struct(PoolIndex<Class>),
    Array(Box<TypeId>),
    StaticArray(Box<TypeId>, u32),
}

impl TypeId {
    pub fn default_value<'gc>(&self, mc: &Mutation<'gc>, meta: &Metadata<'_>) -> Value<'gc> {
        match self {
            TypeId::I64 => Value::I64(0),
            TypeId::I32 => Value::I32(0),
            TypeId::I16 => Value::I16(0),
            TypeId::I8 => Value::I8(0),
            TypeId::U64 => Value::U64(0),
            TypeId::U32 => Value::U32(0),
            TypeId::U16 => Value::U16(0),
            TypeId::U8 => Value::U8(0),
            TypeId::F64 => Value::F64(0.),
            TypeId::F32 => Value::F32(0.),
            TypeId::Bool => Value::Bool(false),
            TypeId::String => Value::InternStr(StringType::String, VMIndex::ZERO),
            TypeId::CName => Value::InternStr(StringType::Name, VMIndex::ZERO),
            TypeId::TweakDbId => Value::InternStr(StringType::TweakDbId, VMIndex::ZERO),
            TypeId::ResRef => Value::InternStr(StringType::Resource, VMIndex::ZERO),
            TypeId::Variant => Value::Obj(Obj::Null),
            TypeId::NodeRef => todo!(),
            TypeId::CRUID => todo!(),
            TypeId::Ref(_) => Value::Obj(Obj::Null),
            TypeId::WRef(_) => Value::Obj(Obj::Null),
            TypeId::ScriptRef(_) => todo!(),
            TypeId::Enum(_) => Value::EnumVal(0),
            TypeId::Struct(class_idx) => {
                let class = meta.pool().class(*class_idx).expect("should resolve classes");
                let fields = class.fields.iter().copied();
                let values = fields.clone().map(|field_idx| {
                    let field = meta.pool().field(field_idx).expect("should resolve fields");
                    let typ = meta.get_type(field.type_).expect("should resolve types");
                    typ.default_value(mc, meta)
                });
                Value::BoxedStruct(Gc::new(mc, RefLock::new(fields.zip(values).collect())))
            }
            TypeId::Array(_) => Value::Array(Gc::new(mc, RefLock::default())),
            TypeId::StaticArray(_, _) => todo!(),
        }
    }

    fn from(idx: PoolIndex<Type>, pool: &ConstantPool, symbols: &Symbols) -> Option<TypeId> {
        let typ = pool.type_(idx).ok()?;
        match typ {
            Type::Prim => {
                let name = pool.def_name(idx).ok()?;
                let res = match &*name {
                    "Int64" => TypeId::I64,
                    "Int32" => TypeId::I32,
                    "Int16" => TypeId::I16,
                    "Int8" => TypeId::I8,
                    "Uint64" => TypeId::U64,
                    "Uint32" => TypeId::U32,
                    "Uint16" => TypeId::U16,
                    "Uint8" => TypeId::U8,
                    "Double" => TypeId::F64,
                    "Float" => TypeId::F32,
                    "Bool" => TypeId::Bool,
                    "String" => TypeId::String,
                    "CName" => TypeId::CName,
                    "TweakDBID" => TypeId::TweakDbId,
                    "Variant" => TypeId::Variant,
                    "NodeRef" => TypeId::NodeRef,
                    "LocalizationString" => TypeId::String,
                    "CRUID" => TypeId::CRUID,
                    "CRUIDRef" => TypeId::CRUID,
                    "redResourceReferenceScriptToken" => TypeId::String,
                    "ResRef" => TypeId::ResRef,
                    _ => return None,
                };
                Some(res)
            }
            Type::Class => {
                let name = pool.def_name(idx).ok()?;
                symbols
                    .classes
                    .get(&name)
                    .map(|idx| TypeId::Struct(*idx))
                    .or_else(|| symbols.enums.get(&name).map(|idx| TypeId::Enum(*idx)))
            }
            Type::Ref(typ) => {
                let name = pool.def_name(*typ).ok()?;
                let class = symbols.classes.get(&name)?;
                Some(TypeId::Ref(*class))
            }
            Type::WeakRef(typ) => {
                let name = pool.def_name(*typ).ok()?;
                let class = symbols.classes.get(&name)?;
                Some(TypeId::WRef(*class))
            }
            Type::ScriptRef(inner) => {
                let inner = TypeId::from(*inner, pool, symbols)?;
                Some(TypeId::ScriptRef(Box::new(inner)))
            }
            Type::Array(inner) => {
                let inner = TypeId::from(*inner, pool, symbols)?;
                Some(TypeId::Array(Box::new(inner)))
            }
            Type::StaticArray(inner, size) => {
                let inner = TypeId::from(*inner, pool, symbols)?;
                Some(TypeId::StaticArray(Box::new(inner), *size))
            }
        }
    }
}
