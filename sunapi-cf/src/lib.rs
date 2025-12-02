//! SunAPI in a Cloudflare Worker

use sunapi::Platform;
use tower_service::Service;
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
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    let plat = CloudflarePlatform;
    let mut router = sunapi::create_router(plat);
    Ok(router.call(req).await?)
}
