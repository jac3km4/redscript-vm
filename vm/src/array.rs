use crate::error::RuntimeResult;
use crate::*;

pub fn clear(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.pop(|val, mc| val.unpinned().as_array().unwrap().borrow_mut(mc).clear());
    Ok(())
}

pub fn size(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.unop(|val, _| Value::I32(val.unpinned().as_array().unwrap().borrow().len() as i32));
    Ok(())
}

pub fn resize(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.arena.mutate(|mc, root| {
        let val = root.pop(mc).unwrap();
        let val = val.unpinned();
        let size = val
            .as_i32()
            .copied()
            .map(|i| i as u64)
            .or_else(|| val.as_u64().copied())
            .unwrap();
        let val = root.pop(mc).unwrap();
        let val = val.unpinned();
        let array = val.as_array().unwrap();
        array.borrow_mut(mc).resize(size as usize, Value::Obj(Obj::Null));
    });
    Ok(())
}

pub fn find_first(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, needle, _| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        if let Some(res) = array.borrow().iter().find(|el| el.equals(&needle)).cloned() {
            res
        } else {
            Value::Obj(Obj::Null)
        }
    });
    Ok(())
}

pub fn find_last(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, needle, _| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        if let Some(res) = array.borrow().iter().rev().find(|el| el.equals(&needle)) {
            res.clone()
        } else {
            Value::Obj(Obj::Null)
        }
    });
    Ok(())
}

pub fn contains(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, needle, _| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        let exists = array.borrow().iter().any(|el| el.equals(&needle));
        Value::Bool(exists)
    });
    Ok(())
}

pub fn count(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, needle, _| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        let count = array.borrow().iter().filter(|el| el.equals(&needle)).count();
        Value::I32(count as i32)
    });
    Ok(())
}

pub fn push(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.arena.mutate(|mc, root| {
        let val = root.pop(mc).unwrap();
        let array = root.pop(mc).unwrap();
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        array.borrow_mut(mc).push(val);
    });
    Ok(())
}

pub fn pop(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.unop(|array, mc| {
        let binding = array.unpinned();
        let array = binding.as_array().unwrap();
        array.borrow_mut(mc).pop().unwrap()
    });
    Ok(())
}

pub fn insert(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.arena.mutate(|mc, root| {
        let value = root.pop(mc).unwrap();
        let index = root.pop(mc).unwrap();
        let index = index.unpinned();
        let index = index.as_i32().unwrap();
        let array = root.pop(mc).unwrap();
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        array.borrow_mut(mc).insert(*index as usize, value);
    });
    Ok(())
}

pub fn remove(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, needle, mc| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        let mut array = array.borrow_mut(mc);
        if let Some(idx) = array.iter().position(|el| el.equals(&needle)) {
            array.remove(idx);
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    });
    Ok(())
}

pub fn erase(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, index, mc| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        let mut array = array.borrow_mut(mc);
        let index = index.unpinned();
        let index = index.as_i32().unwrap();
        if array.get(*index as usize).is_some() {
            array.remove(*index as usize);
            Value::Bool(true)
        } else {
            Value::Bool(false)
        }
    });
    Ok(())
}

pub fn last(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.unop(|array, _| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        array.borrow().last().unwrap().clone()
    });
    Ok(())
}

pub fn element(vm: &mut VM, frame: &mut Frame) -> RuntimeResult<()> {
    vm.exec(frame)?;
    vm.exec(frame)?;
    vm.binop(|array, index, _| {
        let array = array.unpinned();
        let array = array.as_array().unwrap();
        let index = index.unpinned();
        let index = index.as_i32().unwrap();
        array.borrow().get(*index as usize).unwrap().clone()
    });
    Ok(())
}
