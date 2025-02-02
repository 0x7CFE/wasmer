#ifdef __cplusplus
extern "C" {
#endif

#include "wasmer_wasm.h"
#include "wasm.h"
#include "my_wasm.h"

#include <stdio.h>
#include <stdlib.h>

#ifdef __cplusplus
}
#endif

void print_wasmer_error()
{
  int error_len = wasmer_last_error_length();
  printf("Error len: `%d`\n", error_len);
  char* error_str = (char*) malloc(error_len);
  wasmer_last_error_message(error_str, error_len);
  printf("Error str: `%s`\n", error_str);
}

int main() {
  printf("Initializing...\n");
  wasm_config_t* config = wasm_config_new();
  wasm_config_set_engine(config, OBJECT_FILE);
  wasm_engine_t* engine = wasm_engine_new_with_config(config);
  wasm_store_t* store = wasm_store_new(engine);

  wasm_module_t* module = wasmer_object_file_engine_new(store, "qjs.wasm");

  if (!module) {
    printf("Failed to create module\n");
    print_wasmer_error();
    return -1;
  }
        
  // We have now finished the memory buffer book keeping and we have a valid Module.

  // In this example we're passing some JavaScript source code as a command line argument
  // to a WASI module that can evaluate JavaScript.
  wasi_config_t* wasi_config = wasi_config_new("constant_value_here");
  const char* js_string = "function greet(name) { return JSON.stringify('Hello, ' + name); }; print(greet('World'));";
  wasi_config_arg(wasi_config, "--eval");
  wasi_config_arg(wasi_config, js_string);
  wasi_env_t* wasi_env = wasi_env_new(wasi_config);

  if (!wasi_env) {
    printf("> Error building WASI env!\n");
    print_wasmer_error();
    return 1;
  }

  wasm_importtype_vec_t import_types;
  wasm_module_imports(module, &import_types);

  wasm_extern_vec_t imports;
  wasm_extern_vec_new_uninitialized(&imports, import_types.size);

  wasm_importtype_vec_delete(&import_types);
        
  bool get_imports_result = wasi_get_imports(store, module, wasi_env, &imports);

  if (!get_imports_result) {
    printf("> Error getting WASI imports!\n");
    print_wasmer_error();
    return 1;
  }

  wasm_instance_t* instance = wasm_instance_new(store, module, &imports, NULL);

  if (!instance) {
    printf("Failed to create instance\n");
    print_wasmer_error();
    return -1;
  }
  wasi_env_set_instance(wasi_env, instance);
        
  // WASI is now set up.

  void* vmctx = wasm_instance_get_vmctx_ptr(instance);
  wasm_val_t* inout[2] = { NULL, NULL };

  fflush(stdout);
  // We're able to call our compiled function directly through a trampoline.
  wasmer_trampoline_function_call__1(vmctx, wasmer_function__1, &inout);

  wasm_extern_vec_delete(&imports);
  wasm_instance_delete(instance);
  wasm_module_delete(module);
  wasm_store_delete(store);
  wasm_engine_delete(engine);

  return 0;
}
