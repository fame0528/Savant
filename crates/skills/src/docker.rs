//
use savant_core::error::SavantError;
use bollard::Docker;
use async_trait::async_trait;

/// Wraps skills wrapped inside of a Dockerized architecture locally.
pub struct DockerSkillExecutor {
    docker: Docker,
    image_name: String,
}

impl DockerSkillExecutor {
    /// Prepares Docker integration via `bollard`.
    pub fn new(image_name: String) -> Self {
        let docker = Docker::connect_with_local_defaults()
            .expect("Docker is required for DockerSkillExecutor; ensure it is running and accessible");
        tracing::info!("Docker executor initialized for image: {}", image_name);
        Self { docker, image_name }
    }
}

#[async_trait]
impl savant_core::traits::Tool for DockerSkillExecutor {
    fn name(&self) -> &str { "docker_skill" }
    fn description(&self) -> &str { "Executes a skill within a Docker container." }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let docker = self.docker.clone();
        let image = self.image_name.clone();
        let input = payload.to_string();

        use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions, LogsOptions};
        use futures::StreamExt;
        use uuid::Uuid;

        let name = format!("savant-skill-{}", Uuid::new_v4());
        
        // 1. Create Container
        docker.create_container(
            Some(CreateContainerOptions { name: &name, platform: None }),
            Config {
                image: Some(image),
                env: Some(vec![format!("SAVANT_INPUT={}", input)]),
                ..Default::default()
            }
        ).await.map_err(|e| SavantError::Unknown(format!("Docker create fail: {}", e)))?;

        // 2. Start
        docker.start_container(&name, None::<StartContainerOptions<String>>).await
            .map_err(|e| SavantError::Unknown(format!("Docker start fail: {}", e)))?;

        // 3. Wait
        let mut wait_stream = docker.wait_container(&name, None::<WaitContainerOptions<String>>);
        let _ = wait_stream.next().await;

        // 4. Logs (The Output)
        let mut logs = docker.logs(&name, Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            ..Default::default()
        }));

        let mut output = String::new();
        while let Some(log) = logs.next().await {
            if let Ok(l) = log {
                output.push_str(&savant_core::utils::parsing::bytes_to_string(&l.into_bytes()));
            }
        }

        // 5. Cleanup
        let _ = docker.remove_container(&name, None).await;

        Ok(output)
    }
}
