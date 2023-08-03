mod network;
mod http;
mod meta_gripper;

use std::path::Path;
use mlua::prelude::*;
use mlua::{Function, Table};
use network::*;
use mlua::Result;

pub struct ScriptEngine {
    lua: Lua,
}

#[derive(Default, Debug)]
pub struct UserData {
    text: String,
}

impl ScriptEngine {
    pub fn new(script: &str)-> Result<Self> {
        let lua = Lua::new();
        //let globals = lua.globals();
        lua.load(Path::new(script)).exec()?;
        Ok(Self {
            lua
        })
    }

    pub fn run_script(&self, data: String)-> Result<String> {
        let check_tcp_port = self.lua.create_function(check_tcp_port)?;
        self.lua.globals().set("check_tcp_port", check_tcp_port)?;
        let print: Function = self.lua.globals().get("start")?;
        let output = print.call::<_, String>(data)?;
        Ok(output)
    }

    pub fn about(&self) {
        /*
            let description = lua.globals().get::<_, String>("description")?;
            let author = lua.globals().get::<_, String>("author")?;
            let license = lua.globals().get::<_, String>("license")?;
            let categories = lua.globals().get::<_, Vec<String>>("categories")?;
            println!("{description}\n{author}\n{license}\n{categories:?}");
        */
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

    }
}
