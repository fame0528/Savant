#[tokio::main]
async fn main() {
    println!("Prometheus agent started");
    // Simple agent implementation
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        println!("Prometheus agent heartbeat");
    }
}
