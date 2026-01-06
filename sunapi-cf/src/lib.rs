//! SunAPI in a Cloudflare Worker

use sunapi::Platform;
use tower::ServiceExt as _;
use worker::*;

struct CloudflarePlatform;

impl Platform for CloudflarePlatform {
    fn now(&self) -> jiff::Timestamp {
        jiff::Timestamp::now()
    }

    fn schedule_webhook_at<B>(&self, _when: jiff::Timestamp, _req: axum::http::Request<B>)
    where
        B: axum::body::HttpBody,
    {
        todo!()
    }
}

#[event(fetch)]
async fn fetch(req: HttpRequest, _env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let plat = CloudflarePlatform;
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
