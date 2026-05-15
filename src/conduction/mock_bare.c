#include <stdint.h>
#include <stddef.h>

typedef struct {} uv_loop_t;
typedef struct {} bare_t;
typedef struct {} js_env_t;
typedef struct {} js_platform_t;
typedef struct {} js_value_t;

typedef enum {
    UV_RUN_DEFAULT = 0,
    UV_RUN_ONCE,
    UV_RUN_NOWAIT
} uv_run_mode;

typedef struct {
    size_t memory_limit;
} bare_options_t;

uv_loop_t* uv_default_loop() { return (uv_loop_t*)0x1; }
int uv_run(uv_loop_t* loop, uv_run_mode mode) { return 0; }

int bare_setup(uv_loop_t* loop, js_platform_t* platform, js_env_t** env, int argc, const char* argv[], const bare_options_t* options, bare_t** result) {
    *env = (js_env_t*)0x2;
    *result = (bare_t*)0x3;
    return 0;
}

int bare_load(bare_t* bare, const char* filename, const void* source, void** module) { return 0; }
int bare_run(bare_t* bare, uv_run_mode mode) { return 0; }
int bare_teardown(bare_t* bare, uv_run_mode mode, int* exit_code) { return 0; }

int js_get_global(js_env_t* env, js_value_t** result) { *result = (js_value_t*)0x4; return 0; }
int js_create_object(js_env_t* env, js_value_t** result) { *result = (js_value_t*)0x5; return 0; }
int js_set_named_property(js_env_t* env, js_value_t* object, const char* name, js_value_t* value) { return 0; }
int js_get_named_property(js_env_t* env, js_value_t* object, const char* name, js_value_t** result) { *result = (js_value_t*)0x6; return 0; }
int js_create_external_arraybuffer(js_env_t* env, void* data, size_t byte_length, const void* finalize_cb, void* finalize_hint, js_value_t** result) {
    *result = (js_value_t*)0x7;
    return 0;
}
