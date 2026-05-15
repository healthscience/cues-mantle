use wasmtime::{Engine, Linker, Module, Store, Instance, TypedFunc};
use anyhow::Result;
use std::fs;

pub struct HeliRuntime {
    store: Store<()>,
    instance: Instance,
}

impl HeliRuntime {
    pub fn new(wasm_path: &str) -> Result<Self> {
        let engine = Engine::default();
        let wasm_bytes = fs::read(wasm_path)?;
        let module = Module::new(&engine, &wasm_bytes[..])?;
        
        let mut store = Store::new(&engine, ());
        let mut linker = Linker::new(&engine);
        
        // Mock wasm-bindgen imports
        linker.func_wrap("./heli_clock_bg.js", "__wbg___wbindgen_throw_be289d5034ed271b", |_arg0: i32, _arg1: i32| {
        })?;
        
        linker.func_wrap("./heli_clock_bg.js", "__wbindgen_init_externref_table", || {
        })?;

        let instance = linker.instantiate(&mut store, &module)?;
        
        // __wbindgen_start
        if let Ok(func) = instance.get_typed_func::<(), ()>(&mut store, "__wbindgen_start") {
            func.call(&mut store, ())?;
        }

        Ok(Self { store, instance })
    }

    pub fn get_orbital_degree(&mut self, timestamp_ms: i64) -> Result<f64> {
        let func: TypedFunc<i64, f64> = self.instance.get_typed_func(&mut self.store, "helicore_get_orbital_degree")?;
        let degree = func.call(&mut self.store, timestamp_ms)?;
        Ok(degree)
    }

    pub fn get_zenith_angle(&mut self, lat: f64, lon: f64, timestamp_ms: i64) -> Result<f64> {
        let func: TypedFunc<(f64, f64, i64), f64> = self.instance.get_typed_func(&mut self.store, "helicore_get_zenith_angle")?;
        let angle = func.call(&mut self.store, (lat, lon, timestamp_ms))?;
        Ok(angle)
    }
}
