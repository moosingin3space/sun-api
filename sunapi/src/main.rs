use std::error::Error;

use sunapi::Platform;

struct TokioPlatform;

impl Platform for TokioPlatform {
    fn now(&self) -> jiff::Timestamp {
        jiff::Timestamp::now()
    }

    fn schedule_webhook_at<B>(&self, _when: jiff::Timestamp, _req: http::Request<B>)
    where
        B: axum::body::HttpBody,
    {
        todo!()
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
