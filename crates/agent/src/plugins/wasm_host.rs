use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

use wasmtime::component::*;
use wasmtime::{Config, Engine, Store, StoreContextMut};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView, ResourceTable};

bindgen!({
    world: "plugin",
    path: "src/plugins/savant_hooks.wit",
});

use exports::savant::agent_hooks::hooks::HookResult;

use savant_security::{AgentToken, SecurityEnclave};

struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    enclave: Arc<SecurityEnclave>,
    agent_id: u64,
    token: Option<AgentToken>,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

pub struct WasmPluginHost {
    engine: Engine,
    linker: Linker<HostState>,
    enclave: Arc<SecurityEnclave>,
}

impl WasmPluginHost {
    pub fn new(root_authority: ed25519_dalek::VerifyingKey) -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        let enclave = Arc::new(SecurityEnclave::new(root_authority));

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        
        // Implement the host interface with CCT verification
        linker.instance("host")?.func_wrap(
            "call-tool",
            move |cx: StoreContextMut<'_, HostState>, (tool_name, args): (String, String)| {
                let state = cx.data();
                let token = state.token.as_ref().ok_or_else(|| anyhow::anyhow!("No security token provided"))?;
                
                // CRITICAL: Stateless cryptographic verification
                state.enclave.verify_token_and_action(
                    token,
                    state.agent_id,
                    &tool_name,
                    "execute"
                ).map_err(|e| anyhow::anyhow!("Security Boundary Violation: {}", e))?;
                
                // If verified, proceed to "execute" (Mock for now, will connect to ToolRegistry)
                Ok((format!("Result of {} with args {}", tool_name, args),))
            },
        )?;

        Ok(Self { engine, linker, enclave })
    }

    pub async fn load_plugin(&self, path: impl AsRef<Path>) -> Result<Component> {
        Component::from_file(&self.engine, path)
    }

    pub async fn execute_before_llm_call(
        &self, 
        component: &Component, 
        prompt: &str,
        agent_id: u64,
        token: Option<AgentToken>,
    ) -> Result<HookResult> {
        let mut store = Store::new(
            &self.engine,
            HostState {
                ctx: WasiCtxBuilder::new().inherit_stdout().build(),
                table: ResourceTable::new(),
                enclave: self.enclave.clone(),
                agent_id,
                token,
            },
        );
        store.set_fuel(1_000_000)?;

        let plugin = Plugin::instantiate_async(&mut store, component, &self.linker).await?;
        let res = plugin.savant_agent_hooks_hooks().call_before_llm_call(&mut store, prompt)?;
        
        Ok(res)
    }

    pub async fn execute_after_tool_call(
        &self, 
        component: &Component, 
        tool_name: &str, 
        result: &str,
        agent_id: u64,
        token: Option<AgentToken>,
    ) -> Result<HookResult> {
        let mut store = Store::new(
            &self.engine,
            HostState {
                ctx: WasiCtxBuilder::new().inherit_stdout().build(),
                table: ResourceTable::new(),
                enclave: self.enclave.clone(),
                agent_id,
                token,
            },
        );
        store.set_fuel(1_000_000)?;

        let plugin = Plugin::instantiate_async(&mut store, component, &self.linker).await?;
        let res = plugin.savant_agent_hooks_hooks().call_after_tool_call(&mut store, tool_name, result)?;
        
        Ok(res)
    }

    pub async fn execute_before_response_emit(
        &self, 
        component: &Component, 
        response: &str,
        agent_id: u64,
        token: Option<AgentToken>,
    ) -> Result<HookResult> {
        let mut store = Store::new(
            &self.engine,
            HostState {
                ctx: WasiCtxBuilder::new().inherit_stdout().build(),
                table: ResourceTable::new(),
                enclave: self.enclave.clone(),
                agent_id,
                token,
            },
        );
        store.set_fuel(1_000_000)?;

        let plugin = Plugin::instantiate_async(&mut store, component, &self.linker).await?;
        let res = plugin.savant_agent_hooks_hooks().call_before_response_emit(&mut store, response)?;
        
        Ok(res)
    }
}
