//! SunAPI with Tokio: a standalone version of the SunAPI.

use std::error::Error;

use axum::body::HttpBody;
use sunapi::Platform;

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
        sunapi::scheduler::schedule_webhook_with_client(self.now(), when, req, |webhook_req| {
            Box::pin(async move {
                let client = reqwest::Client::new();
                let (parts, body) = webhook_req.into_parts();
                let reqwest_body = reqwest::Body::from(body);
                let reqwest_req = http::Request::from_parts(parts, reqwest_body);
                let _ = client
                    .execute(
                        reqwest_req
                            .try_into()
                            .expect("couldn't convert http::Request to reqwest::Request"),
                    )
                    .await;
            })
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
