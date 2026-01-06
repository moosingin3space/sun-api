//! SunAPI in a Cloudflare Worker

mod webhook_durable;

use http_body_util::BodyExt;
use sunapi::Platform;
use tower::ServiceExt as _;
use worker::*;

struct CloudflarePlatform {
    env: Env,
    ctx: Context,
}

impl Platform for CloudflarePlatform {
    fn now(&self) -> jiff::Timestamp {
        jiff::Timestamp::now()
    }

    fn schedule_webhook_at<B>(&self, when: jiff::Timestamp, req: axum::http::Request<B>)
    where
        B: axum::body::HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: std::error::Error + Send + Sync,
    {
        let env = self.env.clone();

        // Schedule the webhook task to run to completion before the request times out
        self.ctx.wait_until(async move {
            let _ = schedule_webhook_impl(when, req, env).await;
        });
    }
}

/// Implementation of webhook scheduling that can be awaited.
async fn schedule_webhook_impl<B>(
    when: jiff::Timestamp,
    req: axum::http::Request<B>,
    env: Env,
) -> Result<()>
where
    B: axum::body::HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: std::error::Error + Send + Sync,
{
    // Get the durable object namespace from the environment
    let namespace = env.durable_object("SCHEDULED_WEBHOOK")?;

    // Extract request details
    let (parts, body) = req.into_parts();

    // Collect the body into bytes
    let collected = body.collect().await.map_err(|_| "Failed to collect body")?;
    let body_bytes = collected.to_bytes();
    let body_str = if body_bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body_bytes).to_string())
    };

    // Extract headers into a HashMap
    let mut headers = std::collections::HashMap::new();
    for (name, value) in parts.headers.iter() {
        if let Ok(val_str) = value.to_str() {
            headers.insert(name.as_str().to_string(), val_str.to_string());
        }
    }

    // Convert axum::http::Method to worker::Method
    let method = match parts.method {
        axum::http::Method::GET => worker::Method::Get,
        axum::http::Method::POST => worker::Method::Post,
        axum::http::Method::PUT => worker::Method::Put,
        axum::http::Method::DELETE => worker::Method::Delete,
        axum::http::Method::PATCH => worker::Method::Patch,
        axum::http::Method::HEAD => worker::Method::Head,
        axum::http::Method::OPTIONS => worker::Method::Options,
        _ => worker::Method::Post,
    };

    // Build the webhook request
    let webhook_request = webhook_durable::WebhookRequest {
        url: parts.uri.to_string(),
        method,
        body: body_str,
        headers,
        scheduled_at: when,
    };

    // Serialize the webhook request
    let webhook_json = serde_json::to_string(&webhook_request)?;

    // Create a POST request to save the webhook to the durable object
    let post_req = worker::Request::new_with_init(
        "https://durable-object/webhooks",
        worker::RequestInit::default()
            .with_method(worker::Method::Post)
            .with_body(Some(webhook_json.into())),
    )?;

    // Get a unique ID for this webhook based on the scheduled time
    let object_id = namespace.id_from_name(&when.to_string())?;
    let stub = object_id.get_stub()?;

    // Send the POST request to the durable object
    let _response = stub.fetch_with_request(post_req).await?;

    Ok(())
}

#[event(fetch)]
async fn fetch(req: HttpRequest, env: Env, ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let plat = CloudflarePlatform { env, ctx };
    let router = sunapi::create_router(plat);

    match router.oneshot(req).await {
        Ok(response) => {
            let (parts, body) = response.into_parts();
            let bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(b) => b,
                Err(_e) => return Response::error("Failed to read response body", 500),
            };

            let body_str = String::from_utf8_lossy(&bytes);

            // Create response with the actual status code
            let mut resp = if parts.status.is_success() {
                Response::ok(&*body_str)?
            } else {
                Response::error(&*body_str, parts.status.as_u16())?
            };

            // Copy headers from Axum response
            for (name, value) in parts.headers.iter() {
                if let Ok(val_str) = value.to_str()
                    && name.as_str() != "content-length"
                {
                    // Skip content-length as it will be recalculated
                    resp.headers_mut().set(name.as_ref(), val_str)?;
                }
            }

            Ok(resp)
        }
        Err(_e) => Response::error("Internal server error", 500),
    }
}
