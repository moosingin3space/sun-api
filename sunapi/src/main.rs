//! SunAPI with Tokio: a standalone version of the SunAPI.

use std::{error::Error, time::Duration};

use axum::body::HttpBody;
use http_body_util::BodyExt;
use sunapi::Platform;
use tokio::time::Instant;

struct TokioPlatform;

impl Platform for TokioPlatform {
    fn now(&self) -> jiff::Timestamp {
        jiff::Timestamp::now()
    }

    fn schedule_webhook_at<B>(&self, when: jiff::Timestamp, req: http::Request<B>)
    where
        B: HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: Error + Send + Sync,
    {
        if when < jiff::Timestamp::now() {
            // No point in scheduling a webhook in the past
            return;
        }
        let Ok(time_delta) = jiff::Timestamp::now().until(when) else {
            return;
        };
        let Ok(time_delta) = Duration::try_from(time_delta) else {
            return;
        };

        // Schedule the task to convert and send the request.
        let now = Instant::now();
        tokio::spawn(async move {
            let (parts, body) = req.into_parts();
            // Collect the streaming body into bytes
            let body_bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => return,
            };
            let reqwest_body = reqwest::Body::from(body_bytes);
            let reqwest_req = http::Request::from_parts(parts, reqwest_body);

            tokio::time::sleep_until(now + time_delta).await;

            let client = reqwest::Client::new();
            let _ = client
                .execute(reqwest_req.try_into().unwrap_or_else(|_| {
                    panic!("Failed to convert http::Request to reqwest::Request")
                }))
                .await;
        });
    }
}

#[cfg(feature = "tokio")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use tokio::net::TcpListener;

    let plat = TokioPlatform;
    let router = sunapi::create_router(plat);
    let listener = TcpListener::bind(("0.0.0.0", 3000)).await?;
    axum::serve(listener, router.into_make_service()).await?;
    Ok(())
}

#[cfg(not(feature = "tokio"))]
fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("This binary only builds with the `tokio` feature enabled.");
    Ok(())
}
