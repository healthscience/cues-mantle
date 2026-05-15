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
}

pub struct JsRuntime {
    bare: *mut bare_t,
    env: *mut js_env_t,
    uv_loop: *mut uv_loop_t,
}

impl JsRuntime {
    pub fn new(uv_loop: *mut uv_loop_t) -> anyhow::Result<Self> {
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
        }

        Ok(Self {
            bare,
            env,
            uv_loop,
        })
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

    pub fn tick(&mut self) {
        unsafe {
            // Run the event loop in non-blocking mode to keep it in sync with the frame loop
            uv_run(self.uv_loop, uv_run_mode::UV_RUN_NOWAIT);
        }
    }

    pub fn env(&self) -> *mut js_env_t {
        self.env
    }
}

impl Drop for JsRuntime {
    fn drop(&mut self) {
        let mut exit_code: c_int = 0;
        unsafe {
            bare_teardown(self.bare, uv_run_mode::UV_RUN_DEFAULT, &mut exit_code);
        }
    }
}
