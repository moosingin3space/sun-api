use crate::Platform;

pub struct SimPlatform {
    pub current_time: jiff::Timestamp,
}

impl Platform for SimPlatform {
    fn now(&self) -> jiff::Timestamp {
        self.current_time
    }

    fn schedule_webhook_at<B>(&self, _when: jiff::Timestamp, _req: http::Request<B>)
    where
        B: axum::body::HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: std::error::Error + Send + Sync,
    {
    }
}
