use savant_core::error::SavantError;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::WasiCtxBuilder;
use async_trait::async_trait;

/// Data stored within the Wasmtime Store.
struct HostState {
    p1: wasmtime_wasi::preview1::WasiP1Ctx,
}

/// An execution wrapper for WASM capabilities safely sandboxing untrusted code.
pub struct WasmSkillExecutor {
    engine: Engine,
    module: Module,
}

impl WasmSkillExecutor {
    /// Constructs a Wasm runtime execution environment for a specific payload.
    pub fn new(wasm_bytes: &[u8]) -> Result<Self, SavantError> {
        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)
            .map_err(|e| SavantError::Unknown(format!("Failed to create WASM engine: {}", e)))?;
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| SavantError::Unknown(format!("WASM Compilation failed: {}", e)))?;
        Ok(Self { engine, module })
    }
}

#[async_trait]
impl savant_core::traits::Tool for WasmSkillExecutor {
    fn name(&self) -> &str { "wasm_skill" }
    fn description(&self) -> &str { "Executes a skill within a WebAssembly sandbox." }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let engine = self.engine.clone();
        let module = self.module.clone();
        let payload_str = payload.to_string();

        let mut wasi_builder = WasiCtxBuilder::new();
        
        wasi_builder.arg("savant_skill");
        wasi_builder.arg(&payload_str);

        let p1 = wasi_builder.build_p1();
        
        let mut store = Store::new(&engine, HostState { p1 });
        let mut linker = Linker::new(&engine);
        
        wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |hs: &mut HostState| &mut hs.p1)
            .map_err(|e| SavantError::Unknown(format!("Linker error: {}", e)))?;

        let instance = linker.instantiate_async(&mut store, &module).await
            .map_err(|e| SavantError::Unknown(format!("Instantiation failed: {}", e)))?;

        let func = instance.get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| SavantError::Unknown(format!("Missing _start in WASM: {}", e)))?;

        let _: () = func.call_async(&mut store, ()).await
            .map_err(|e| SavantError::Unknown(format!("Execution failed: {}", e)))?;

        Ok("WASM execution completed".to_string())
    }
}
