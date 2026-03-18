use async_trait::async_trait;
use bollard::Docker;
use savant_core::error::SavantError;
use tokio::time::{timeout, Duration};

/// Maximum execution time for Docker container runs (30 seconds)
const DOCKER_EXEC_TIMEOUT_SECS: u64 = 30;

/// Wraps skills wrapped inside of a Dockerized architecture locally.
pub struct DockerSkillExecutor {
    docker: Docker,
    image_name: String,
}

impl DockerSkillExecutor {
    /// Prepares Docker integration via `bollard`.
    pub fn new(image_name: String) -> Result<Self, SavantError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| SavantError::Unknown(format!("Docker connection failed: {}", e)))?;
        tracing::info!("Docker executor initialized for image: {}", image_name);
        Ok(Self { docker, image_name })
    }

    /// Verifies Docker daemon is reachable.
    pub async fn health_check(&self) -> Result<String, SavantError> {
        let version = self
            .docker
            .version()
            .await
            .map_err(|e| SavantError::Unknown(format!("Docker health check failed: {}", e)))?;
        Ok(version.version.unwrap_or_else(|| "unknown".to_string()))
    }
}

#[async_trait]
impl savant_core::traits::Tool for DockerSkillExecutor {
    fn name(&self) -> &str {
        "docker_skill"
    }
    fn description(&self) -> &str {
        "Executes a skill within a Docker container."
    }
    async fn execute(&self, payload: serde_json::Value) -> Result<String, SavantError> {
        let docker = self.docker.clone();
        let image = self.image_name.clone();
        let input = payload.to_string();

        use bollard::container::{
            Config, CreateContainerOptions, KillContainerOptions, LogsOptions,
            StartContainerOptions, WaitContainerOptions,
        };
        use futures::StreamExt;
        use uuid::Uuid;

        let name = format!("savant-skill-{}", Uuid::new_v4());

        let run_task = async {
            // 1. Create Container
            docker
                .create_container(
                    Some(CreateContainerOptions {
                        name: &name,
                        platform: None,
                    }),
                    Config {
                        image: Some(image),
                        env: Some(vec![format!("SAVANT_INPUT={}", input)]),
                        host_config: Some(bollard::service::HostConfig {
                            memory: Some(512 * 1024 * 1024),        // 512MB
                            nano_cpus: Some(1_000_000_000),         // 1.0 CPU
                            readonly_rootfs: Some(true),            // Immutable root filesystem
                            network_mode: Some("none".to_string()), // No network access
                            security_opt: Some(vec!["no-new-privileges:true".to_string()]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| SavantError::Unknown(format!("Docker create fail: {}", e)))?;

            // 2. Start
            docker
                .start_container(&name, None::<StartContainerOptions<String>>)
                .await
                .map_err(|e| SavantError::Unknown(format!("Docker start fail: {}", e)))?;

            // 3. Wait with timeout
            let wait_future = async {
                let mut wait_stream =
                    docker.wait_container(&name, None::<WaitContainerOptions<String>>);
                wait_stream.next().await
            };

            match timeout(Duration::from_secs(DOCKER_EXEC_TIMEOUT_SECS), wait_future).await {
                Ok(Some(Ok(status))) => {
                    if status.status_code != 0 {
                        tracing::warn!(
                            "Docker container {} exited with code {}",
                            name,
                            status.status_code
                        );
                    }
                }
                Ok(Some(Err(e))) => {
                    return Err(SavantError::Unknown(format!("Docker wait error: {}", e)));
                }
                Ok(None) => {
                    return Err(SavantError::Unknown(
                        "Docker wait stream closed without status".to_string(),
                    ));
                }
                Err(_) => {
                    // Timeout: kill the container and return error
                    tracing::warn!(
                        "Docker container {} timed out after {}s, killing",
                        name,
                        DOCKER_EXEC_TIMEOUT_SECS
                    );
                    let _ = docker
                        .kill_container(&name, Some(KillContainerOptions { signal: "SIGKILL" }))
                        .await;
                    return Err(SavantError::Unknown(format!(
                        "Docker execution timed out after {} seconds",
                        DOCKER_EXEC_TIMEOUT_SECS
                    )));
                }
            }

            // 4. Logs (The Output)
            let mut logs = docker.logs(
                &name,
                Some(LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    ..Default::default()
                }),
            );

            let mut output = String::new();
            while let Some(log) = logs.next().await {
                if let Ok(l) = log {
                    output.push_str(&savant_core::utils::parsing::bytes_to_string(
                        &l.into_bytes(),
                    ));
                }
            }
            Ok(output)
        };

        let result = run_task.await;

        // 5. Mandatory Cleanup (🏰 HS-007: Resource Integrity)
        if let Err(e) = docker
            .remove_container(
                &name,
                Some(bollard::container::RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
        {
            tracing::warn!(
                "DockerSkillExecutor: Failed to cleanup Docker container {}: {}",
                name,
                e
            );
        }

        result
    }
}
