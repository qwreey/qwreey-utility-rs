use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

struct MapField<T> {
    data: T,
    lock: RwLock<()>,
}
impl<T> MapField<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            lock: RwLock::new(()),
        }
    }
}

pub struct RwHashMap<Key: Hash + Eq> {
    #[allow(clippy::type_complexity)]
    table: UnsafeCell<HashMap<Key, MapField<Box<UnsafeCell<dyn Any + Send>>>>>,
    table_rw: RwLock<()>,
}
unsafe impl<T: Hash + Eq> Send for RwHashMap<T> {}
unsafe impl<T: Hash + Eq> Sync for RwHashMap<T> {}
impl<Key: Hash + Eq> RwHashMap<Key> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            table: UnsafeCell::new(HashMap::new()),
            table_rw: RwLock::new(()),
        }
    }
    pub fn get<T: 'static + Any + Send>(&'_ self, key: &Key) -> Option<MapReader<'_, T>> {
        let field = unsafe { self.table.get().as_ref().unwrap() }.get(key)?;

        Some(MapReader::<T>::new(
            self.table_rw.read().ok()?,
            field.lock.read().ok()?,
            unsafe { field.data.get().as_ref().unwrap() },
        ))
    }
    pub fn get_mut<T: 'static + Any + Send>(&'_ self, key: &Key) -> Option<MapWriter<'_, T>> {
        let field = unsafe { self.table.get().as_ref().unwrap() }.get(key)?;

        Some(MapWriter::<T>::new(
            self.table_rw.read().ok()?,
            field.lock.write().ok()?,
            unsafe { field.data.get().as_mut().unwrap() },
        ))
    }
    pub fn insert<T: 'static + Any + Send>(&self, key: Key, value: T) -> Option<()> {
        let lock = self.table_rw.write().ok()?;
        unsafe { self.table.get().as_mut().unwrap() }
            .insert(key, MapField::new(Box::new(UnsafeCell::new(value))));
        drop(lock);
        Some(())
    }
    /// # Safety
    /// This function does not lock the table or the field, so it is up to the caller to ensure
    /// that no other threads are accessing the table or the field while this function is being
    /// called.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut_nonlock<T: 'static + Any + Send>(
        &'_ self,
        key: &Key,
    ) -> Option<&'_ mut T> {
        let field = unsafe { self.table.get().as_ref().unwrap() }.get(key)?;

        field.data.get().as_mut().unwrap().downcast_mut::<T>()
    }
}

pub struct RwTypedMap(RwHashMap<TypeId>);
unsafe impl Send for RwTypedMap {}
unsafe impl Sync for RwTypedMap {}
impl RwTypedMap {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(RwHashMap::new())
    }
    #[inline]
    pub fn get_of<T: 'static + Send>(&'_ self) -> Option<MapReader<'_, T>> {
        self.0.get(&TypeId::of::<T>())
    }
    #[inline]
    pub fn get_of_mut<T: 'static + Send>(&'_ self) -> Option<MapWriter<'_, T>> {
        self.0.get_mut(&TypeId::of::<T>())
    }
    /// # Safety
    /// This function does not lock the table or the field, so it is up to the caller to ensure
    /// that no other threads are accessing the table or the field while this function is being
    /// called.
    #[allow(clippy::mut_from_ref)]
    #[inline]
    pub unsafe fn get_of_mut_nonlock<T: 'static + Send>(&'_ self) -> Option<&'_ mut T> {
        self.0.get_mut_nonlock(&TypeId::of::<T>())
    }
    #[inline]
    pub fn insert_of<T: 'static + Send>(&self, value: T) {
        self.0.insert(TypeId::of::<T>(), value);
    }
}

pub struct RwMap {
    named: RwHashMap<String>,
    typed: RwTypedMap,
}
unsafe impl Send for RwMap {}
unsafe impl Sync for RwMap {}
impl RwMap {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            named: RwHashMap::<String>::new(),
            typed: RwTypedMap::new(),
        }
    }
    #[inline]
    pub fn get_of<T: 'static + Send + Sync>(&'_ self) -> Option<MapReader<'_, T>> {
        self.typed.get_of::<T>()
    }
    #[inline]
    pub fn get_of_mut<T: 'static + Send + Sync>(&'_ self) -> Option<MapWriter<'_, T>> {
        self.typed.get_of_mut::<T>()
    }
    /// # Safety
    /// This function does not lock the table or the field, so it is up to the caller to ensure
    /// that no other threads are accessing the table or the field while this function is being
    /// called.
    #[allow(clippy::mut_from_ref)]
    #[inline]
    pub unsafe fn get_of_mut_nonlock<T: 'static + Send>(&'_ self) -> Option<&'_ mut T> {
        self.typed.get_of_mut_nonlock::<T>()
    }
    #[inline]
    pub fn insert_of<T: 'static + Send + Sync>(&self, value: T) {
        self.typed.insert_of::<T>(value);
    }
    #[inline]
    pub fn get<T: 'static + Send + Sync>(
        &'_ self,
        key: impl Into<String>,
    ) -> Option<MapReader<'_, T>> {
        self.named.get::<T>(&key.into())
    }
    #[inline]
    pub fn get_mut<T: 'static + Send + Sync>(
        &'_ self,
        key: impl Into<String>,
    ) -> Option<MapWriter<'_, T>> {
        self.named.get_mut::<T>(&key.into())
    }
    /// # Safety
    /// This function does not lock the table or the field, so it is up to the caller to ensure
    /// that no other threads are accessing the table or the field while this function is being
    /// called.
    #[allow(clippy::mut_from_ref)]
    #[inline]
    pub unsafe fn get_mut_nonlock<T: 'static + Send + Sync>(
        &'_ self,
        key: impl Into<String>,
    ) -> Option<&'_ mut T> {
        self.named.get_mut_nonlock::<T>(&key.into())
    }
    #[inline]
    pub fn insert<T: 'static + Send + Sync>(&self, key: impl Into<String>, value: T) {
        self.named.insert(key.into(), value);
    }
}

pub struct MapReader<'a, T> {
    data: &'a T,
    _table_lock: RwLockReadGuard<'a, ()>,
    _field_lock: RwLockReadGuard<'a, ()>,
    _p: PhantomData<T>,
}
unsafe impl<'a, T> Send for MapReader<'a, T> {}
impl<'a, T: 'static> Deref for MapReader<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: 'static> MapReader<'a, T> {
    fn new(
        table_lock: RwLockReadGuard<'a, ()>,
        field_lock: RwLockReadGuard<'a, ()>,
        data: &'a (dyn Any + Send),
    ) -> Self {
        Self {
            data: data.downcast_ref::<T>().unwrap(),
            _field_lock: field_lock,
            _table_lock: table_lock,
            _p: PhantomData,
        }
    }
}

pub struct MapWriter<'a, T> {
    data: &'a mut T,
    _table_lock: RwLockReadGuard<'a, ()>,
    _field_lock: RwLockWriteGuard<'a, ()>,
    _p: PhantomData<T>,
}
unsafe impl<'a, T> Send for MapWriter<'a, T> {}
impl<'a, T: 'static> Deref for MapWriter<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T: 'static> DerefMut for MapWriter<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}
impl<'a, T: 'static> MapWriter<'a, T> {
    fn new(
        table_lock: RwLockReadGuard<'a, ()>,
        field_lock: RwLockWriteGuard<'a, ()>,
        data: &'a mut (dyn Any + Send),
    ) -> Self {
        Self {
            data: data.downcast_mut::<T>().unwrap(),
            _field_lock: field_lock,
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
