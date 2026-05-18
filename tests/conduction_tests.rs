// tests/conduction_tests.rs
// Baseline Fence for Gate 1 and Gate 2

#[cfg(test)]
mod conduction_tests {
    use cues_mantle::conduction::runtime::MantleRuntime;

    // Gate 1 Baseline: Verify raw page-aligned memory and cross-FFI visibility
    #[test]
    fn test_substrate_buffer_alignment_and_integrity() {
        // Initialize the core runtime wrapper
        let mut mantle_runtime = MantleRuntime::new()
            .expect("Mantle core failed initialization");
        
        let ptr = mantle_runtime.get_substrate_ptr();
        let size = mantle_runtime.get_substrate_size();

        // Enforce the strict 16-byte alignment and 4096-byte slot capacity boundaries
        assert_eq!(size, 4096, "Substrate buffer must be exactly 4096 bytes");
        assert!(ptr as usize % 16 == 0, "Substrate memory allocation must be 16-byte page-aligned");

        // Write directly to the layout on the native side
        unsafe {
            let memory_view = std::slice::from_raw_parts_mut(ptr as *mut f32, 16);
            memory_view[0] = 77.7; // Test Impulse Value
        }

        // Assert JavaScript reads the exact memory address without serialization overhead
        let js_verification = r#"
            if (hop.buffers.substrate[0] !== 77.7) {
                throw new Error("FFI substrate memory boundary misaligned");
            }
        "#;

        let result = mantle_runtime.execute_string(js_verification);
        assert!(result.is_ok(), "JavaScript environment failed to read the memory alignment target");
    }

    // Gate 2 Baseline: Verify the native host intercepts runtime crashes without panicking
    #[test]
    fn test_js_exception_trapping_and_isolation() {
        let mut mantle_runtime = MantleRuntime::new()
            .expect("Mantle core failed initialization");

        // Inject an explicitly broken, unaligned string current
        let broken_current = r#"
            @experience mode
            hop.onTick(() => {
                let failure = non_existent_variable.trigger_crash();
            });
        "#;

        let result = mantle_runtime.execute_string(broken_current);
        
        // Assert that the native runtime traps the crash cleanly instead of dropping the binary process
        assert!(result.is_err(), "Mantle core failed to trap and isolate the execution crash");
        
        // Verify the master clock loop is perfectly preserved and responsive
        let tick_status = mantle_runtime.tick_engine();
        assert!(tick_status.is_ok(), "The frame clock loop collapsed following an execution crash");
    }

    // Gate 3 Baseline: Verify the Watchdog Guard terminates long-running scripts
    #[test]
    fn test_js_infinite_loop_freeze_termination() {
        let mut mantle_runtime = MantleRuntime::new()
            .expect("Mantle core failed initialization");

        // Inject a simulated infinite loop
        let infinite_script = r#"
            @experience mode
            hop.onTick(() => {
                while(true) { let x = 1; }
            });
        "#;

        mantle_runtime.execute_string(infinite_script).unwrap();
        
        // Trigger a tick with a tight timeout. The watchdog must intercept and kill the execution.
        let tick_result = mantle_runtime.tick_engine_with_timeout(std::time::Duration::from_millis(1));
        
        assert!(tick_result.is_err(), "Engine allowed an infinite loop to freeze the thread");
        assert!(
            tick_result.unwrap_err().to_string().contains("Execution terminated by Watchdog Guard"),
            "Unexpected error variant returned"
        );
    }

    // Gate 4 Baseline: Verify sandbox permissions and fuel metering
    #[test]
    fn test_isolate_sandbox_permissions_and_fuel() {
        let mut mantle_runtime = MantleRuntime::new()
            .expect("Mantle core failed initialization");

        // 1. Test Sandbox (Probing for forbidden host resources)
        let malicious_probe = r#"
            if (typeof require !== 'undefined' || typeof process !== 'undefined') {
                throw new Error("Sandbox breach: host resources detected");
            }
        "#;
        let sandbox_result = mantle_runtime.execute_string(malicious_probe);
        assert!(sandbox_result.is_ok(), "Isolate sandbox breached: Script detected host environment!");

        // 2. Test Fuel Metering
        mantle_runtime.set_execution_fuel_limit(15); // Very low fuel
        
        // First tick should pass (costs 10)
        assert!(mantle_runtime.tick_engine().is_ok());
        
        // Second tick should fail (exhausted)
        let tick_result = mantle_runtime.tick_engine();
        assert!(tick_result.is_err(), "Engine allowed execution with exhausted fuel");
        assert!(
            tick_result.unwrap_err().to_string().contains("Fuel Exhausted"),
            "Expected Fuel Exhaustion error type"
        );
    }

    // Gate 5 Baseline: Verify @experience mode interceptor
    #[test]
    fn test_experience_mode_interceptor() {
        let mut mantle_runtime = MantleRuntime::new()
            .expect("Mantle core failed initialization");

        // The script contains the @experience mode tag, which should be stripped and executed as valid JS
        let experience_script = r#"
            @experience mode
            if (typeof hop === 'undefined') throw new Error('hop missing');
        "#;

        let result = mantle_runtime.execute_string(experience_script);
        assert!(result.is_ok(), "Experience mode interceptor failed to process valid script");
    }

    // Assert Phase 1.1: Cold startup time must not block the host ecosystem
    #[test]
    fn test_mantle_cold_boot_perf_boundary() {
        let start_time = std::time::Instant::now();
        
        let runtime = MantleRuntime::new();
        assert!(runtime.is_ok(), "Mantle runtime crashed during initialization");
        
        let boot_duration = start_time.elapsed().as_millis();
        // Enforce that even during test compilation, the foundation settles in under 1.5 seconds
        assert!(boot_duration < 1500, "Cold boot duration took {}ms - initialization is flooding the thread!", boot_duration);
    }
}
