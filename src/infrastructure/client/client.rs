use reqwest::Client;
use crate::domain::limit::result::{RequestResult, RequestError};
pub(crate) use crate::domain::limit::network_client::NetworkClient;

pub struct HttpClient {
    connection: Client
}

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::new();
        Self { connection: client }
    }
}

impl NetworkClient for HttpClient {
    async fn process_request(&self, destination: &str) -> Result<RequestResult, RequestError> {
        let result = self.connection.get(destination).send().await?;
        let status_code = result.status().as_u16();
        let response = result.text().await?;
        let result = RequestResult { status_code, result: response };
        Ok(result)
    }
}