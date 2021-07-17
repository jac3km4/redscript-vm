use std::rc::Rc;

use rand::Rng;
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
    register_prim("Int8");
    register_prim("Int16");
    register_prim("Int32");
    register_prim("Int64");
    register_prim("Uint8");
    register_prim("Uint16");
    register_prim("Uint32");
    register_prim("Uint64");
    register_prim("Float");
    register_prim("Double");
    register_prim("String");
    register_prim("Bool");
    register_prim("CName");
    register_prim("TweakDBID");
    register_prim("ResRef");

    pool
}

#[rustfmt::skip]
macro_rules! to_native {
    (Int8) => { i8 };
    (Int16) => { i16 };
    (Int32) => { i32 };
    (Int64) => { i64 };
    (Uint8) => { u8 };
    (Uint16) => { u16 };
    (Uint32) => { u32 };
    (Uint64) => { u64 };
    (Float) => { f32 };
    (Double) => { f64 };
    (Bool) => { bool };
}

#[rustfmt::skip]
macro_rules! impl_arithmetic {
    ( $meta:expr, $ty:ident ) => {
        $meta.register_native(
            concat!("OperatorAdd;", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x + y)
        );
        $meta.register_native(
            concat!("OperatorAssignAdd;Out", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| RetOut(x + y, x + y)
        );
        $meta.register_native(
            concat!("OperatorSubtract;", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x - y)
        );
        $meta.register_native(
            concat!("OperatorAssignSubtract;Out", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| RetOut(x - y, x - y)
        );
        $meta.register_native(
            concat!("OperatorMultiply;", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x * y)
        );
        $meta.register_native(
            concat!("OperatorAssignMultiply;Out", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| RetOut(x * y, x * y)
        );
        $meta.register_native(
            concat!("OperatorDivide;", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x / y)
        );
        $meta.register_native(
            concat!("OperatorAssignDivide;Out", stringify!($ty), stringify!($ty), ';', stringify!($ty)),
            |x: to_native!($ty), y: to_native!($ty)| RetOut(x / y, x / y)
        );
    
        $meta.register_native(
            concat!("OperatorEqual;", stringify!($ty), stringify!($ty), ';', "Bool"),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x == y)
        );
        $meta.register_native(
            concat!("OperatorLess;", stringify!($ty), stringify!($ty), ';', "Bool"),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x < y)
        );
        $meta.register_native(
            concat!("OperatorLessEqual;", stringify!($ty), stringify!($ty), ';', "Bool"),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x <= y)
        );
        $meta.register_native(
            concat!("OperatorGreater;", stringify!($ty), stringify!($ty), ';', "Bool"),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x > y)
        );
        $meta.register_native(
            concat!("OperatorGreaterEqual;", stringify!($ty), stringify!($ty), ';', "Bool"),
            |x: to_native!($ty), y: to_native!($ty)| Ret(x >= y)
        );
      
    };
}

macro_rules! impl_cast {
    ($meta:expr, $from:ident, $to:ident) => {
        $meta.register_native(
            concat!("Cast;", stringify!($from), ';', stringify!($to)),
            |x: to_native!($from)| Ret(x as to_native!($to)),
        );
    };
}

