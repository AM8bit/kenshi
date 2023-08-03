use std::net::TcpStream;
use mlua::prelude::*;
use mlua::{Lua, Result};

fn http_get(_: &Lua, (_, _):(String, u16)) -> Result<()>{
    Ok(())
}

fn http_post(_: &Lua, (_, _):(String, u16)) -> LuaResult<()>{
    Ok(())
}

fn json_post(_: &Lua, (_, _):(String, u16)) -> LuaResult<()>{
    Ok(())
}