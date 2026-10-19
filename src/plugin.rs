use anyhow::{Context, Result};
use std::{path::Path, sync::Arc};
use wasmtime::{Engine, Instance, Module, Store};

#[derive(Clone)]
pub struct PluginManager {
    engine: Engine,
    module: Arc<Module>,
}

// 创建插件管理器
impl PluginManager {
    // 加载插件
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        // 创建 Wasmtime 引擎
        let engine = Engine::default();

        // 加载插件模块
        let module = Module::from_file(&engine, path.as_ref())
            .context("failed to load wasm plugin")?;

        //  返回插件管理器实例
        Ok(Self {
            engine,
            module: Arc::new(module),
        })
    }

    // 执行插件
    pub fn execute(&self, value: i32) -> Result<i32> {
        // 创建一个新的 Store
        let mut store = Store::new(&self.engine, ());

        // 创建插件实例
        let instance = Instance::new(&mut store, &self.module, &[])
            .context("failed to create wasm instance")?;

        //  获取插件导出的 execute 函数
        let execute = instance
            .get_typed_func::<i32, i32>(&mut store, "execute")
            .context("plugin does not export execute(i32) -> i32")?;

        //  调用插件的 execute 函数并返回结果
        execute
            .call(&mut store, value)
            .context("wasm plugin execution failed")
    }
}