#[rustfmt::skip]
pub fn register_natives(vm: &mut VM, on_log: impl Fn(String) + 'static) {
    let meta = vm.metadata_mut();
    
    meta.register_native(
        "Log",
        on_log
    );

    meta.register_native(
        "RandRange",
        |min: i32, max: i32| {
            let res: i32 = rand::thread_rng().gen_range(min..max);
            Ret(res)
        }
    );

    meta.register_native(
        "RandF",
        || Ret(rand::random::<f32>())
    );
    meta.register_native(
        "RandRangeF",
        |min: f32, max: f32| {
            let res: f32 = rand::thread_rng().gen_range(min..max);
            Ret(res)
        }
    );
    meta.register_native(
        "SqrtF",
        |val: f32| Ret(val.sqrt())
    );
    meta.register_native(
        "LogF",
        |val: f32| Ret(val.log10())
    );
    meta.register_native(
        "CosF",
        |val: f32| Ret(val.cos())
    );

    meta.register_native(
        "OperatorAdd;StringString;String",
        |x: String, y: String| Ret(x + &y)
    );

    meta.register_native(
        "OperatorLogicAnd;BoolBool;Bool",
        |x: bool, y: bool| Ret(x && y)
    );
    meta.register_native(
        "OperatorLogicOr;BoolBool;Bool",
        |x: bool, y: bool| Ret(x || y)
    );

    impl_arithmetic!(meta, Int8);
    impl_arithmetic!(meta, Int16);
    impl_arithmetic!(meta, Int32);
    impl_arithmetic!(meta, Int64);
    impl_arithmetic!(meta, Uint8);
    impl_arithmetic!(meta, Uint16);
    impl_arithmetic!(meta, Uint32);
    impl_arithmetic!(meta, Uint64);
    impl_arithmetic!(meta, Float);
    impl_arithmetic!(meta, Double);

    impl_cast!(meta, Int8, Int16);
    impl_cast!(meta, Int8, Int32);
    impl_cast!(meta, Int8, Int64);
    impl_cast!(meta, Int8, Uint8);
    impl_cast!(meta, Int8, Uint16);
    impl_cast!(meta, Int8, Uint32);
    impl_cast!(meta, Int8, Uint64);
    impl_cast!(meta, Int8, Float);
    impl_cast!(meta, Int8, Double);

    impl_cast!(meta, Int16, Int8);
    impl_cast!(meta, Int16, Int32);
    impl_cast!(meta, Int16, Int64);
    impl_cast!(meta, Int16, Uint8);
    impl_cast!(meta, Int16, Uint16);
    impl_cast!(meta, Int16, Uint32);
    impl_cast!(meta, Int16, Uint64);
    impl_cast!(meta, Int16, Float);
    impl_cast!(meta, Int16, Double);

    impl_cast!(meta, Int32, Int8);
    impl_cast!(meta, Int32, Int16);
    impl_cast!(meta, Int32, Int64);
    impl_cast!(meta, Int32, Uint8);
    impl_cast!(meta, Int32, Uint16);
    impl_cast!(meta, Int32, Uint32);
    impl_cast!(meta, Int32, Uint64);
    impl_cast!(meta, Int32, Float);
    impl_cast!(meta, Int32, Double);

    impl_cast!(meta, Int64, Int8);
    impl_cast!(meta, Int64, Int16);
    impl_cast!(meta, Int64, Int32);
    impl_cast!(meta, Int64, Uint8);
    impl_cast!(meta, Int64, Uint16);
    impl_cast!(meta, Int64, Uint32);
    impl_cast!(meta, Int64, Uint64);
    impl_cast!(meta, Int64, Float);
    impl_cast!(meta, Int64, Double);

    impl_cast!(meta, Uint8, Int8);
    impl_cast!(meta, Uint8, Int16);
    impl_cast!(meta, Uint8, Int32);
    impl_cast!(meta, Uint8, Int64);
    impl_cast!(meta, Uint8, Uint16);
    impl_cast!(meta, Uint8, Uint32);
    impl_cast!(meta, Uint8, Uint64);
    impl_cast!(meta, Uint8, Float);
    impl_cast!(meta, Uint8, Double);

    impl_cast!(meta, Uint16, Int8);
    impl_cast!(meta, Uint16, Int16);
    impl_cast!(meta, Uint16, Int32);
    impl_cast!(meta, Uint16, Int64);
    impl_cast!(meta, Uint16, Uint8);
    impl_cast!(meta, Uint16, Uint32);
    impl_cast!(meta, Uint16, Uint64);
    impl_cast!(meta, Uint16, Float);
    impl_cast!(meta, Uint16, Double);

    impl_cast!(meta, Uint32, Int8);
    impl_cast!(meta, Uint32, Int16);
    impl_cast!(meta, Uint32, Int32);
    impl_cast!(meta, Uint32, Int64);
    impl_cast!(meta, Uint32, Uint8);
    impl_cast!(meta, Uint32, Uint16);
    impl_cast!(meta, Uint32, Uint64);
    impl_cast!(meta, Uint32, Float);
    impl_cast!(meta, Uint32, Double);

    impl_cast!(meta, Uint64, Int8);
    impl_cast!(meta, Uint64, Int16);
    impl_cast!(meta, Uint64, Int32);
    impl_cast!(meta, Uint64, Int64);
    impl_cast!(meta, Uint64, Uint8);
    impl_cast!(meta, Uint64, Uint16);
    impl_cast!(meta, Uint64, Uint32);
    impl_cast!(meta, Uint64, Float);
    impl_cast!(meta, Uint64, Double);

    impl_cast!(meta, Float, Int8);
    impl_cast!(meta, Float, Int16);
    impl_cast!(meta, Float, Int32);
    impl_cast!(meta, Float, Int64);
    impl_cast!(meta, Float, Uint8);
    impl_cast!(meta, Float, Uint16);
    impl_cast!(meta, Float, Uint32);
    impl_cast!(meta, Float, Uint64);
    impl_cast!(meta, Float, Double);

    impl_cast!(meta, Double, Int8);
    impl_cast!(meta, Double, Int16);
    impl_cast!(meta, Double, Int32);
    impl_cast!(meta, Double, Int64);
    impl_cast!(meta, Double, Uint8);
    impl_cast!(meta, Double, Uint16);
    impl_cast!(meta, Double, Uint32);
    impl_cast!(meta, Double, Uint64);
    impl_cast!(meta, Double, Float);
}
