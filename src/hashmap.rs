use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

pub struct RwAnyHashMap<Key: Hash + Eq> {
    table: UnsafeCell<HashMap<Key, Box<RwLock<dyn Any + Send + Sync>>>>,
    table_rw: RwLock<()>,
}
unsafe impl<T: Hash + Eq> Send for RwAnyHashMap<T> {}
unsafe impl<T: Hash + Eq> Sync for RwAnyHashMap<T> {}
impl<Key: Hash + Eq> RwAnyHashMap<Key> {
    pub fn new() -> Self {
        Self {
            table: UnsafeCell::new(HashMap::new()),
            table_rw: RwLock::new(()),
        }
    }
    pub fn get<T: Any + Send + Sync>(&self, key: &Key) -> Option<ElementReadHandle<T>> {
        Some(ElementReadHandle::<T>::new(
            self.table_rw.read().ok()?,
            unsafe { self.table.get().as_ref().unwrap() }
                .get(key)?
                .read()
                .ok()?,
        ))
    }
    pub fn get_mut<T: Any + Send + Sync>(&self, key: &Key) -> Option<ElementWriteHandle<T>> {
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

pub struct RwTypedHashMap(RwAnyHashMap<TypeId>);
impl RwTypedHashMap {
    pub fn new() -> Self {
        Self(RwAnyHashMap::new())
    }
    #[inline]
    pub fn get_of<T: 'static + Send + Sync>(&self) -> Option<ElementReadHandle<T>> {
        self.0.get(&TypeId::of::<T>())
    }
    #[inline]
    pub fn get_of_mut<T: 'static + Send + Sync>(&self) -> Option<ElementWriteHandle<T>> {
        self.0.get_mut(&TypeId::of::<T>())
    }
    #[inline]
    pub fn insert_of<T: 'static + Send + Sync>(&self, value: T) {
        self.0.insert(TypeId::of::<T>(), value);
    }
}

pub struct RwUserdata {
    named: RwAnyHashMap<String>,
    typed: RwTypedHashMap,
}
impl RwUserdata {
    pub fn new() -> Self {
        Self {
            named: RwAnyHashMap::<String>::new(),
            typed: RwTypedHashMap::new(),
        }
    }
    #[inline]
    pub fn get_of<T: 'static + Send + Sync>(&self) -> Option<ElementReadHandle<T>> {
        self.typed.get_of::<T>()
    }
    #[inline]
    pub fn get_of_mut<T: 'static + Send + Sync>(&self) -> Option<ElementWriteHandle<T>> {
        self.typed.get_of_mut::<T>()
    }
    #[inline]
    pub fn insert_of<T: 'static + Send + Sync>(&self, value: T) {
        self.typed.insert_of::<T>(value);
    }
    #[inline]
    pub fn get<T: 'static + Send + Sync>(
        &self,
        key: impl Into<String>,
    ) -> Option<ElementReadHandle<T>> {
        self.named.get::<T>(&key.into())
    }
    #[inline]
    pub fn get_mut<T: 'static + Send + Sync>(
        &self,
        key: impl Into<String>,
    ) -> Option<ElementWriteHandle<T>> {
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
pub struct ArcRwUserdata(Arc<RwUserdata>);
impl Deref for ArcRwUserdata {
    type Target = RwUserdata;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ArcRwUserdata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.0).unwrap()
    }
}
impl ArcRwUserdata {
    pub fn new() -> Self {
        Self(Arc::new(RwUserdata::new()))
    }
}
