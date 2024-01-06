use std::fmt::Debug;
use std::rc::Rc;
use std::usize;

use error::{RuntimeError, RuntimeResult};
use gc_arena::lock::{GcRefLock, RefLock};
use gc_arena::{Arena, Collect, Gc, Mutation, Rootable};
use index_map::IndexMap;
use interop::FromVM;
use metadata::Metadata;
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::bytecode::{Instr, Location, Offset};
use redscript::definition::{Function, Parameter};
use value::Value;

use crate::value::{Instance, Obj, StringType};

mod array;
pub mod error;
mod index_map;
pub mod interop;
pub mod metadata;
pub mod native;
pub mod value;

pub struct VM<'pool> {
    arena: Arena<Rootable![VMRoot<'_>]>,
    metadata: Metadata<'pool>,
}

impl<'pool> VM<'pool> {
    pub fn new(pool: &'pool ConstantPool) -> Self {
        let metadata = Metadata::new(pool);
        let arena = Arena::new(|mc| VMRoot {
            frames: GcRefLock::new(mc, Default::default()),
            stack: GcRefLock::new(mc, Default::default()),
            contexts: GcRefLock::new(mc, Default::default()),
        });
        Self { arena, metadata }
    }

    pub fn metadata(&self) -> &Metadata<'pool> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut Metadata<'pool> {
        &mut self.metadata
    }

