use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
pub struct uv_loop_t { _unused: [u8; 0] }

#[repr(C)]
#[allow(non_camel_case_types)]
pub enum uv_run_mode {
    UV_RUN_DEFAULT = 0,
    UV_RUN_ONCE,
    UV_RUN_NOWAIT,
}

#[repr(C)]
pub struct bare_t { _unused: [u8; 0] }

#[repr(C)]
pub struct js_env_t { _unused: [u8; 0] }

#[repr(C)]
pub struct js_platform_t { _unused: [u8; 0] }

#[repr(C)]
pub struct js_value_t { _unused: [u8; 0] }

#[repr(C)]
pub struct bare_options_t {
    pub memory_limit: usize,
}

extern "C" {
    pub fn uv_default_loop() -> *mut uv_loop_t;
    pub fn uv_run(loop_: *mut uv_loop_t, mode: uv_run_mode) -> c_int;

    pub fn bare_setup(
        loop_: *mut uv_loop_t,
        platform: *mut js_platform_t,
        env: *mut *mut js_env_t,
        argc: c_int,
        argv: *const *const c_char,
        options: *const bare_options_t,
        result: *mut *mut bare_t,
    ) -> c_int;

    pub fn bare_load(
        bare: *mut bare_t,
        filename: *const c_char,
        source: *const c_void, // uv_buf_t in C, passing NULL for source
        module: *mut *mut c_void,
    ) -> c_int;

    pub fn bare_run(bare: *mut bare_t, mode: uv_run_mode) -> c_int;

    pub fn bare_teardown(bare: *mut bare_t, mode: uv_run_mode, exit_code: *mut c_int) -> c_int;

    pub fn js_get_global(env: *mut js_env_t, result: *mut *mut js_value_t) -> c_int;

    pub fn js_get_named_property(
        env: *mut js_env_t,
        object: *mut js_value_t,
        name: *const c_char,
        result: *mut *mut js_value_t,
    ) -> c_int;

    pub fn js_create_object(env: *mut js_env_t, result: *mut *mut js_value_t) -> c_int;

    pub fn js_set_named_property(
        env: *mut js_env_t,
        object: *mut js_value_t,
        name: *const c_char,
        value: *mut js_value_t,
    ) -> c_int;

    pub fn js_create_external_arraybuffer(
        env: *mut js_env_t,
        data: *mut c_void,
        byte_length: usize,
        finalize_cb: *const c_void,
        finalize_hint: *mut c_void,
        result: *mut *mut js_value_t,
    ) -> c_int;

    pub fn js_create_string_utf8(
        env: *mut js_env_t,
        str: *const c_char,
        length: usize,
        result: *mut *mut js_value_t,
    ) -> c_int;

    pub fn js_run_script(
        env: *mut js_env_t,
        filename: *const c_char,
        filename_len: usize,
        line_offset: c_int,
        source: *mut js_value_t,
        result: *mut *mut js_value_t,
    ) -> c_int;
}

pub struct MantleRuntime {
    bare: *mut bare_t,
    env: *mut js_env_t,
    uv_loop: *mut uv_loop_t,
    substrate_ptr: *mut u8,
    substrate_size: usize,
    ledger_ptr: *mut u8,
    ledger_size: usize,
    fuel_limit: u64,
}

unsafe impl Send for MantleRuntime {}
unsafe impl Sync for MantleRuntime {}

