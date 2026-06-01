use crate::domain::limit::repository::LimitRepsiotry;
use crate::infrastructure::db::client::RateLimitDbClient;
use crate::infrastructure::db::client::client::RateLimitClient;
use fred::prelude::*;
use fred::types::Value;
use std::sync::Arc;
use std::vec::Vec;
use tokio::time::{Duration, timeout};

const MAX_REQUESTS: u64 = 100;

pub struct LimitRepositoryImpl {
    client: Arc<dyn RateLimitClient>,
}
impl LimitRepositoryImpl {
    pub fn new(client: Arc<dyn RateLimitClient>) -> Self {
        Self { client }
    }
}

impl LimitRepsiotry for LimitRepositoryImpl {
    async fn check_rate_limit(&self, user_ip: &str, destination_ip: &str, window: &u64) -> Result<bool, Error> {
        let connection = self.client.get_ref();
        let pipeline = connection.pipeline();
        let key = format!("{}:{}:{}", user_ip, destination_ip, window);
        pipeline.incr::<(), _>(&key).await?;
        pipeline.expire::<(), _>(&key, 60, None).await?;
        let pipeline_future = pipeline.all::<Vec<Value>>();

        let mut results: Vec<Value> = match timeout(Duration::from_secs(1), pipeline_future).await {
            Ok(Ok(results)) => results,
            Ok(Err(e)) => {
                println!("Dragonfly error: {}", e);
                return Ok(false);
            }
            Err(_) => {
                println!("Timeout! Failing open.");
                return Ok(false);
            }
        };
        let count: Option<u64> = results.remove(0).convert().unwrap_or(None);
        count
            .map(|c| c >= MAX_REQUESTS)
            .ok_or_else(|| Error::new(ErrorKind::Unknown, "failed to read count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db::client::RateLimitDbClient;
    use crate::infrastructure::db::repository::limit::limit_repository::LimitRepositoryImpl;
    use std::env;
    use crate::application::request::services::window_builder::WindowBuilder;
    use async_trait::async_trait;

    struct FailingClientOnGetRef;
    impl FailingClientOnGetRef {
        pub fn new(_: Config) -> Self { FailingClientOnGetRef }
    }

    #[async_trait]
    impl RateLimitClient for FailingClientOnGetRef {
        async fn enable_client(&self) {}
        fn get_ref(&self) -> &Client {
            panic!("no real client")
        }
    }

    #[tokio::test]
    async fn test_checks_rate_limit_when_doesnt_exist() {
        let binding = env::var("REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
        let redis_url = binding.as_str();
        let config = Config::from_url(redis_url).unwrap();
        let client = Arc::new(RateLimitDbClient::new(config));
        let window_builder = WindowBuilder::new();

        client.enable_client().await;

        let repo = LimitRepositoryImpl::new(client.clone());
        let connection = client.get_ref();

        let fake_user_ip = "104.15.59.171";
        let fake_destination_ip = "104.15.59.172";
        let window = window_builder.build();

        let exposed_limit = repo
            .check_rate_limit(fake_user_ip, fake_destination_ip, &window)
            .await
            .unwrap();
        assert!(exposed_limit.eq(&false));
        connection.flushall::<()>(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_checks_rate_limit_should_be_two() {
        let binding = env::var("REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
        let redis_url = binding.as_str();
        let config = fred::prelude::Config::from_url(redis_url).unwrap();
        let client = Arc::new(RateLimitDbClient::new(config));
        let window_builder = WindowBuilder::new();

        client.enable_client().await;

        let repo = LimitRepositoryImpl::new(client.clone());
        let connection = client.get_ref();

        let fake_user_ip = "104.15.59.173";
        let fake_destination_ip = "104.15.59.174";
        let window = window_builder.build();
        let key = format!("{}:{}:{}", fake_user_ip, fake_destination_ip, window);

        let exposed_limit = repo
            .check_rate_limit(fake_user_ip, fake_destination_ip, &window)
            .await
            .unwrap();
        assert!(exposed_limit.eq(&false));

        let count: u64 = connection.get(&key).await.unwrap();
        assert_eq!(count, 1);

        let exposed_limit = repo
            .check_rate_limit(fake_user_ip, fake_destination_ip, &window)
            .await
            .unwrap();
        assert!(exposed_limit.eq(&false));

        let count: u64 = connection.get(&key).await.unwrap();
        assert_eq!(count, 2);

        connection.flushall::<()>(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_disallows_after_100() {
        let binding = env::var("REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
        let redis_url = binding.as_str();
        let config = Config::from_url(redis_url).unwrap();
        let client = Arc::new(RateLimitDbClient::new(config));
        let window_builder = WindowBuilder::new();

        client.enable_client().await;

        let repo = LimitRepositoryImpl::new(client.clone());
        let connection = client.get_ref();

        let fake_user_ip = "104.15.59.173";
        let fake_destination_ip = "104.15.59.174";
        let window = window_builder.build();
        let key = format!("{}:{}:{}", fake_user_ip, fake_destination_ip, window);

        for _ in 0..100 {
            let exposed_limit = repo
                .check_rate_limit(fake_user_ip, fake_destination_ip, &window)
                .await
                .unwrap();
        }
        let exposed_limit = repo
            .check_rate_limit(fake_user_ip, fake_destination_ip, &window)
            .await
            .unwrap();
        assert!(exposed_limit.eq(&true));
        connection.flushall::<()>(false).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_checks_behavior_on_failed_connection() {
        let config = Config::from_url("redis://localhost:9999").unwrap();
        let client = Arc::new(RateLimitDbClient::new(config));
        let window_builder = WindowBuilder::new();

        let fake_user_ip = "104.15.59.173";
        let fake_destination_ip = "104.15.59.174";
        let window = window_builder.build();

        let repo = LimitRepositoryImpl::new(client.clone());

        let result = repo.check_rate_limit(fake_user_ip, fake_destination_ip, &window).await;

        assert!(result.is_err());
    }
}