    #[inline]
    fn push<F>(&mut self, f: F)
    where
        for<'gc> F: FnOnce(&Mutation<'gc>) -> Value<'gc>,
    {
        self.arena.mutate(|mc, root| root.push(f(mc), mc))
    }

    #[inline]
    fn pop<F, A>(&mut self, f: F) -> A
    where
        for<'gc> F: FnOnce(Value<'gc>, &Mutation<'gc>) -> A,
    {
        self.arena.mutate(|mc, root| f(root.pop(mc).unwrap(), mc))
    }

    #[inline]
    fn copy(&mut self, idx: usize) {
        self.arena.mutate(|mc, root| root.copy(idx, mc))
    }

    #[inline]
    fn unop<F>(&mut self, f: F)
    where
        for<'gc> F: FnOnce(Value<'gc>, &Mutation<'gc>) -> Value<'gc>,
    {
        self.arena.mutate(|mc, root| root.unop(f, mc))
    }

    #[inline]
    fn binop<F>(&mut self, f: F)
    where
        for<'gc> F: FnOnce(Value<'gc>, Value<'gc>, &Mutation<'gc>) -> Value<'gc>,
    {
        self.arena.mutate(|mc, root| root.binop(f, mc))
    }

    #[inline]
    fn adjust_stack(&mut self, size: usize) {
        self.arena.mutate(|mc, root| root.adjust_stack(size, mc))
    }

    fn run(&mut self, frame: &mut Frame) -> Result<bool, RuntimeError> {
        loop {
            match self.exec(frame)? {
                Action::Continue => {}
                Action::Exit => return Ok(false),
                Action::Return => return Ok(true),
            }
        }
    }

    #[inline]
    fn exec(&mut self, frame: &mut Frame) -> RuntimeResult<Action> {
        self.exec_with(frame, false)
    }

    fn exec_with(&mut self, frame: &mut Frame, pin: bool) -> RuntimeResult<Action> {
        let location = frame.location();
        let instr = match frame.next_instr() {
            Some(i) => i,
            None => return Ok(Action::Exit),
        };
        match instr {
            Instr::Nop => {}
            Instr::Null => {
                self.push(|_| Value::Obj(Obj::Null));
            }
            Instr::I32One => {
                self.push(|_| Value::I32(1));
            }
            Instr::I32Zero => {
                self.push(|_| Value::I32(0));
            }
            Instr::I8Const(val) => {
                self.push(|_| Value::I8(val));
            }
            Instr::I16Const(val) => {
                self.push(|_| Value::I16(val));
            }
            Instr::I32Const(val) => {
                self.push(|_| Value::I32(val));
            }
            Instr::I64Const(val) => {
                self.push(|_| Value::I64(val));
            }
            Instr::U8Const(val) => {
                self.push(|_| Value::U8(val));
            }
            Instr::U16Const(val) => {
                self.push(|_| Value::U16(val));
            }
            Instr::U32Const(val) => {
                self.push(|_| Value::U32(val));
            }
            Instr::U64Const(val) => {
                self.push(|_| Value::U64(val));
            }
            Instr::F32Const(val) => {
                self.push(|_| Value::F32(val));
            }
            Instr::F64Const(val) => {
                self.push(|_| Value::F64(val));
            }
            Instr::NameConst(idx) => {
                self.push(|_| Value::InternStr(StringType::Name, idx.into()));
            }
            Instr::EnumConst(_, member) => {
                let val = self.metadata.pool().enum_value(member).expect("Enum member not found");
                self.push(|_| Value::EnumVal(val));
            }
            Instr::StringConst(str) => {
                self.push(|_| Value::InternStr(StringType::String, str.into()));
            }
            Instr::TweakDbIdConst(idx) => {
                self.push(|_| Value::InternStr(StringType::TweakDbId, idx.into()));
            }
            Instr::ResourceConst(idx) => {
                self.push(|_| Value::InternStr(StringType::Resource, idx.into()));
            }
            Instr::TrueConst => {
                self.push(|_| Value::Bool(true));
            }
            Instr::FalseConst => {
                self.push(|_| Value::Bool(false));
            }
            Instr::Breakpoint(_) => todo!(),
            Instr::Assign => {
                self.assignment(frame)?;
            }
            Instr::Target(_) => todo!(),
            Instr::Local(idx) => {
                self.with_local(idx, |local, mc, root| {
                    if pin {
                        local.pin(mc);
                    }
                    root.push(local.copied(mc), mc);
                });
            }
            Instr::Param(idx) => {
                self.with_local(idx, |local, mc, root| {
                    if pin {
                        local.pin(mc);
                    }
                    root.push(local.copied(mc), mc);
                });
            }
            Instr::ObjectField(idx) => {
                self.arena.mutate(|mc, root| {
                    let contexts = root.contexts.borrow_mut(mc);
                    let context = contexts
                        .last()
                        .and_then(Obj::as_instance)
                        .ok_or(RuntimeError::NullPointer)?;
                    let mut context = context.borrow_mut(mc);
                    let val = context.fields.get_mut(idx).unwrap();
                    if pin {
                        val.pin(mc);
                    }
                    root.push(val.copied(mc), mc);
                    Ok(())
                })?;
            }
            Instr::StructField(idx) => {
                self.exec(frame)?;
                self.unop(|val, mc| match &*val.unpinned() {
                    Value::BoxedStruct(cell) => {
                        let mut val = cell.borrow_mut(mc);
                        let val = val.get_mut(idx).unwrap();
                        if pin {
                            val.pin(mc);
                        }
                        val.copied(mc)
                    }
                    Value::PackedStruct(_) => todo!(),
                    _ => panic!("invalid bytecode"),
                });
            }
            Instr::ExternalVar => todo!(),
            Instr::Switch(_, _) => {
                let sp = self.arena.mutate(|_, root| root.stack.borrow().len());
                self.exec(frame)?;
                let mut pos = frame.location().unwrap();
                while let Some(Instr::SwitchLabel(next, body)) = frame.current_instr() {
                    frame.next_instr();

                    self.copy(sp);
                    self.exec(frame)?;
                    self.binop(|lhs, rhs, _| Value::Bool(lhs.equals(&rhs)));

                    let equal = self.pop(|val, _| *val.unpinned().as_bool().unwrap());
                    if equal {
                        frame.seek(body.absolute(pos));
                        break;
                    }
                    pos = next.absolute(pos);
                    frame.seek(pos);
                }
                self.adjust_stack(sp);
            }
            Instr::SwitchLabel(_, _) => {}
            Instr::SwitchDefault => {}
            Instr::Jump(offset) => {
                frame.seek(offset.absolute(location.unwrap()));
            }
            Instr::JumpIfFalse(offset) => {
                self.exec(frame)?;
                let cond: bool = self.pop(|val, _| *val.unpinned().as_bool().unwrap());
                if !cond {
                    frame.seek(offset.absolute(location.unwrap()));
                }
            }
            Instr::Skip(_) => todo!(),
            Instr::Conditional(when_false, exit) => {
                self.exec(frame)?;
                let cond: bool = self.pop(|val, _| *val.unpinned().as_bool().unwrap());
                if !cond {
                    frame.seek(when_false.absolute(location.unwrap()));
                }
                self.exec(frame)?;
                frame.seek(exit.absolute(location.unwrap()));
            }
            Instr::Construct(args, class_idx) => {
                for _ in 0..args {
                    self.exec(frame)?;
                }
                let class = self.metadata.pool().class(class_idx).unwrap();
                let fields = class.fields.iter();

                self.arena.mutate(|mc, root| {
                    let mut stack = root.stack.borrow_mut(mc);
                    let range = (stack.len() - args as usize)..;
                    let args = stack.drain(range);
                    let data = fields.copied().zip(args).collect();
                    stack.push(Value::BoxedStruct(Gc::new(mc, RefLock::new(data))))
                });
            }
            Instr::InvokeStatic(_, _, idx, _) => {
                self.call_static(idx, frame)?;
            }
            Instr::InvokeVirtual(_, _, name, _) => {
                let tag = self.arena.mutate(|_, root| {
                    let ctx = root.contexts.borrow();
                    let inst = ctx.last().and_then(Obj::as_instance).ok_or(RuntimeError::NullPointer)?;
                    Ok(inst.borrow().tag)
                })?;
                let vtable = self.metadata.get_vtable(tag.to_pool()).unwrap();
                let idx = vtable.get(name).unwrap().to_pool();
                self.call_static(idx, frame)?;
            }
            Instr::ParamEnd => {}
            Instr::Return => {
                if !matches!(frame.current_instr(), Some(Instr::Nop)) {
                    self.exec(frame)?;
                    return Ok(Action::Return);
                } else {
                    return Ok(Action::Exit);
                }
            }
            Instr::Context(_) => {
                self.exec(frame)?;
                self.arena.mutate(|mc, root| {
                    let val = root.pop(mc).unwrap();
                    let val = val.unpinned();
                    let obj = val.as_obj().unwrap();
                    root.contexts.borrow_mut(mc).push(obj.clone());
                });
                self.exec(frame)?;
                self.arena.mutate(|mc, root| {
                    root.contexts.borrow_mut(mc).pop();
                });
            }
            Instr::Equals(_) => {
                self.exec(frame)?;
                self.exec(frame)?;
                self.binop(|lhs, rhs, _| Value::Bool(lhs.equals(&rhs)));
            }
            Instr::RefStringEqualsString(_) | Instr::StringEqualsRefString(_) => todo!(),
            Instr::NotEquals(_) => {
                self.exec(frame)?;
                self.exec(frame)?;
                self.binop(|lhs, rhs, _| Value::Bool(!lhs.equals(&rhs)));
            }
            Instr::RefStringNotEqualsString(_) | Instr::StringNotEqualsRefString(_) => todo!(),
            Instr::New(class) => {
                let meta = &mut self.metadata;
                self.arena.mutate(|mc, root| {
                    let instance = Instance::new(class, meta, mc);
                    root.push(Value::Obj(Obj::Instance(Gc::new(mc, RefLock::new(instance)))), mc);
                });
                self.check_gc();
            }
            Instr::Delete => todo!(),
            Instr::This => {
                self.arena
                    .mutate(|mc, root| root.push(Value::Obj(root.contexts.borrow().last().unwrap().clone()), mc));
            }
            Instr::StartProfiling(_) => todo!(),
            Instr::ArrayClear(_) => {
                array::clear(self, frame)?;
            }
            Instr::ArraySize(_) => {
                array::size(self, frame)?;
            }
            Instr::ArrayResize(_) => {
                array::resize(self, frame)?;
            }
            Instr::ArrayFindFirst(_) => {
                array::find_first(self, frame)?;
            }
            Instr::ArrayFindFirstFast(_) => {
                array::find_first(self, frame)?;
            }
            Instr::ArrayFindLast(_) => {
                array::find_last(self, frame)?;
            }
            Instr::ArrayFindLastFast(_) => {
                array::find_last(self, frame)?;
            }
            Instr::ArrayContains(_) => {
                array::contains(self, frame)?;
            }
            Instr::ArrayContainsFast(_) => {
                array::contains(self, frame)?;
            }
            Instr::ArrayCount(_) => {
                array::count(self, frame)?;
            }
            Instr::ArrayCountFast(_) => {
                array::count(self, frame)?;
            }
            Instr::ArrayPush(_) => {
                array::push(self, frame)?;
            }
            Instr::ArrayPop(_) => {
                array::pop(self, frame)?;
            }
            Instr::ArrayInsert(_) => {
                array::insert(self, frame)?;
            }
            Instr::ArrayRemove(_) => {
                array::remove(self, frame)?;
            }
            Instr::ArrayRemoveFast(_) => {
                array::remove(self, frame)?;
            }
            Instr::ArrayGrow(_) => {
                array::resize(self, frame)?;
            }
            Instr::ArrayErase(_) => {
                array::erase(self, frame)?;
            }
            Instr::ArrayEraseFast(_) => {
                array::erase(self, frame)?;
            }
            Instr::ArrayLast(_) => {
                array::last(self, frame)?;
            }
            Instr::ArrayElement(_) => {
                array::element(self, frame)?;
            }
            Instr::ArraySort(_) | Instr::ArraySortByPredicate(_) => todo!(),
            Instr::StaticArraySize(_) => todo!(),
            Instr::StaticArrayFindFirst(_) => todo!(),
            Instr::StaticArrayFindFirstFast(_) => todo!(),
            Instr::StaticArrayFindLast(_) => todo!(),
            Instr::StaticArrayFindLastFast(_) => todo!(),
            Instr::StaticArrayContains(_) => todo!(),
            Instr::StaticArrayContainsFast(_) => todo!(),
            Instr::StaticArrayCount(_) => todo!(),
            Instr::StaticArrayCountFast(_) => todo!(),
            Instr::StaticArrayLast(_) => todo!(),
            Instr::StaticArrayElement(_) => todo!(),
            Instr::RefToBool => {
                self.exec(frame)?;
                self.unop(|val, _| match val {
                    Value::Obj(Obj::Null) => Value::Bool(false),
                    _ => Value::Bool(true),
                })
            }
            Instr::WeakRefToBool => {
                self.exec(frame)?;
                self.unop(|val, _| match val {
                    Value::Obj(Obj::Null) => Value::Bool(false),
                    _ => Value::Bool(true),
                })
            }
            Instr::EnumToI32(_, _) => {
                self.exec(frame)?;
                self.unop(|val, _| Value::I32(*val.unpinned().as_enum_val().unwrap() as i32))
            }
            Instr::I32ToEnum(_, _) => {
                self.exec(frame)?;
                self.unop(|val, _| Value::EnumVal((*val.unpinned().as_i32().unwrap()).into()))
            }
            Instr::DynamicCast(expected, _) => {
                self.exec(frame)?;

                let meta = &self.metadata;
                self.arena.mutate(|mc, root| {
                    let mut stack = root.stack.borrow_mut(mc);
                    let val = stack.pop().unwrap();
                    let val = val.unpinned();
                    let obj = val.as_obj().unwrap();
                    let tag = obj
                        .as_instance()
                        .ok_or(RuntimeError::NullPointer)?
                        .borrow()
                        .tag
                        .to_pool();
                    let obj = if meta.is_instance_of(tag, expected) {
                        obj.clone()
                    } else {
                        Obj::Null
                    };
                    stack.push(Value::Obj(obj));
                    Ok(())
                })?;
            }
            Instr::ToString(_) | Instr::VariantToString => {
                self.exec(frame)?;
                let pool = self.metadata.pool();
                self.unop(|val, mc| Value::Str(Gc::new(mc, val.to_string(pool).into_boxed_str())));
            }
            Instr::ToVariant(_) => {
                self.exec(frame)?;
            }
            Instr::FromVariant(typ) => {
                let typ = self.metadata.get_type(typ).unwrap().clone();
                self.exec(frame)?;
                self.unop(|val, _| if val.has_type(&typ) { val } else { Value::Obj(Obj::Null) })
            }
            Instr::VariantIsDefined => {
                // TODO: actually do something
                self.exec(frame)?;
                self.unop(|_, _| Value::Bool(true))
            }
            Instr::VariantIsRef => {
                self.exec(frame)?;
                self.unop(|val, _| Value::Bool(matches!(val, Value::Obj(_))))
            }
            Instr::VariantIsArray => {
                self.exec(frame)?;
                self.unop(|val, _| Value::Bool(matches!(val, Value::Array(_))))
            }
            Instr::VariantTypeName => todo!(),
            Instr::WeakRefToRef => {}
            Instr::RefToWeakRef => {}
            Instr::WeakRefNull => {
                self.push(|_| Value::Obj(Obj::Null));
            }
            Instr::AsRef(_) => {
                self.exec(frame)?;
                self.unop(|val, mc| Value::Pinned(Gc::new(mc, RefLock::new(val))));
            }
            Instr::Deref(_) => {
                self.exec(frame)?;
                self.unop(|val, _| val.unpinned().clone());
            }
        };
        Ok(Action::Continue)
    }

    #[inline]
    pub fn call<F, A>(&mut self, idx: PoolIndex<Function>, args: F) -> RuntimeResult<A>
    where
        F: for<'gc> Fn(&Mutation<'gc>) -> Vec<Value<'gc>>,
        A: for<'gc> FromVM<'gc>,
    {
        let pool = self.metadata.pool();
        self.call_with_callback(idx, args, |res| FromVM::from_vm(res.unwrap(), pool).unwrap())
    }

    #[inline]
    pub fn call_with_callback<F, C, A>(&mut self, idx: PoolIndex<Function>, args: F, cb: C) -> RuntimeResult<A>
    where
        F: for<'gc> Fn(&Mutation<'gc>) -> Vec<Value<'gc>>,
        C: for<'gc> Fn(Option<Value<'gc>>) -> A,
    {
        self.call_void(idx, args)?;
        Ok(self.arena.mutate(|mc, root| cb(root.pop(mc))))
    }

    pub fn call_void<F>(&mut self, idx: PoolIndex<Function>, args: F) -> RuntimeResult<()>
    where
        F: for<'gc> Fn(&Mutation<'gc>) -> Vec<Value<'gc>>,
    {
        let function = self.metadata.pool().function(idx).unwrap();
        self.arena.mutate(|mc, root| {
            let args = args(mc);
            if args.len() != function.parameters.len() {
                return Err(RuntimeError::InvalidInteropParameters);
            }
            for arg in args {
                root.push(arg, mc);
            }
            Ok(())
        })?;
        self.call_with_params(idx, &function.parameters)
    }

    fn call_static(&mut self, idx: PoolIndex<Function>, frame: &mut Frame) -> RuntimeResult<()> {
        let function = self.metadata.pool().function(idx).unwrap();
        let mut indexes = Vec::with_capacity(function.parameters.len());

        for param_idx in &function.parameters {
            let param = self.metadata.pool().parameter(*param_idx).unwrap();
            if !matches!(frame.current_instr(), Some(Instr::Nop)) {
                indexes.push(*param_idx);
            }
            self.exec_with(frame, param.flags.is_out())?;
        }
        if matches!(frame.current_instr(), Some(Instr::ParamEnd)) {
            frame.skip(1);
        }
        self.call_with_params(idx, &indexes)
    }

    fn call_with_params(&mut self, idx: PoolIndex<Function>, params: &[PoolIndex<Parameter>]) -> RuntimeResult<()> {
        let function = self.metadata.pool().function(idx).unwrap();

        if function.flags.is_native() {
            self.call_native(idx)?;
            return Ok(());
        }

        let meta = &self.metadata;
        self.arena.mutate(|mc, root| {
            let mut stack = root.stack.borrow_mut(mc);
            let mut locals = IndexMap::with_capacity(function.locals.len() + params.len());

            for idx in params.iter().rev() {
                let value = stack.pop().unwrap();
                locals.put(*idx, value);
            }
            for idx in &function.locals {
                let local = meta.pool().local(*idx).unwrap();
                let typ = meta.get_type(local.type_).unwrap();
                locals.put(*idx, typ.default_value(mc, meta));
            }
            root.frames.borrow_mut(mc).push(locals);
        });

        let sp = self.arena.mutate(|_, root| root.stack.borrow().len());
        let offsets = self.metadata.get_code_offsets(idx).unwrap();

        let mut frame = Frame::new(function, offsets, sp);
        let returns = self.run(&mut frame)?;
        self.exit(&frame, returns);
        Ok(())
    }

    fn call_native(&mut self, idx: PoolIndex<Function>) -> RuntimeResult<()> {
        let Some(call) = self.metadata.get_native(idx) else {
            let name = self.metadata.pool().def_name(idx).unwrap();
            return Err(RuntimeError::UndefinedNative(name));
        };
        let pool = self.metadata.pool();

        self.arena.mutate(|mc, root| {
            if let Some(res) = call(mc, root, pool) {
                root.push(res, mc);
            }
        });
        Ok(())
    }

    fn exit(&mut self, frame: &Frame, returns: bool) {
        self.arena.mutate(|mc, root| {
            let mut stack = root.stack.borrow_mut(mc);
            if returns {
                let val = stack.pop().unwrap();
                stack.resize(frame.sp, Value::Obj(Obj::Null));
                stack.push(val);
            } else {
                stack.resize(frame.sp, Value::Obj(Obj::Null));
            }
            root.frames.borrow_mut(mc).pop();
        });
    }

    fn check_gc(&mut self) {
        if self.arena.metrics().allocation_debt() >= 64000. {
            log::debug!("GC incremental step, debt: {}", self.arena.metrics().allocation_debt());
            self.arena.collect_debt();
        }
    }

    fn assignment(&mut self, frame: &mut Frame) -> RuntimeResult<()> {
        match frame.next_instr().unwrap() {
            Instr::Local(idx) => {
                self.exec(frame)?;
                self.with_local(idx, |local, mc, root| match local {
                    Value::Pinned(inner) => *inner.borrow_mut(mc) = root.pop(mc).unwrap(),
                    val => *val = root.pop(mc).unwrap(),
                });
            }
            Instr::Param(idx) => {
                self.exec(frame)?;
                self.with_local(idx, |local, mc, root| match local {
                    Value::Pinned(inner) => *inner.borrow_mut(mc) = root.pop(mc).unwrap(),
                    val => *val = root.pop(mc).unwrap(),
                });
            }
            Instr::ObjectField(idx) => {
                self.exec(frame)?;

                self.arena.mutate(|mc, root| {
                    let instance = root.contexts.borrow_mut(mc);
                    let mut instance = instance
                        .last()
                        .and_then(Obj::as_instance)
                        .ok_or(RuntimeError::NullPointer)?
                        .borrow_mut(mc);
                    let field = instance.fields.get_mut(idx).unwrap();
                    let value = root.pop(mc).unwrap();
                    *field = value;
                    Ok(())
                })?;
            }
            Instr::StructField(idx) => {
                self.exec(frame)?;
                self.exec(frame)?;

                self.arena.mutate(|mc, root| {
                    let val = root.pop(mc).unwrap();
                    let str = root.pop(mc).unwrap();
                    match &*str.unpinned() {
                        Value::BoxedStruct(str) => str.borrow_mut(mc).put(idx, val),
                        Value::PackedStruct(_) => todo!(),
                        _ => panic!("invalid bytecode"),
                    };
                });
            }
            Instr::ArrayElement(_) => {
                self.exec(frame)?;
                self.exec(frame)?;
                self.exec(frame)?;

                self.arena.mutate(|mc, root| {
                    let val = root.pop(mc).unwrap();
                    let idx = root.pop(mc).unwrap();
                    let idx = idx
                        .as_i32()
                        .copied()
                        .map(|i| i as u64)
                        .or_else(|| idx.as_u64().copied())
                        .unwrap();
                    let array = root.pop(mc).unwrap();
                    let array = array.unpinned();
                    let array = array.as_array().unwrap();
                    array.borrow_mut(mc)[idx as usize] = val;
                });
            }
            Instr::Context(_) => {
                self.exec(frame)?;

                match frame.next_instr().unwrap() {
                    Instr::ObjectField(idx) => {
                        self.exec(frame)?;

                        self.arena.mutate(|mc, root| {
                            let val = root.pop(mc).unwrap();
                            let obj = root.pop(mc).unwrap();
                            let mut instance = obj
                                .as_obj()
                                .unwrap()
                                .as_instance()
                                .ok_or(RuntimeError::NullPointer)?
                                .borrow_mut(mc);
                            let field = instance.fields.get_mut(idx).unwrap();
                            *field = val;
                            Ok(())
                        })?;
                    }
                    _ => return Err(RuntimeError::UnsupportedAssignmentOperand),
                }
            }
            _ => return Err(RuntimeError::UnsupportedAssignmentOperand),
        };
        Ok(())
    }

    fn with_local<F, A>(&mut self, idx: PoolIndex<A>, f: F)
    where
        F: for<'gc> FnOnce(&mut Value<'gc>, &Mutation<'gc>, &VMRoot<'gc>),
    {
        self.arena.mutate(|mc, root| {
            let mut local = root.frames.borrow_mut(mc);
            let local = local.last_mut().unwrap().get_mut(idx).unwrap();
            f(local, mc, root)
        });
    }
}

