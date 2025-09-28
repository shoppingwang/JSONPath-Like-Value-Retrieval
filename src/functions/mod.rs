use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use crate::errors::Result;

/// Trait for pluggable functions used by the expression evaluator.
pub trait Function: Send + Sync {
    fn name(&self) -> &'static str;
    fn arity(&self) -> std::ops::RangeInclusive<usize>;
    fn call(&self, args: &[Value]) -> Result<Value>;
}

/// Thread-safe function registry.
#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<HashMap<&'static str, Arc<dyn Function>>>,
}

impl Registry {
    pub fn new() -> Self { Self::default() }

    pub fn with_builtins() -> Self {
        let mut map: HashMap<&'static str, Arc<dyn Function>> = HashMap::new();
        // Lower/upper match existing engine functions; exposing as plugins too.
        map.insert("lower", Arc::new(builtins::Lower));
        map.insert("upper", Arc::new(builtins::Upper));
        map.insert("first", Arc::new(builtins::First));
        map.insert("unique", Arc::new(builtins::Unique));
        map.insert("or_default", Arc::new(builtins::OrDefault));
        map.insert("from_json", Arc::new(builtins::FromJson));
        Self { inner: Arc::new(map) }
    }

    pub fn register<F: Function + 'static>(&mut self, f: F) {
        let mut_map = Arc::make_mut(&mut self.inner);
        mut_map.insert(f.name(), Arc::new(f));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Function>> {
        self.inner.get(name).cloned()
    }
}

pub mod builtins {
    use super::*;
    use serde_json::Value;

    pub struct Lower;
    impl Function for Lower {
        fn name(&self) -> &'static str { "lower" }
        fn arity(&self) -> std::ops::RangeInclusive<usize> { 1..=1 }
        fn call(&self, args: &[Value]) -> Result<Value> {
            let s = args.get(0).cloned().unwrap_or(Value::Null);
            Ok(match s {
                Value::String(t) => Value::String(t.to_lowercase()),
                other => other,
            })
        }
    }

    pub struct Upper;
    impl Function for Upper {
        fn name(&self) -> &'static str { "upper" }
        fn arity(&self) -> std::ops::RangeInclusive<usize> { 1..=1 }
        fn call(&self, args: &[Value]) -> Result<Value> {
            let s = args.get(0).cloned().unwrap_or(Value::Null);
            Ok(match s {
                Value::String(t) => Value::String(t.to_uppercase()),
                other => other,
            })
        }
    }

    pub struct First;
    impl Function for First {
        fn name(&self) -> &'static str { "first" }
        fn arity(&self) -> std::ops::RangeInclusive<usize> { 1..=1 }
        fn call(&self, args: &[Value]) -> Result<Value> {
            Ok(crate::engine::first(args.get(0).unwrap_or(&Value::Null)))
        }
    }

    pub struct Unique;
    impl Function for Unique {
        fn name(&self) -> &'static str { "unique" }
        fn arity(&self) -> std::ops::RangeInclusive<usize> { 1..=1 }
        fn call(&self, args: &[Value]) -> Result<Value> {
            Ok(crate::engine::unique(args.get(0).unwrap_or(&Value::Null)))
        }
    }

    pub struct OrDefault;
    impl Function for OrDefault {
        fn name(&self) -> &'static str { "or_default" }
        fn arity(&self) -> std::ops::RangeInclusive<usize> { 2..=2 }
        fn call(&self, args: &[Value]) -> Result<Value> {
            let a = args.get(0).unwrap_or(&Value::Null);
            let b = args.get(1).and_then(|v| v.as_str()).unwrap_or("null");
            Ok(crate::engine::or_default(a, b))
        }
    }

    pub struct FromJson;
    impl Function for FromJson {
        fn name(&self) -> &'static str { "from_json" }
        fn arity(&self) -> std::ops::RangeInclusive<usize> { 2..=2 }
        fn call(&self, args: &[Value]) -> Result<Value> {
            let json = args.get(0).and_then(|v| v.as_str()).unwrap_or("");
            let path = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
            Ok(crate::engine::from_json(json, path))
        }
    }
}