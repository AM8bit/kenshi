use std::net::TcpStream;
use mlua::prelude::*;
use mlua::{Lua, Result};


pub fn check_tcp_port(_: &Lua, (addr, port):(String, u16)) -> Result<bool>{
    Ok(TcpStream::connect(format!("{addr}:{port}")).is_ok())
}

