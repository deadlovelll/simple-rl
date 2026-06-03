use crate::application::request::services::window_builder::WindowBuilder;
use crate::domain::limit::repository::LimitRepsiotry;
use crate::domain::limit::network_client::NetworkClient;
use crate::domain::limit::window::Window;
use crate::domain::limit::result::{RequestError, RequestResult};

pub struct ProcessRequestImpl<W: Window, R: LimitRepsiotry, N: NetworkClient> {
    window_builder: W,
    rate_limit_repository: R,
    network_client: N,
}

impl<W: Window, R: LimitRepsiotry, N: NetworkClient> ProcessRequestImpl<W, R, N> {
    pub fn new(window_builder: W, rate_limit_repository: R, network_client: N) -> Self {
        Self { window_builder, rate_limit_repository, network_client }
    }

    pub async fn process(&self, from: &str, to: &str) -> Result<RequestResult, RequestError> {
        let window = self.window_builder.build();
        let can_be_processed = self.rate_limit_repository.check_rate_limit(from, to, &window).await;
        if can_be_processed.is_ok() && can_be_processed? == false {
            return Ok(RequestResult{ status_code: 403, result: "Can't be accessed".to_string() });
        }
        let request_result = self.network_client.process_request(to).await?;
        Ok(request_result)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::Arc;
    use fred::prelude::*;
    use fred::types::scan::{ScanType, Scanner};
    use futures::StreamExt;
    use crate::infrastructure::db::client::client::RateLimitClient;
    use crate::infrastructure::db::client::RateLimitDbClient;
    use crate::infrastructure::db::repository::limit::limit_repository::LimitRepositoryImpl;
    use super::*;

    async fn collect_key(connection: &Client) -> Option<i64> {
        let mut counts_stream = connection.scan("*", Some(1), Some(ScanType::String));
        let mut count: Option<i64> = Some(0);
        while let Some(result) = counts_stream.next().await {
            let mut page = result.unwrap();
            if let Some(keys) = page.results() {
                for key in keys {
                    count = connection.get(key).await.unwrap();
                    return count;
                }
            }

        }
        count
    }

    #[tokio::test]
    async fn test_process_request() {
        let binding = env::var("REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
        let redis_url = binding.as_str();
        let config = Config::from_url(redis_url).unwrap();
        let client = Arc::new(RateLimitDbClient::new(config));
        let window_builder = WindowBuilder::new();
        let repo = LimitRepositoryImpl::new(client.clone());
        let interactor = ProcessRequestImpl::new(window_builder, repo);
        client.enable_client().await;

        let connection = client.get_ref();
        let fake_user_ip = "104.15.59.178";
        let fake_destination_ip = "104.15.59.179";
        let result = interactor.process(fake_user_ip, fake_destination_ip).await;
        let count = collect_key(connection).await.unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
        assert_eq!(count, 1);

        connection.flushall::<()>(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_process_request_does_not_bypass_after_100_times() {
        let binding = env::var("REDIS_URL").unwrap_or("redis://localhost:6379".to_string());
        let redis_url = binding.as_str();
        let config = Config::from_url(redis_url).unwrap();
        let client = Arc::new(RateLimitDbClient::new(config));
        let window_builder = WindowBuilder::new();
        let repo = LimitRepositoryImpl::new(client.clone());
        let interactor = ProcessRequestImpl::new(window_builder, repo);
        client.enable_client().await;

        let connection = client.get_ref();
        let fake_user_ip = "104.15.59.170";
        let fake_destination_ip = "104.15.59.169";

        for _ in 0..100 {
            interactor.process(fake_user_ip, fake_destination_ip).await;
        }

        let result_after = interactor.process(fake_user_ip, fake_destination_ip).await;
        let count = collect_key(connection).await.unwrap();

        assert!(result_after.is_ok());
        assert_eq!(result_after.unwrap(), true);
        assert_eq!(count, 101);

        connection.flushall::<()>(false).await.unwrap();
    }
}