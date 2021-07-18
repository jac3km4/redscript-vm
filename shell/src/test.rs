use std::cell::RefCell;
use std::ffi::OsStr;
use std::rc::Rc;

use colored::*;
use redscript::ast::Pos;
use redscript::bundle::{ConstantPool, PoolIndex};
use redscript::definition::{Function, Visibility};
use redscript::error::Error;
use redscript_compiler::source_map::Files;
use redscript_compiler::Compiler;
use redscript_vm::{args, native, VM};
use walkdir::WalkDir;

use crate::ShellConfig;

pub fn run_suite(mut pool: ConstantPool, suite: &str, config: &ShellConfig) -> Result<(), Error> {
    let mut compiler = Compiler::new(&mut pool)?;
    let sources = WalkDir::new(&config.source_dir).into_iter();
    let tests = WalkDir::new(&config.test_dir).into_iter();
    let all = sources
        .chain(tests)
        .filter_map(|e| Some(e.ok()?.into_path()).filter(|path| path.extension() == Some(OsStr::new("reds"))));
    let mut files = Files::from_files(all)?;
    files.add("stdlib.reds".into(), include_str!("test-stdlib.reds").to_owned());

    compiler.compile(&files)?;

    let mut vm = VM::new(&pool);

    let test_errors = Rc::new(RefCell::new(vec![]));
    native::register_natives(&mut vm, |str| println!("{}", str));
    register_test_natives(&mut vm, test_errors.clone());

    let class_idx = vm
        .metadata()
        .get_class(suite)
        .ok_or_else(|| Error::CompileError("Test suite not defined".to_owned(), Pos::ZERO))?;
    let class = vm.metadata().pool().class(class_idx)?;

    for fun_idx in &class.functions {
        let fun = vm.metadata().pool().function(*fun_idx)?;
        if fun.parameters.is_empty() && fun.visibility == Visibility::Public {
            run_test(&mut vm, *fun_idx, test_errors.clone())?;
        }
    }
    Ok(())
}

fn run_test(vm: &mut VM, fun_idx: PoolIndex<Function>, errors: Rc<RefCell<Vec<String>>>) -> Result<(), Error> {
    vm.call_void(fun_idx, args!());

    let name = vm.metadata().pool().definition_name(fun_idx)?;
    let pretty_name = pretty_test_name(&name);
    let mut errors = errors.borrow_mut();
    if errors.is_empty() {
        println!("{}", format!("+ {}", pretty_name).green());
    } else {
        println!("{}", format!("- {}", pretty_name).red());
        for error in errors.iter() {
            println!("{}", format!("- {}", error).red());
        }
        errors.clear();
    }
    Ok(())
}

fn pretty_test_name(name: &str) -> String {
    let mut str = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            str.push(' ');
            str.push(c.to_ascii_lowercase());
        } else if c != ';' {
            str.push(c);
        }
    }
    str
}

fn register_test_natives(vm: &mut VM, errors: Rc<RefCell<Vec<String>>>) {
    let meta = vm.metadata_mut();

    let copy = errors.clone();
    meta.register_native("FailEquality", move |a: String, b: String| {
        let msg = format!("{} is not equal to {}", a, b);
        copy.borrow_mut().push(msg);
    });
    let copy = errors.clone();
    meta.register_native("FailInequality", move |a: String, b: String| {
        let msg = format!("{} is equal to {}", a, b);
        copy.borrow_mut().push(msg);
    });
    let copy = errors.clone();
    meta.register_native("Assert", move |res: bool| {
        if !res {
            copy.borrow_mut().push("Assertion failed".to_owned());
        }
    });
}
