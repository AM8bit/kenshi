use mlua::prelude::*;
use std::net::TcpStream;

fn check_tcp_port(_: &Lua, (addr, port):(String, u16)) -> LuaResult<bool>{
    Ok(TcpStream::connect(format!("{addr}:{port}")).is_ok())
}