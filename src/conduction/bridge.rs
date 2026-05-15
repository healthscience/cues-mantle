use std::os::raw::{c_void};
use std::ptr;
use std::ffi::CString;
use crate::conduction::runtime::{js_env_t, js_value_t, js_get_global, js_get_named_property, js_set_named_property, js_create_external_arraybuffer};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

pub struct Bridge {
    pub particles: Vec<Particle>,
}

impl Bridge {
    pub fn new(count: usize) -> Self {
        let mut particles = Vec::with_capacity(count);
        for i in 0..count {
            particles.push(Particle {
                x: i as f32 * 10.0,
                y: 360.0,
                vx: 0.0,
                vy: 0.0,
            });
        }
        Self { particles }
    }

    pub fn inject(&self, env: *mut js_env_t) -> anyhow::Result<()> {
        unsafe {
            let mut global: *mut js_value_t = ptr::null_mut();
            js_get_global(env, &mut global);

            let hop_name = CString::new("hop").unwrap();
            let mut hop: *mut js_value_t = ptr::null_mut();
            js_get_named_property(env, global, hop_name.as_ptr(), &mut hop);

            if hop.is_null() {
                return Err(anyhow::anyhow!("'hop' global not found"));
            }

            let mut array_buffer: *mut js_value_t = ptr::null_mut();
            let byte_length = self.particles.len() * std::mem::size_of::<Particle>();
            
            let res = js_create_external_arraybuffer(
                env,
                self.particles.as_ptr() as *mut c_void,
                byte_length,
                ptr::null(), // No finalize_cb for now, Bridge owns the memory
                ptr::null_mut(),
                &mut array_buffer,
            );

            if res != 0 {
                return Err(anyhow::anyhow!("Failed to create ExternalArrayBuffer: {}", res));
            }

            let buffer_name = CString::new("particleBuffer").unwrap();
            js_set_named_property(env, hop, buffer_name.as_ptr(), array_buffer);
        }

        Ok(())
    }
}
