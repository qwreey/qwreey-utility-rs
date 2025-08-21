use std::{error::Error, ops::Deref};

use mlua::{prelude::*, MaybeSend};

pub trait ToLuaResult<T> {
    fn to_lua_result(self) -> LuaResult<T>;
}
impl<T, E: Into<Box<dyn Error + Send + Sync>>> ToLuaResult<T> for Result<T, E> {
    fn to_lua_result(self) -> LuaResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(LuaError::external(err)),
        }
    }
}

pub trait LuaOkOr<T, E: Into<Box<dyn Error + Send + Sync>>> {
    fn lua_ok_or(self, or: E) -> LuaResult<T>;
    fn lua_ok_or_else<F: FnOnce() -> E>(self, or: F) -> LuaResult<T>;
}
impl<T, E: Into<Box<dyn Error + Send + Sync>>> LuaOkOr<T, E> for Option<T> {
    fn lua_ok_or(self, or: E) -> LuaResult<T> {
        match self {
            Some(ok) => Ok(ok),
            None => Err(LuaError::external(or)),
        }
    }
    fn lua_ok_or_else<F: FnOnce() -> E>(self, or: F) -> LuaResult<T> {
        match self {
            Some(ok) => Ok(ok),
            None => Err(LuaError::external(or())),
        }
    }
}

use std::future::Future;

use crate::ArcRwMap;
pub struct TableBuilder {
    lua: Lua,
    tbl: LuaTable,
}

impl TableBuilder {
    pub fn new(lua: &Lua) -> LuaResult<Self> {
        Ok(Self {
            lua: lua.to_owned(),
            tbl: lua.create_table()?,
        })
    }
    pub fn from_table(lua: &Lua, tbl: LuaTable) -> Self {
        Self {
            lua: lua.to_owned(),
            tbl,
        }
    }
    pub fn from_global<K: Into<String>>(lua: &Lua, key: K) -> LuaResult<Self> {
        let key: String = key.into();
        let tbl = lua
            .globals()
            .raw_get::<LuaTable>(key.as_str())
            .or_else(|_| lua.create_table())?;
        lua.globals().set(key.as_str(), &tbl)?;

        Ok(Self {
            lua: lua.clone(),
            tbl,
        })
    }

    pub fn set<K: IntoLua, V: IntoLua>(self, key: K, value: V) -> LuaResult<Self> {
        self.tbl.raw_set(key, value)?;
        Ok(self)
    }

    pub fn bulk_set<K: IntoLua, V: IntoLua>(self, values: Vec<(K, V)>) -> LuaResult<Self> {
        for (key, value) in values {
            self.tbl.raw_set(key, value)?;
        }
        Ok(self)
    }

    pub fn push<V: IntoLua>(self, value: V) -> LuaResult<Self> {
        self.tbl.raw_push(value)?;
        Ok(self)
    }

    pub fn bulk_push<V: IntoLua>(self, values: Vec<V>) -> LuaResult<Self> {
        for value in values {
            self.tbl.raw_push(value)?;
        }
        Ok(self)
    }

    pub fn add_function<K, A, R, F>(self, key: K, func: F) -> LuaResult<Self>
    where
        K: IntoLua,
        A: FromLuaMulti,
        R: IntoLuaMulti,
        F: Fn(&Lua, A) -> LuaResult<R> + 'static + MaybeSend,
    {
        let f = self.lua.create_function(func)?;
        self.set(key, LuaValue::Function(f))
    }

    pub fn add_async_function<K, A, R, F, FR>(self, key: K, func: F) -> LuaResult<Self>
    where
        K: IntoLua,
        A: FromLuaMulti,
        R: IntoLuaMulti,
        F: Fn(Lua, A) -> FR + 'static + MaybeSend,
        FR: Future<Output = LuaResult<R>> + 'static + MaybeSend,
    {
        let f = self.lua.create_async_function(func)?;
        self.set(key, LuaValue::Function(f))
    }

    pub fn set_metatable(self, table: LuaTable) -> LuaResult<Self> {
        self.tbl.set_metatable(Some(table))?;
        Ok(self)
    }

    pub fn build_readonly(self) -> LuaResult<LuaTable> {
        self.tbl.set_readonly(true);
        Ok(self.tbl)
    }

    pub fn build(self) -> LuaResult<LuaTable> {
        Ok(self.tbl)
    }
}

pub trait GetArcRwMap {
    fn get_rw_map(&self) -> LuaResult<ArcRwMap>;
}
impl GetArcRwMap for Lua {
    fn get_rw_map(&self) -> LuaResult<ArcRwMap> {
        Ok(self
            .app_data_ref::<ArcRwMap>()
            .ok_or_else(|| LuaError::external("Failed to retrieve app data"))?
            .deref()
            .clone())
    }
}
