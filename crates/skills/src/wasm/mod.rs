use savant_core::traits::SkillExecutor;
use savant_core::error::SavantError;
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, ResourceTable, WasiView};
use wasmtime_wasi::preview1::WasiPreview1View;
use std::pin::Pin;
use futures::future::Future;

/// Data stored within the Wasmtime Store.
struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    p1: wasmtime_wasi::preview1::WasiPreview1Adapter,
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
}

impl WasiPreview1View for HostState {
    fn adapter(&self) -> &wasmtime_wasi::preview1::WasiPreview1Adapter { &self.p1 }
    fn adapter_mut(&mut self) -> &mut wasmtime_wasi::preview1::WasiPreview1Adapter { &mut self.p1 }
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

impl SkillExecutor for WasmSkillExecutor {
    fn execute(&self, payload: &str) -> Pin<Box<dyn Future<Output = Result<String, SavantError>> + Send>> {
        let engine = self.engine.clone();
        let module = self.module.clone();
        let payload = payload.to_string();

        Box::pin(async move {
            let mut wasi_builder = WasiCtxBuilder::new();
            
            wasi_builder.arg("savant_skill");
            wasi_builder.arg(&payload);

            let ctx = wasi_builder.build();
            let table = ResourceTable::new();
            let p1 = wasmtime_wasi::preview1::WasiPreview1Adapter::new();
            
            let mut store = Store::new(&engine, HostState { ctx, table, p1 });
            let mut linker = Linker::new(&engine);
            
            wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |hs: &mut HostState| hs)
                .map_err(|e| SavantError::Unknown(format!("Linker error: {}", e)))?;

            let instance = linker.instantiate_async(&mut store, &module).await
                .map_err(|e| SavantError::Unknown(format!("Instantiation failed: {}", e)))?;

            let func = instance.get_typed_func::<(), ()>(&mut store, "_start")
                .map_err(|e| SavantError::Unknown(format!("Missing _start in WASM: {}", e)))?;

            let _: () = func.call_async(&mut store, ()).await
                .map_err(|e| SavantError::Unknown(format!("Execution failed: {}", e)))?;

            Ok("WASM execution completed".to_string())
        })
    }
}
