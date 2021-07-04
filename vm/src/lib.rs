use std::fmt::Debug;
use std::rc::Rc;
use std::usize;

use gc_arena::{make_arena, ArenaParameters, Collect, Gc, GcCell, MutationContext};
use index_map::IndexMap;
use metadata::Metadata;
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::bytecode::{Instr, Location, Offset};
use redscript::definition::{Function, Parameter};
use value::Value;

use crate::interop::FromVM;
use crate::value::{Instance, Obj};

pub mod index_map;
pub mod interop;
pub mod metadata;
pub mod native;
pub mod value;

pub struct VM<'pool> {
    arena: RootArena,
    metadata: Metadata<'pool>,
}

impl<'pool> VM<'pool> {
    pub fn new(pool: &'pool ConstantPool) -> Self {
        let metadata = Metadata::new(pool);
        let arena = RootArena::new(ArenaParameters::default(), |mc| VMRoot {
            frames: GcCell::allocate(mc, vec![]),
            stack: GcCell::allocate(mc, vec![]),
            contexts: GcCell::allocate(mc, vec![]),
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
        for<'gc, 'ctx> F: FnOnce(MutationContext<'gc, 'ctx>) -> Value<'gc>,
    {
        self.arena.mutate(|mc, root| root.push(f(mc), mc))
    }

    #[inline]
    fn pop<F, A>(&mut self, f: F) -> A
    where
        for<'gc, 'ctx> F: FnOnce(MutationContext<'gc, 'ctx>, Value<'gc>) -> A,
    {
        self.arena.mutate(|mc, root| f(mc, root.pop(mc).unwrap()))
    }

    #[inline]
    fn swap<F>(&mut self, f: F)
    where
        for<'gc, 'ctx> F: FnOnce(MutationContext<'gc, 'ctx>, Value<'gc>) -> Value<'gc>,
    {
        self.arena.mutate(|mc, root| root.swap(|val| f(mc, val), mc))
    }

    fn run(&mut self, frame: &mut Frame) -> bool {
        loop {
            match self.exec(frame) {
                Action::Continue => {}
                Action::Exit => return false,
                Action::Return => return true,
            }
        }
    }

    #[inline]
    fn exec(&mut self, frame: &mut Frame) -> Action {
        self.exec_with(frame, false)
    }

    fn exec_with(&mut self, frame: &mut Frame, pin: bool) -> Action {
        let location = frame.location();
        let instr = match frame.next_instr() {
            Some(i) => i,
            None => return Action::Exit,
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
            Instr::NameConst(_) => todo!(),
            Instr::EnumConst(_, member) => {
                let val = self.metadata.pool().enum_value(member).expect("Enum member not found");
                self.push(|_| Value::EnumVal(val));
            }
            Instr::StringConst(str) => {
                self.push(|mc| Value::Str(Gc::allocate(mc, str)));
            }
            Instr::TweakDbIdConst(_) => todo!(),
            Instr::ResourceConst(_) => todo!(),
            Instr::TrueConst => {
                self.push(|_| Value::Bool(true));
            }
            Instr::FalseConst => {
                self.push(|_| Value::Bool(false));
            }
            Instr::Breakpoint(_, _, _, _, _, _) => todo!(),
            Instr::Assign => {
                self.assign(frame);
            }
            Instr::Target(_) => todo!(),
            Instr::Local(idx) => {
                self.arena.mutate(|mc, root| {
                    let mut frames = root.frames.write(mc);
                    let local = frames.last_mut().unwrap().get_mut(idx).unwrap();
                    if pin && !local.is_pinned() {
                        let pinned = Value::Pinned(GcCell::allocate(mc, local.clone()));
                        *local = pinned.clone();
                        root.push(pinned, mc);
                    } else {
                        root.push(local.clone(), mc);
                    }
                });
            }
            Instr::Param(idx) => {
                self.arena.mutate(|mc, root| {
                    let value = root.local(idx).unwrap().clone();
                    root.push(value, mc);
                });
            }
            Instr::ObjectField(idx) => {
                self.arena.mutate(|mc, root| {
                    let ctx = root.contexts.read();
                    let instance = ctx.last().unwrap().as_instance().unwrap();
                    let val = instance.read().fields.get(idx).unwrap().clone();
                    root.push(val, mc)
                });
            }
            Instr::StructField(idx) => {
                self.exec(frame);
                self.swap(|_, v| match v {
                    Value::BoxedStruct(cell) => cell.read().get(idx).unwrap().clone(),
                    _ => todo!(),
                });
            }
            Instr::ExternalVar => todo!(),
            Instr::Switch(_, _) => todo!(),
            Instr::SwitchLabel(_, _) => todo!(),
            Instr::SwitchDefault => todo!(),
            Instr::Jump(offset) => {
                frame.seek(offset.absolute(location.unwrap()));
            }
            Instr::JumpIfFalse(offset) => {
                self.exec(frame);
                let pool = self.metadata.pool();
                let cond: bool = self.pop(|_, v| FromVM::from_vm(v, pool).unwrap());
                if !cond {
                    frame.seek(offset.absolute(location.unwrap()));
                }
            }
            Instr::Skip(_) => todo!(),
            Instr::Conditional(_, _) => todo!(),
            Instr::Construct(_, _) => todo!(),
            Instr::InvokeStatic(_, _, idx) => {
                self.call_static(idx, frame);
            }
            Instr::InvokeVirtual(_, _, name) => {
                let tag = self
                    .arena
                    .mutate(|_, root| root.contexts.read().last().unwrap().as_instance().unwrap().read().tag);
                let idx = self.metadata.get_vtable(tag.to_pool()).unwrap();
                let idx = PoolIndex::new(idx.get(name).unwrap().0);
                self.call_static(idx, frame);
            }
            Instr::ParamEnd => {}
            Instr::Return => {
                if !matches!(frame.current_instr(), Some(Instr::Nop)) {
                    self.exec(frame);
                    return Action::Return;
                } else {
                    return Action::Exit;
                }
            }
            Instr::Context(_) => {
                self.exec(frame);
                self.arena.mutate(|mc, root| {
                    let obj = root.pop(mc).unwrap().into_obj().unwrap();
                    root.contexts.write(mc).push(obj)
                });
                self.exec(frame);
                self.arena.mutate(|mc, root| {
                    root.contexts.write(mc).pop();
                });
            }
            Instr::Equals(_) => todo!(),
            Instr::NotEquals(_) => todo!(),
            Instr::New(class) => {
                let meta = &mut self.metadata;
                self.arena.mutate(move |mc, root| {
                    let instance = Instance::new(class, meta, mc);
                    root.push(Value::Obj(Obj::Instance(GcCell::allocate(mc, instance))), mc);
                });
                self.check_gc();
            }
            Instr::Delete => todo!(),
            Instr::This => self.arena.mutate(|mc, root| {
                let obj = root.contexts.read();
                root.push(Value::Obj(obj.last().unwrap().clone()), mc)
            }),
            Instr::StartProfiling(_, _) => todo!(),
            Instr::ArrayClear(_) => {
                self.exec(frame);
                self.pop(|mc, val| val.as_array().unwrap().write(mc).clear());
            }
            Instr::ArraySize(_) => {
                self.exec(frame);
                self.swap(|_, val| Value::I32(val.as_array().unwrap().read().len() as i32));
            }
            Instr::ArrayResize(_) => todo!(),
            Instr::ArrayFindFirst(_) => todo!(),
            Instr::ArrayFindFirstFast(_) => todo!(),
            Instr::ArrayFindLast(_) => todo!(),
            Instr::ArrayFindLastFast(_) => todo!(),
            Instr::ArrayContains(_) => todo!(),
            Instr::ArrayContainsFast(_) => todo!(),
            Instr::ArrayCount(_) => todo!(),
            Instr::ArrayCountFast(_) => todo!(),
            Instr::ArrayPush(_) => {
                self.exec(frame);
                self.exec(frame);
                self.arena.mutate(|mc, root| {
                    let value = root.pop(mc).unwrap();
                    let array = root.pop(mc).unwrap();
                    array.as_array().unwrap().write(mc).push(value);
                });
            }
            Instr::ArrayPop(_) => todo!(),
            Instr::ArrayInsert(_) => todo!(),
            Instr::ArrayRemove(_) => todo!(),
            Instr::ArrayRemoveFast(_) => todo!(),
            Instr::ArrayGrow(_) => todo!(),
            Instr::ArrayErase(_) => todo!(),
            Instr::ArrayEraseFast(_) => todo!(),
            Instr::ArrayLast(_) => todo!(),
            Instr::ArrayElement(_) => {
                self.exec(frame);
                self.exec(frame);
                let pool = self.metadata.pool();
                self.arena.mutate(|mc, root| {
                    let index: i32 = FromVM::from_vm(root.pop(mc).unwrap(), pool).unwrap();
                    let array = root.pop(mc).unwrap();
                    let elem = array.as_array().unwrap().read().get(index as usize).unwrap().clone();
                    root.push(elem, mc);
                });
            }
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
                self.exec(frame);
                self.swap(|_, val| match val {
                    Value::Obj(Obj::Null) => Value::Bool(false),
                    _ => Value::Bool(true),
                })
            }
            Instr::WeakRefToBool => {
                self.exec(frame);
                self.swap(|_, val| match val {
                    Value::Obj(Obj::Null) => Value::Bool(false),
                    _ => Value::Bool(true),
                })
            }
            Instr::EnumToI32(_, _) => {
                self.exec(frame);
                self.swap(|_, val| Value::I32(val.into_enum_val().unwrap() as i32))
            }
            Instr::I32ToEnum(_, _) => {
                self.exec(frame);
                let pool = self.metadata.pool();
                self.swap(|_, val| {
                    let v: i32 = FromVM::from_vm(val, pool).unwrap();
                    Value::EnumVal(v.into())
                })
            }
            Instr::DynamicCast(_, _) => todo!(),
            Instr::ToString(_) => {
                self.exec(frame);
                self.swap(|mc, val| Value::Str(Gc::allocate(mc, val.to_string())));
            }
            Instr::ToVariant(_) => todo!(),
            Instr::FromVariant(_) => todo!(),
            Instr::VariantIsValid => todo!(),
            Instr::VariantIsRef => todo!(),
            Instr::VariantIsArray => todo!(),
            Instr::VatiantToCName => todo!(),
            Instr::VariantToString => todo!(),
            Instr::WeakRefToRef => {}
            Instr::RefToWeakRef => {}
            Instr::WeakRefNull => {
                self.push(|_| Value::Obj(Obj::Null));
            }
            Instr::AsRef(_) => todo!(),
            Instr::Deref(_) => todo!(),
        };
        Action::Continue
    }

    pub fn call_with(&mut self, idx: PoolIndex<Function>, params: &[PoolIndex<Parameter>]) {
        let function = self.metadata.pool().function(idx).unwrap();

        if function.flags.is_native() {
            let call = self
                .metadata
                .get_native(idx)
                .unwrap_or_else(|| panic!("Native {} is not defined", idx.index));
            let pool = self.metadata.pool();

            self.arena.mutate(|mc, root| {
                if let Some(res) = call(mc, root, pool) {
                    root.push(res, mc);
                }
            });
            return;
        }

        let meta = &self.metadata;
        self.arena.mutate(|mc, root| {
            let mut stack = root.stack.write(mc);
            let mut locals = IndexMap::with_capacity(function.locals.len() + params.len());

            for idx in params.iter().rev() {
                let value = stack.pop().unwrap();
                locals.put(*idx, value);
            }
            for idx in &function.locals {
                let local = meta.pool().local(*idx).unwrap();
                let typ = meta.get_type(local.type_).unwrap();
                locals.put(*idx, typ.default_value(mc, meta.pool()));
            }
            root.frames.write(mc).push(locals);
        });

        let sp = self.arena.mutate(|_, root| root.stack.read().len());
        let offsets = self.metadata.get_offsets(idx).unwrap();

        let mut frame = Frame::new(function, offsets, sp);
        let returns = self.run(&mut frame);
        self.exit(&frame, returns);
    }

    fn call_static(&mut self, idx: PoolIndex<Function>, frame: &mut Frame) {
        let function = self.metadata.pool().function(idx).unwrap();
        let mut indexes = Vec::with_capacity(function.parameters.len());

        for param_idx in &function.parameters {
            let param = self.metadata.pool().parameter(*param_idx).unwrap();
            if !matches!(frame.current_instr(), Some(Instr::Nop)) {
                indexes.push(*param_idx);
            }
            self.exec_with(frame, param.flags.is_out());
        }
        if matches!(frame.current_instr(), Some(Instr::ParamEnd)) {
            frame.skip(1);
        }
        self.call_with(idx, &indexes);
    }

    pub fn pretty_result(&mut self) -> Option<String> {
        self.arena
            .mutate(|_, root| root.stack.read().last().map(Value::to_string))
    }

    fn exit(&mut self, frame: &Frame, returns: bool) {
        self.arena.mutate(|mc, root| {
            let mut stack = root.stack.write(mc);
            if returns {
                let val = stack.pop().unwrap();
                stack.resize(frame.sp, Value::Obj(Obj::Null));
                stack.push(val);
            } else {
                stack.resize(frame.sp, Value::Obj(Obj::Null));
            }
            root.frames.write(mc).pop();
        });
    }

    fn check_gc(&mut self) {
        if self.arena.allocation_debt() >= 64000. {
            log::debug!("GC incremental step, debt: {}", self.arena.allocation_debt());
            self.arena.collect_debt();
        }
    }

    fn assign(&mut self, frame: &mut Frame) {
        match frame.next_instr().unwrap() {
            Instr::Local(idx) => {
                self.exec(frame);

                self.arena.mutate(|mc, root| {
                    let mut frames = root.frames.write(mc);
                    let local = frames.last_mut().unwrap().get_mut(idx).unwrap();
                    let value = root.pop(mc).unwrap();
                    *local = value;
                });
            }
            Instr::ObjectField(idx) => {
                self.exec(frame);

                self.arena.mutate(|mc, root| {
                    let ctx = root.contexts.read();
                    let mut instance = ctx.last().unwrap().as_instance().unwrap().write(mc);
                    let field = instance.fields.get_mut(idx).unwrap();
                    let value = root.pop(mc).unwrap();
                    *field = value;
                });
            }
            Instr::Context(_) => {
                self.exec(frame);

                match frame.next_instr().unwrap() {
                    Instr::ObjectField(idx) => {
                        self.exec(frame);

                        self.arena.mutate(|mc, root| {
                            let value = root.pop(mc).unwrap();
                            let obj = root.pop(mc).unwrap();
                            let mut instance = obj.as_obj().unwrap().as_instance().unwrap().write(mc);
                            let field = instance.fields.get_mut(idx).unwrap();
                            *field = value;
                        });
                    }
                    _ => panic!("Unexpected assign instruction"),
                }
            }
            _ => panic!("Unexpected assign instruction"),
        }
    }
}

#[derive(Debug)]
struct Frame<'pool> {
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
    frames: GcCell<'gc, Vec<IndexMap<Value<'gc>>>>,
    stack: GcCell<'gc, Vec<Value<'gc>>>,
    contexts: GcCell<'gc, Vec<Obj<'gc>>>,
}

impl<'gc> VMRoot<'gc> {
    #[inline]
    fn local<A>(&self, idx: PoolIndex<A>) -> Option<Value<'gc>> {
        let val = self.frames.read();
        val.last()?.get(idx).cloned()
    }

    #[inline]
    fn pop<'ctx>(&self, mc: MutationContext<'gc, 'ctx>) -> Option<Value<'gc>> {
        self.stack.write(mc).pop()
    }

    #[inline]
    fn push<'ctx>(&self, val: Value<'gc>, mc: MutationContext<'gc, 'ctx>) {
        self.stack.write(mc).push(val)
    }

    #[inline]
    fn swap<'ctx, F: FnOnce(Value<'gc>) -> Value<'gc>>(&self, fun: F, mc: MutationContext<'gc, 'ctx>) {
        let mut stack = self.stack.write(mc);
        let val = stack.pop().unwrap();
        stack.push(fun(val))
    }
}

make_arena!(RootArena, VMRoot);