#[derive(Debug)]
pub struct Frame<'pool> {
    function: &'pool Function,
    offsets: Rc<Vec<u16>>,
    ip: usize,
    sp: usize,
}

impl<'pool> Frame<'pool> {
    fn new(function: &'pool Function, offsets: Rc<Vec<u16>>, sp: usize) -> Self {
        Self {
            function,
            offsets,
            ip: 0,
            sp,
        }
    }

    #[inline]
    fn seek(&mut self, location: Location) {
        let index = self.offsets.binary_search(&location.value).unwrap();
        self.ip = index;
    }

    #[inline]
    fn skip(&mut self, n: usize) {
        self.ip += n;
    }

    #[inline]
    fn location(&self) -> Option<Location> {
        self.offsets.get(self.ip).copied().map(Location::new)
    }

    #[inline]
    fn current_instr(&self) -> Option<Instr<Offset>> {
        self.function.code.0.get(self.ip).cloned()
    }

    #[inline]
    fn next_instr(&mut self) -> Option<Instr<Offset>> {
        let instr = self.current_instr();
        self.ip += 1;
        instr
    }
}

enum Action {
    Continue,
    Exit,
    Return,
}

#[derive(Collect)]
#[collect(no_drop)]
pub struct VMRoot<'gc> {
    frames: GcRefLock<'gc, Vec<IndexMap<Value<'gc>>>>,
    stack: GcRefLock<'gc, Vec<Value<'gc>>>,
    contexts: GcRefLock<'gc, Vec<Obj<'gc>>>,
}

