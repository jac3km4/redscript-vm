use std::rc::Rc;

use redscript::bundle::ConstantPool;
use redscript::definition::{Definition, Type};

use crate::interop::{Ret, RetOut};
use crate::VM;

pub fn default_pool() -> ConstantPool {
    let mut pool = ConstantPool::default();

    let mut register_prim = |str: &str| {
        let idx = pool.names.add(Rc::new(str.to_owned()));
        pool.add_definition(Definition::type_(idx, Type::Prim));
    };

    register_prim("");
    register_prim("Int32");
    register_prim("Float");
    register_prim("String");
    register_prim("Bool");

    pool
}

#[rustfmt::skip]
pub fn register_natives(vm: &mut VM, on_log: impl Fn(String) + 'static) {
    vm.metadata_mut().register_native(
        "OperatorAdd;StringString;String", 
        |x: String, y: String| Ret(x + &y)
    );

    vm.metadata_mut().register_native(
        "OperatorAdd;Int32Int32;Int32",
        |x: i32, y: i32| Ret(x + y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignAdd;OutInt32Int32;Int32",
        |x: i32, y: i32| RetOut(x + y, x + y)
    );
    vm.metadata_mut().register_native(
        "OperatorSubtract;Int32Int32;Int32",
        |x: i32, y: i32| Ret(x - y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignSubtract;OutInt32Int32;Int32",
        |x: i32, y: i32| RetOut(x - y, x - y)
    );
    vm.metadata_mut().register_native(
        "OperatorMultiply;Int32Int32;Int32",
        |x: i32, y: i32| Ret(x * y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignMultiply;OutInt32Int32;Int32",
        |x: i32, y: i32| RetOut(x * y, x * y)
    );
    vm.metadata_mut().register_native(
        "OperatorDivide;Int32Int32;Int32",
        |x: i32, y: i32| Ret(x / y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignDivide;OutInt32Int32;Int32",
        |x: i32, y: i32| RetOut(x / y, x / y)
    );

    vm.metadata_mut().register_native(
        "OperatorEqual;Int32Int32;Bool",
        |x: i32, y: i32| Ret(x == y)
    );
    vm.metadata_mut().register_native(
        "OperatorLess;Int32Int32;Bool",
        |x: i32, y: i32| Ret(x < y)
    );
    vm.metadata_mut().register_native(
        "OperatorLessEqual;Int32Int32;Bool",
        |x: i32, y: i32| Ret(x <= y)
    );
    vm.metadata_mut().register_native(
        "OperatorGreater;Int32Int32;Bool",
        |x: i32, y: i32| Ret(x > y)
    );
    vm.metadata_mut().register_native(
        "OperatorGreaterEqual;Int32Int32;Bool",
        |x: i32, y: i32| Ret(x >= y)
    );

    vm.metadata_mut().register_native(
        "OperatorAdd;FloatFloat;Float",
        |x: f32, y: f32| Ret(x + y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignAdd;OutFloatFloat;Float",
        |x: f32, y: f32| RetOut(x + y, x + y)
    );
    vm.metadata_mut().register_native(
        "OperatorSubtract;FloatFloat;Float",
        |x: f32, y: f32| Ret(x - y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignSubtract;OutFloatFloat;Float",
        |x: f32, y: f32| RetOut(x - y, x - y)
    );
    vm.metadata_mut().register_native(
        "OperatorMultiply;FloatFloat;Float",
        |x: f32, y: f32| Ret(x * y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignMultiply;OutFloatFloat;Float",
        |x: f32, y: f32| RetOut(x * y, x * y)
    );
    vm.metadata_mut().register_native(
        "OperatorDivide;FloatFloat;Float",
        |x: f32, y: f32| Ret(x / y)
    );
    vm.metadata_mut().register_native(
        "OperatorAssignDivide;OutFloatFloat;Float",
        |x: f32, y: f32| RetOut(x / y, x / y)
    );

    vm.metadata_mut().register_native(
        "OperatorEqual;FloatFloat;Bool",
        |x: f32, y: f32| Ret(x == y)
    );
    vm.metadata_mut().register_native(
        "OperatorLess;FloatFloat;Bool",
        |x: f32, y: f32| Ret(x < y)
    );
    vm.metadata_mut().register_native(
        "OperatorLessEqual;FloatFloat;Bool",
        |x: f32, y: f32| Ret(x <= y)
    );
    vm.metadata_mut().register_native(
        "OperatorGreater;FloatFloat;Bool",
        |x: f32, y: f32| Ret(x > y)
    );
    vm.metadata_mut().register_native(
        "OperatorGreaterEqual;FloatFloat;Bool",
        |x: f32, y: f32| Ret(x >= y)
    );

    vm.metadata_mut().register_native(
        "OperatorLogicAnd;BoolBool;Bool",
        |x: bool, y: bool| Ret(x && y)
    );
    vm.metadata_mut().register_native(
        "OperatorLogicOr;BoolBool;Bool",
        |x: bool, y: bool| Ret(x || y)
    );

    vm.metadata_mut().register_native(
        "Log",
        on_log
    );
}
