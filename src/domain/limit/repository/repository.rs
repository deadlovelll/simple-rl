use fred::error::Error;
use async_trait::async_trait;
pub trait LimitRepsiotry {
    async fn check_rate_limit(&self, user_ip: &str, destination_ip: &str, window: &u64) -> Result<bool, Error>;
}
