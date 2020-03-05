//! Implements a call frame (activation record) for the Cranelift interpreter.

use cranelift_codegen::ir::{FuncRef, Function, Value as ValueRef};
use cranelift_entity::EntityList;
use cranelift_value::Value;
use std::collections::HashMap;

/// Holds the mutable elements of an interpretation. At some point I thought about using
/// Cell/RefCell to do field-level mutability, thinking that otherwise I would have to
/// pass around a mutable object (for inst and registers) and an immutable one (for function,
/// could be self)--in the end I decided to do exactly that but perhaps one day that will become
/// untenable.
#[derive(Debug)]
pub struct Frame<'a> {
    // TODO we really should have one or the other of `func_ref` or `function`...
    /// The function reference of the currently executing function.
    pub func_ref: FuncRef,
    /// The currently executing function.
    pub function: &'a Function,
    /// The current mapping of SSA value-references to their actual values.
    registers: HashMap<ValueRef, Value>,
}

impl<'a> Frame<'a> {
    /// Construct a new frame for a function. This allocates a slot in the hash map for each SSA
    /// `Value` (renamed as `ValueRef` here) which should mean that no additional allocations are
    /// needed while interpreting the frame.
    pub fn new(func_ref: FuncRef, function: &'a Function) -> Self {
        Self {
            func_ref,
            function,
            registers: HashMap::with_capacity(function.dfg.num_values()),
        }
    }

    pub fn with_parameters(mut self, parameters: &[ValueRef], values: &[Value]) -> Self {
        for (n, v) in parameters.iter().zip(values) {
            self.registers.insert(*n, v.clone());
        }
        self
    }

    /// Retrieve the actual value associated with an SSA reference.
    #[inline]
    pub fn get(&self, name: &ValueRef) -> &Value {
        self.registers
            .get(name)
            .unwrap_or_else(|| panic!("unknown value: {}", name))
    }

    /// Retrieve multiple SSA references; see `get`.
    pub fn get_all(&self, names: &[ValueRef]) -> Vec<Value> {
        names.iter().map(|r| self.get(r)).cloned().collect()
    }

    /// Retrieve multiple SSA references from an `EntityList`; see `get`.
    pub fn get_list(&self, names: &EntityList<ValueRef>) -> Vec<Value> {
        self.get_all(names.as_slice(&self.function.dfg.value_lists))
    }

    /// Assign `value` to the SSA reference `name`.
    #[inline]
    pub fn set(&mut self, name: ValueRef, value: Value) -> Option<Value> {
        self.registers.insert(name, value)
    }

    /// Assign to multiple SSA references; see `set`.
    pub fn set_all(&mut self, names: &[ValueRef], values: Vec<Value>) {
        assert_eq!(names.len(), values.len());
        for (n, v) in names.iter().zip(values) {
            self.set(*n, v);
        }
    }

    /// Rename all of the SSA references in `old_names` to those in `new_names`. This will remove
    /// any old references that are not in `old_names`. TODO This performs an extra allocation that
    /// could be removed if we copied the values in the right order (i.e. when modifying in place,
    /// we need to avoid changing a value before it is referenced).
    pub fn rename(&mut self, old_names: &[ValueRef], new_names: &[ValueRef]) {
        let mut registers = HashMap::with_capacity(self.registers.len());
        for (on, nn) in old_names.iter().zip(new_names) {
            let v = self.registers.get(on).unwrap().clone();
            registers.insert(*nn, v);
        }
        self.registers = registers;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::InstBuilder;
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    /// Build an empty function with a single return.
    fn empty_function() -> Function {
        let mut func = Function::new();
        let mut context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        builder.ins().return_(&[]);
        func
    }

    #[test]
    fn construction() {
        let func = empty_function();
        // construction should not fail
        Frame::new(FuncRef::from_u32(0), &func);
    }

    #[test]
    fn assignment() {
        let func = empty_function();
        let mut frame = Frame::new(FuncRef::from_u32(0), &func);

        let a = ValueRef::with_number(1).unwrap();
        let fortytwo = Value::Int(42);
        frame.set(a, fortytwo.clone());
        assert_eq!(frame.get(&a), &fortytwo);
    }

    #[test]
    #[should_panic]
    fn no_existing_value() {
        let func = empty_function();
        let frame = Frame::new(FuncRef::from_u32(0), &func);

        let a = ValueRef::with_number(1).unwrap();
        frame.get(&a);
    }
}