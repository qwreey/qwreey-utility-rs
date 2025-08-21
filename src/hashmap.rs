use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub struct RwNamedMap<Key: Hash + Eq> {
    table: UnsafeCell<HashMap<Key, Box<RwLock<dyn Any + Send + Sync>>>>,
    table_rw: RwLock<()>,
}
unsafe impl<T: Hash + Eq> Send for RwNamedMap<T> {}
unsafe impl<T: Hash + Eq> Sync for RwNamedMap<T> {}
impl<Key: Hash + Eq> RwNamedMap<Key> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            table: UnsafeCell::new(HashMap::new()),
            table_rw: RwLock::new(()),
        }
    }
    pub fn get<T: Any + Send + Sync>(&'_ self, key: &Key) -> Option<ElementReadHandle<'_, T>> {
        Some(ElementReadHandle::<T>::new(
            self.table_rw.read().ok()?,
            unsafe { self.table.get().as_ref().unwrap() }
                .get(key)?
                .read()
                .ok()?,
        ))
    }
    pub fn get_mut<T: Any + Send + Sync>(&'_ self, key: &Key) -> Option<ElementWriteHandle<'_, T>> {
        Some(ElementWriteHandle::<T>::new(
            self.table_rw.read().ok()?,
            unsafe { self.table.get().as_ref().unwrap() }
                .get(key)?
                .write()
                .ok()?,
        ))
    }
    pub fn insert<T: Any + Send + Sync>(&self, key: Key, value: T) -> Option<()> {
        let lock = self.table_rw.write().ok()?;
        unsafe { self.table.get().as_mut().unwrap() }.insert(key, Box::new(RwLock::new(value)));
        drop(lock);
        Some(())
    }
}

pub struct RwTypedMap(RwNamedMap<TypeId>);
impl RwTypedMap {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(RwNamedMap::new())
    }
    #[inline]
    pub fn get_of<T: 'static + Send + Sync>(&'_ self) -> Option<ElementReadHandle<'_, T>> {
        self.0.get(&TypeId::of::<T>())
    }
    #[inline]
    pub fn get_of_mut<T: 'static + Send + Sync>(&'_ self) -> Option<ElementWriteHandle<'_, T>> {
        self.0.get_mut(&TypeId::of::<T>())
    }
    #[inline]
    pub fn insert_of<T: 'static + Send + Sync>(&self, value: T) {
        self.0.insert(TypeId::of::<T>(), value);
    }
}

pub struct RwMap {
    named: RwNamedMap<String>,
    typed: RwTypedMap,
}
unsafe impl Send for RwMap {}
impl RwMap {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            named: RwNamedMap::<String>::new(),
            typed: RwTypedMap::new(),
        }
    }
    #[inline]
    pub fn get_of<T: 'static + Send + Sync>(&'_ self) -> Option<ElementReadHandle<'_, T>> {
        self.typed.get_of::<T>()
    }
    #[inline]
    pub fn get_of_mut<T: 'static + Send + Sync>(&'_ self) -> Option<ElementWriteHandle<'_, T>> {
        self.typed.get_of_mut::<T>()
    }
    #[inline]
    pub fn insert_of<T: 'static + Send + Sync>(&self, value: T) {
        self.typed.insert_of::<T>(value);
    }
    #[inline]
    pub fn get<T: 'static + Send + Sync>(
        &'_ self,
        key: impl Into<String>,
    ) -> Option<ElementReadHandle<'_, T>> {
        self.named.get::<T>(&key.into())
    }
    #[inline]
    pub fn get_mut<T: 'static + Send + Sync>(
        &'_ self,
        key: impl Into<String>,
    ) -> Option<ElementWriteHandle<'_, T>> {
        self.named.get_mut::<T>(&key.into())
    }
    #[inline]
    pub fn insert<T: 'static + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.named.insert(key.into(), value);
    }
}

pub struct ElementReadHandle<'a, T> {
    data: RwLockReadGuard<'a, dyn Any + Send + Sync>,
    _table_lock: RwLockReadGuard<'a, ()>,
    _p: PhantomData<T>,
}
unsafe impl<'a, T> Send for ElementReadHandle<'a, T> {}
impl<'a, T: 'static> Deref for ElementReadHandle<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data.downcast_ref::<T>().unwrap()
    }
}
impl<'a, T> ElementReadHandle<'a, T> {
    fn new(
        table_lock: RwLockReadGuard<'a, ()>,
        data: RwLockReadGuard<'a, dyn Any + Send + Sync>,
    ) -> Self {
        Self {
            data,
            _table_lock: table_lock,
            _p: PhantomData,
        }
    }
}

pub struct ElementWriteHandle<'a, T> {
    data: RwLockWriteGuard<'a, dyn Any + Send + Sync>,
    _table_lock: RwLockReadGuard<'a, ()>,
    _p: PhantomData<T>,
}
unsafe impl<'a, T> Send for ElementWriteHandle<'a, T> {}
impl<'a, T: 'static> Deref for ElementWriteHandle<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data.downcast_ref().unwrap()
    }
}
impl<'a, T: 'static> DerefMut for ElementWriteHandle<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.downcast_mut::<T>().unwrap()
    }
}
impl<'a, T> ElementWriteHandle<'a, T> {
    fn new(
        table_lock: RwLockReadGuard<'a, ()>,
        data: RwLockWriteGuard<'a, dyn Any + Send + Sync>,
    ) -> Self {
        Self {
            data,
            _table_lock: table_lock,
            _p: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct ArcRwMap(Arc<RwMap>);
unsafe impl Send for ArcRwMap {}
impl Deref for ArcRwMap {
    type Target = RwMap;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ArcRwMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.0).unwrap()
    }
}
impl ArcRwMap {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Arc::new(RwMap::new()))
    }
}

#[macro_export]
macro_rules! write_map {
    ($map:expr, { $($key:expr => $val:expr),* $(,)? }) => {
        {
            #[allow(non_snake_case)]
            let mut WRITE_MAP_HANDLE = $map;

            $(WRITE_MAP_HANDLE.insert($key, $val);)*;

            WRITE_MAP_HANDLE
        }
    };
}
