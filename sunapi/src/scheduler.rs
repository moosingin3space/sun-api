//! Webhook scheduling utilities.

use std::error::Error;

use axum::body::HttpBody;
use http_body_util::BodyExt;

/// Schedules a webhook to be sent at a specific time using a provided HTTP client.
///
/// This function spawns an async task that collects the request body, sleeps until
/// the scheduled time, and then sends the request via the provided client.
///
/// # Arguments
///
/// * `now` - The current time (used to determine if the webhook is in the past)
/// * `when` - The time when the webhook should be sent
/// * `req` - The HTTP request to send
/// * `client` - The HTTP client to use for sending the webhook
pub fn schedule_webhook_with_client<B, C>(
    now: jiff::Timestamp,
    when: jiff::Timestamp,
    req: http::Request<B>,
    client: C,
) where
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Error + Send + Sync,
    C: Send + 'static,
    C: Fn(
        http::Request<Vec<u8>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>>,
{
    if when < now {
        // No point in scheduling a webhook in the past
        return;
    }

    let start = tokio::time::Instant::now();
    // Spawn the async task
    tokio::spawn(async move {
        let (parts, body) = req.into_parts();
        // Collect the streaming body into bytes
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => return,
        };

        let http_req = http::Request::from_parts(parts, body_bytes.to_vec());

        // Sleep until the scheduled time
        let time_delta = now
            .until(when)
            .ok()
            .and_then(|d| std::time::Duration::try_from(d).ok());

        if let Some(delta) = time_delta {
            let desired_instant = start + delta;
            tokio::time::sleep_until(desired_instant).await;
        }

        client(http_req).await;
    });
}
