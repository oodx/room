use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use thiserror::Error;

/// SharedState is a type-erased resource map that can be reused by different
/// runtimes. Resources are keyed by their [`TypeId`], so each type can only
/// appear once. Wrap the container in an `Arc` if you need to clone it across
/// threads.
#[derive(Clone, Default)]
pub struct SharedState {
    inner: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_arc<T>(&self, value: Arc<T>) -> Result<(), SharedStateError>
    where
        T: Send + Sync + 'static,
    {
        let mut guard = self.inner.write().map_err(|_| SharedStateError::Poisoned)?;
        let type_id = TypeId::of::<T>();
        if guard.contains_key(&type_id) {
            return Err(SharedStateError::AlreadyExists);
        }
        guard.insert(type_id, Box::new(value));
        Ok(())
    }

    pub fn get<T>(&self) -> Result<Arc<T>, SharedStateError>
    where
        T: Send + Sync + 'static,
    {
        let guard = self.inner.read().map_err(|_| SharedStateError::Poisoned)?;
        let boxed = guard
            .get(&TypeId::of::<T>())
            .ok_or(SharedStateError::Missing)?;
        let arc = boxed
            .downcast_ref::<Arc<T>>()
            .cloned()
            .ok_or(SharedStateError::TypeMismatch)?;
        Ok(arc)
    }

    pub fn get_or_insert_with<T, F>(&self, make: F) -> Result<Arc<T>, SharedStateError>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> T,
    {
        if let Ok(value) = self.get::<T>() {
            return Ok(value);
        }
        let value = Arc::new(make());
        {
            let mut guard = self.inner.write().map_err(|_| SharedStateError::Poisoned)?;
            guard
                .entry(TypeId::of::<T>())
                .or_insert_with(|| Box::new(value.clone()));
        }
        Ok(value)
    }
}

#[derive(Debug, Error)]
pub enum SharedStateError {
    #[error("resource already exists")]
    AlreadyExists,
    #[error("resource missing")]
    Missing,
    #[error("resource type mismatch")]
    TypeMismatch,
    #[error("shared state poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct Thing(u32);

    #[test]
    fn insert_and_get() {
        let state = SharedState::new();
        state.insert_arc(Arc::new(Thing(5))).unwrap();
        let value = state.get::<Thing>().unwrap();
        assert_eq!(value.0, 5);
    }

    #[test]
    fn duplicate_insert_fails() {
        let state = SharedState::new();
        state.insert_arc(Arc::new(Thing(1))).unwrap();
        let err = state.insert_arc(Arc::new(Thing(2))).unwrap_err();
        assert!(matches!(err, SharedStateError::AlreadyExists));
    }

    #[test]
    fn get_missing() {
        let state = SharedState::new();
        let err = state.get::<Thing>().unwrap_err();
        assert!(matches!(err, SharedStateError::Missing));
    }

    #[test]
    fn lazy_init() {
        let state = SharedState::new();
        let value = state.get_or_insert_with::<Thing, _>(|| Thing(9)).unwrap();
        assert_eq!(value.0, 9);
        let second = state.get::<Thing>().unwrap();
        assert_eq!(Arc::ptr_eq(&value, &second), true);
    }
}
