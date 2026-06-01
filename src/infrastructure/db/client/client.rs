use async_trait::async_trait;
use fred::prelude::*;
use std::env;
use std::time::Duration;

#[async_trait]
pub trait RateLimitClient {
    async fn enable_client(&self);
    fn get_ref(&self) -> &Client;
}

pub struct RateLimitDbClient {
    client: Client,
}

fn init_client(config: Config) -> Client {
    let client = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(1);
            config.tcp = TcpConfig {
                nodelay: Some(true),
                ..Default::default()
            };
        })
        .build()
        .unwrap();

    client
}

impl RateLimitDbClient {
    pub(crate) fn new(config: Config) -> Self {
        let client: Client = init_client(config);
        Self { client }
    }
}

#[async_trait]
impl RateLimitClient for RateLimitDbClient {
    async fn enable_client(&self) {
        self.client.init().await.unwrap();
    }
    fn get_ref(&self) -> &Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::client::RateLimitDbClient;

    #[tokio::test]
    async fn test_init_client() {
        let binding = env::var("REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
        let redis_url = binding.as_str();
        let config = fred::prelude::Config::from_url(redis_url).unwrap();
        let client = RateLimitDbClient::new(config);

        client.enable_client().await;

        let ref_ = client.get_ref();
        let pong_msg = String::from("PING");
        let result = ref_.ping::<String>(Option::from(pong_msg)).await;
        assert!(result.is_ok());
        assert_eq!("PING", result.unwrap().as_str());
    }
}
