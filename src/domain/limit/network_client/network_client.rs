use crate::domain::limit::result::{RequestError, RequestResult};

pub trait NetworkClient {
    async fn process_request(&self, destination: &str) -> Result<RequestResult, RequestError>;
}