impl<'gc> VMRoot<'gc> {
    #[inline]
    fn pop(&self, mc: &Mutation<'gc>) -> Option<Value<'gc>> {
        self.stack.borrow_mut(mc).pop()
    }

    #[inline]
    fn push(&self, val: Value<'gc>, mc: &Mutation<'gc>) {
        self.stack.borrow_mut(mc).push(val);
    }

    #[inline]
    fn copy(&self, idx: usize, mc: &Mutation<'gc>) {
        let mut stack = self.stack.borrow_mut(mc);
        let val = stack[idx].clone();
        stack.push(val);
    }

    #[inline]
    fn unop<F>(&self, fun: F, mc: &Mutation<'gc>)
    where
        F: FnOnce(Value<'gc>, &Mutation<'gc>) -> Value<'gc>,
    {
        let mut stack = self.stack.borrow_mut(mc);
        let val = stack.pop().unwrap();
        stack.push(fun(val, mc))
    }

    #[inline]
    fn binop<F>(&self, fun: F, mc: &Mutation<'gc>)
    where
        F: FnOnce(Value<'gc>, Value<'gc>, &Mutation<'gc>) -> Value<'gc>,
    {
        let mut stack = self.stack.borrow_mut(mc);
        let rhs = stack.pop().unwrap();
        let lhs = stack.pop().unwrap();
        stack.push(fun(lhs, rhs, mc))
    }

    #[inline]
    fn adjust_stack(&self, size: usize, mc: &Mutation<'gc>) {
        let mut stack = self.stack.borrow_mut(mc);
        stack.resize(size, Value::Obj(Obj::Null));
    }
}
