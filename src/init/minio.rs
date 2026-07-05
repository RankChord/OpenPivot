use std::error::Error;

use aws_sdk_s3::Client;
use aws_credential_types::Credentials;

use super::config::MinioConfig;

async fn create_client(config: &MinioConfig) -> Result<Client, Box<dyn Error + Send + Sync>> {
    let credentials = Credentials::new(
        config.username,
        config.password,
        None,
        None,
        "minio_init",
    );
}

async fn ensure_bucket_exists(
    client: Client,
    bucket: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 先查 bucket 列表
    let out = client.list_buckets().send().await?;
    let exists = out
        .buckets()
        .unwrap_or_default()
        .iter()
        .any(|b| b.name().unwrap_or_default() == bucket);

    if !exists {
        // 不存在则创建
        client
            .create_bucket()
            .bucket(bucket)
            .send()
            .await?;
    }
    Ok(())
}