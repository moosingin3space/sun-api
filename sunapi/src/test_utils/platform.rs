use crate::{Platform, scheduler::schedule_webhook_with_client};
use hyper_util::rt::{TokioExecutor, TokioTimer};

pub struct SimPlatform {
    pub current_time: jiff::Timestamp,
}

impl Platform for SimPlatform {
    fn now(&self) -> jiff::Timestamp {
        self.current_time
    }

    fn schedule_webhook_at<B>(&self, when: jiff::Timestamp, req: http::Request<B>)
    where
        B: axum::body::HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: std::error::Error + Send + Sync,
    {
        schedule_webhook_with_client(when, req, |webhook_req| {
            Box::pin(async move {
                let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .build(super::connector::connector());

                let (parts, body) = webhook_req.into_parts();
                let body = axum::body::Body::from(body);
                let req = http::Request::from_parts(parts, body);
                let _ = client.request(req).await;
            })
        });
    }
}
