use crate::*;

pub fn clear(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.pop(|val, mc| val.unpinned().as_array().unwrap().write(mc).clear());
}

pub fn size(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.unop(|val, _| Value::I32(val.unpinned().as_array().unwrap().read().len() as i32));
}

pub fn resize(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.arena.mutate(|mc, root| {
        let size = root.pop(mc).unwrap().unpinned().into_i32().unwrap();
        let array = root.pop(mc).unwrap().unpinned().into_array().unwrap();
        array.write(mc).resize(size as usize, Value::Obj(Obj::Null));
    });
}

pub fn find_first(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, needle, _| {
        let cell = array.unpinned().into_array().unwrap();
        let array = cell.read();
        if let Some(res) = array.iter().cloned().find(|el| el.clone().equals(needle.clone())) {
            res
        } else {
            Value::Obj(Obj::Null)
        }
    });
}

pub fn find_last(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, needle, _| {
        let cell = array.unpinned().into_array().unwrap();
        let array = cell.read();
        if let Some(res) = array.iter().rev().cloned().find(|el| el.clone().equals(needle.clone())) {
            res
        } else {
            Value::Obj(Obj::Null)
        }
    });
}

pub fn contains(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, needle, _| {
        let cell = array.unpinned().into_array().unwrap();
        let array = cell.read();
        let exists = array.iter().any(|el| el.clone().equals(needle.clone()));
        Value::Bool(exists)
    });
}

pub fn count(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, needle, _| {
        let cell = array.unpinned().into_array().unwrap();
        let array = cell.read();
        let count = array
            .iter()
            .cloned()
            .filter(|el| el.clone().equals(needle.clone()))
            .count();
        Value::I32(count as i32)
    });
}

pub fn push(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.arena.mutate(|mc, root| {
        let value = root.pop(mc).unwrap();
        let array = root.pop(mc).unwrap().unpinned().into_array().unwrap();
        array.write(mc).push(value);
    });
}

pub fn pop(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.unop(|array, mc| {
        let cell = array.unpinned().into_array().unwrap();
        let mut array = cell.write(mc);
        array.pop().unwrap()
    });
}

pub fn insert(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.exec(frame);
    vm.arena.mutate(|mc, root| {
        let value = root.pop(mc).unwrap();
        let index = root.pop(mc).unwrap().unpinned().into_i32().unwrap();
        let array = root.pop(mc).unwrap().unpinned().into_array().unwrap();
        array.write(mc).insert(index as usize, value);
    });
}

pub fn remove(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, needle, mc| {
        let cell = array.unpinned().into_array().unwrap();
        let mut array = cell.write(mc);
        if let Some(idx) = array.iter().position(|el| el.clone().equals(needle.clone())) {
            array.remove(idx);
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    });
}

pub fn erase(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, index, mc| {
        let index = index.unpinned().into_i32().unwrap();
        let cell = array.unpinned().into_array().unwrap();
        let mut array = cell.write(mc);
        if array.get(index as usize).is_some() {
            array.remove(index as usize);
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    });
}

pub fn last(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.unop(|array, _| {
        let cell = array.unpinned().into_array().unwrap();
        let array = cell.read();
        array.last().unwrap().clone()
    });
}

pub fn element(vm: &mut VM, frame: &mut Frame) {
    vm.exec(frame);
    vm.exec(frame);
    vm.binop(|array, index, _| {
        let index = index.unpinned().into_i32().unwrap();
        let cell = array.unpinned().into_array().unwrap();
        let array = cell.read();
        array.get(index as usize).unwrap().clone()
    });
}