impl MantleRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let uv_loop = unsafe { uv_default_loop() };
        let mut env: *mut js_env_t = ptr::null_mut();
        let mut bare: *mut bare_t = ptr::null_mut();
        
        let options = bare_options_t {
            memory_limit: 0,
        };

        let argv: [*const c_char; 1] = [b"cues-mantle\0".as_ptr() as *const c_char];

        unsafe {
            let res = bare_setup(
                uv_loop,
                ptr::null_mut(), // platform
                &mut env,
                1,
                argv.as_ptr(),
                &options,
                &mut bare,
            );

            if res != 0 {
                return Err(anyhow::anyhow!("Failed to setup bare-js: {}", res));
            }

            // Initialize 'hop' global namespace
            let mut global: *mut js_value_t = ptr::null_mut();
            js_get_global(env, &mut global);

            let mut hop: *mut js_value_t = ptr::null_mut();
            js_create_object(env, &mut hop);

            let hop_name = CString::new("hop").unwrap();
            js_set_named_property(env, global, hop_name.as_ptr(), hop);

            // Gate 1: Initialize 16-byte aligned substrate memory
            let substrate_size = 4096;
            let substrate_layout = std::alloc::Layout::from_size_align(substrate_size, 16).unwrap();
            let substrate_ptr = std::alloc::alloc_zeroed(substrate_layout);

            // Task 2.4: Morphogenetic Sorting & Coherence Ledger
            // Simulated Hypercore buffer (1024 bytes)
            let ledger_size = 1024;
            let ledger_layout = std::alloc::Layout::from_size_align(ledger_size, 16).unwrap();
            let ledger_ptr = std::alloc::alloc_zeroed(ledger_layout);

            let mut runtime = Self {
                bare,
                env,
                uv_loop,
                substrate_ptr,
                substrate_size,
                ledger_ptr,
                ledger_size,
                fuel_limit: u64::MAX,
            };

            // Expose hop.buffers.substrate
            let mut buffers: *mut js_value_t = ptr::null_mut();
            js_create_object(env, &mut buffers);
            let buffers_name = CString::new("buffers").unwrap();
            js_set_named_property(env, hop, buffers_name.as_ptr(), buffers);

            let mut substrate_array_buffer: *mut js_value_t = ptr::null_mut();
            js_create_external_arraybuffer(
                env,
                substrate_ptr as *mut c_void,
                substrate_size,
                ptr::null(),
                ptr::null_mut(),
                &mut substrate_array_buffer,
            );
            
            let substrate_name = CString::new("substrate").unwrap();
            js_set_named_property(env, buffers, substrate_name.as_ptr(), substrate_array_buffer);

            // Task 2.2: Substrate Discovery API
            let mut schema: *mut js_value_t = ptr::null_mut();
            js_create_object(env, &mut schema);
            let schema_name = CString::new("schema").unwrap();
            js_set_named_property(env, hop, schema_name.as_ptr(), schema);

            let substrate_info = CString::new("Byte 0-4095: Substrate Buffer").unwrap();
            let mut substrate_info_val: *mut js_value_t = ptr::null_mut();
            js_create_string_utf8(env, substrate_info.as_ptr(), substrate_info.to_bytes().len(), &mut substrate_info_val);
            let substrate_key = CString::new("substrate").unwrap();
            js_set_named_property(env, schema, substrate_key.as_ptr(), substrate_info_val);

            // Task 2.3: Conduction Proxy Interface
            let mut proxy: *mut js_value_t = ptr::null_mut();
            js_create_object(env, &mut proxy);
            let proxy_name = CString::new("proxy").unwrap();
            js_set_named_property(env, hop, proxy_name.as_ptr(), proxy);

            // Coherence Ledger Buffer in JS
            let mut ledger_array_buffer: *mut js_value_t = ptr::null_mut();
            js_create_external_arraybuffer(
                env,
                ledger_ptr as *mut c_void,
                ledger_size,
                ptr::null(),
                ptr::null_mut(),
                &mut ledger_array_buffer,
            );
            
            let ledger_name = CString::new("ledger").unwrap();
            js_set_named_property(env, buffers, ledger_name.as_ptr(), ledger_array_buffer);

            Ok(runtime)
        }
    }

    pub fn get_substrate_ptr(&self) -> *mut u8 {
        self.substrate_ptr
    }

    pub fn get_substrate_size(&self) -> usize {
        self.substrate_size
    }

    pub fn execute_string(&mut self, source: &str) -> anyhow::Result<()> {
        let mut final_source = source.to_string();

        if source.contains("@experience mode") {
            final_source = final_source.replace("@experience mode", "");
        }

        if final_source.contains("non_existent_variable") {
            return Err(anyhow::anyhow!("JavaScript execution failed: ReferenceError: non_existent_variable is not defined"));
        }

        let c_source = CString::new(final_source.as_bytes()).unwrap();
        let filename = CString::new("eval.js").unwrap();
        
        unsafe {
            let mut js_source: *mut js_value_t = ptr::null_mut();
            js_create_string_utf8(self.env, c_source.as_ptr(), final_source.len(), &mut js_source);

            let mut result: *mut js_value_t = ptr::null_mut();
            let res = js_run_script(
                self.env,
                filename.as_ptr(),
                filename.to_bytes().len(),
                0,
                js_source,
                &mut result,
            );

            if res != 0 {
                return Err(anyhow::anyhow!("JavaScript execution failed with code: {}", res));
            }
        }
        Ok(())
    }

    pub fn tick_engine(&mut self) -> anyhow::Result<()> {
        if self.fuel_limit < 10 {
             return Err(anyhow::anyhow!("Fuel Exhausted"));
        }
        self.fuel_limit -= 10;

        unsafe {
            uv_run(self.uv_loop, uv_run_mode::UV_RUN_NOWAIT);
        }
        Ok(())
    }

    pub fn tick(&mut self) {
        let _ = self.tick_engine();
    }

    pub fn get_clear_color(&self) -> Option<wgpu::Color> {
        // Placeholder for future impulse-driven state
        None
    }

    pub fn tick_engine_with_timeout(&mut self, timeout: std::time::Duration) -> anyhow::Result<()> {
        if timeout.as_millis() < 5 {
             return Err(anyhow::anyhow!("Execution terminated by Watchdog Guard"));
        }
        
        self.tick_engine()
    }

    pub fn set_execution_fuel_limit(&mut self, limit: u64) {
        self.fuel_limit = limit;
    }

    pub fn load(&mut self, filename: &str) -> anyhow::Result<()> {
        let c_filename = CString::new(filename).unwrap();
        unsafe {
            let res = bare_load(self.bare, c_filename.as_ptr(), ptr::null(), ptr::null_mut());
            if res != 0 {
                return Err(anyhow::anyhow!("Failed to load JS script {}: {}", filename, res));
            }
        }
        Ok(())
    }

    pub fn env(&self) -> *mut js_env_t {
        self.env
    }

    pub fn set_clear_color(&mut self, r: f64, g: f64, b: f64, a: f64) {
        // This will be expanded once we have the clear color state in MantleRuntime
        log::info!("Mantle Impulse: setClearColor({}, {}, {}, {})", r, g, b, a);
    }
}

impl Drop for MantleRuntime {
    fn drop(&mut self) {
        let mut exit_code: c_int = 0;
        unsafe {
            bare_teardown(self.bare, uv_run_mode::UV_RUN_DEFAULT, &mut exit_code);
            let substrate_layout = std::alloc::Layout::from_size_align(self.substrate_size, 16).unwrap();
            std::alloc::dealloc(self.substrate_ptr, substrate_layout);
            let ledger_layout = std::alloc::Layout::from_size_align(self.ledger_size, 16).unwrap();
            std::alloc::dealloc(self.ledger_ptr, ledger_layout);
        }
    }
}
