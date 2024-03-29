use casey::lower;
use gc_arena::Mutation;
use redscript::bundle::ConstantPool;

use crate::value::Value;
use crate::VMRoot;

pub type VMFunction = dyn for<'gc> Fn(&Mutation<'gc>, &VMRoot<'gc>, &ConstantPool) -> Option<Value<'gc>>;

pub struct Ret<A>(pub A);

pub struct RetOut<A, B>(pub A, pub B);

pub trait IntoVM<'gc> {
    fn into_vm(self, mc: &Mutation<'gc>) -> Value<'gc>;
}

pub trait FromVM<'gc>: Sized {
    fn from_vm<'pool>(val: Value<'gc>, pool: &'pool ConstantPool) -> Result<Self, &'static str>;
}

pub trait IntoVMFunction<A, R> {
    fn into_vm_function(self) -> Box<VMFunction>;
}

macro_rules! impl_function_unit {
    ( [$( $types:ident ),*], [$( $locals:ident ),*] ) => {
        #[allow(unused_variables)]
        impl<$($types,)* F> IntoVMFunction<($($types,)*), ()> for F
        where
            F: Fn($($types,)*) + 'static,
            $($types: for<'gc> FromVM<'gc>,)*
        {
            fn into_vm_function(self) -> Box<VMFunction> {
                Box::new(move |mc, st, pool| {
                    $(let lower!($locals) = FromVM::from_vm(st.pop(mc).unwrap(), pool).unwrap();)*
                    self($(lower!($types),)*);
                    None
                })
            }
        }
    };
}

impl_function_unit!([], []);
impl_function_unit!([A], [a]);
impl_function_unit!([A, B], [b, a]);
impl_function_unit!([A, B, C], [c, b, a]);
impl_function_unit!([A, B, C, D], [d, c, b, a]);

macro_rules! impl_function_ret {
    ( [$( $types:ident ),*], [$( $locals:ident ),*] ) => {
        #[allow(unused_variables)]
        impl<$($types,)* R, F> IntoVMFunction<($($types,)*), Ret<R>> for F
        where
            F: Fn($($types,)*) -> Ret<R> + 'static,
            $($types: for<'gc> FromVM<'gc>,)*
            R: for<'gc> IntoVM<'gc>,
        {
            fn into_vm_function(self) -> Box<VMFunction> {
                Box::new(move |mc, st, pool| {
                    $(let lower!($locals) = FromVM::from_vm(st.pop(mc).unwrap(), pool).unwrap();)*
                    Some(self($(lower!($types),)*).0.into_vm(mc))
                })
            }
        }
    };
}

impl_function_ret!([], []);
impl_function_ret!([A], [a]);
impl_function_ret!([A, B], [b, a]);
impl_function_ret!([A, B, C], [c, b, a]);
impl_function_ret!([A, B, C, D], [d, c, b, a]);

macro_rules! impl_function_out {
    ( [ $type:ident $( ,$types:ident )*], [ $( $locals:ident ),*], $local:ident ) => {
        #[allow(unused_variables)]
        impl<$type, $($types,)* R, F> IntoVMFunction<($type, $($types,)*), RetOut<R, $type>> for F
        where
            F: Fn($type, $($types,)*) -> RetOut<R, $type> + 'static,
            $type: for<'gc> FromVM<'gc> + for<'gc> IntoVM<'gc>,
            $($types: for<'gc> FromVM<'gc>,)*
            R: for<'gc> IntoVM<'gc>,
        {
            fn into_vm_function(self) -> Box<VMFunction> {
                Box::new(move |mc, st, pool| {
                    $(let lower!($locals) = st.pop(mc).unwrap();)*
                    let $local = st.pop(mc).unwrap();
                    if let Value::Pinned(pinned) = $local {
                        let res = self(FromVM::from_vm($local, pool).unwrap(), $(FromVM::from_vm(lower!($types), pool).unwrap(),)*);
                        *pinned.borrow_mut(mc) = res.1.into_vm(mc);
                        Some(res.0.into_vm(mc))
                    } else {
                        panic!("expected a pinned value for out parameter")
                    }
                })
            }
        }
    };
}

impl_function_out!([A], [], a);
impl_function_out!([A, B], [b], a);
impl_function_out!([A, B, C], [c, b], a);
impl_function_out!([A, B, C, D], [d, c, b], a);

#[macro_export]
macro_rules! args {
    ( $( $exprs:expr ),* ) => {
       |mc| vec![$($exprs.into_vm(mc)),*]
    };
}
