use std::sync::Arc;

use axum::body::HttpBody;
use const_format::formatcp;
use expect_test::Expect;
use http::{Request, Uri, uri::Authority};
use http_body_util::BodyExt;
use hyper_util::rt::{TokioExecutor, TokioTimer};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use testresult::TestResult;
use tokio::sync::Notify;
use tower::BoxError;

pub use self::platform::SimPlatform;
pub mod connector;

mod platform;

#[track_caller]
pub fn check_api<B>(time: jiff::Zoned, req: Request<B>, expected: Expect) -> TestResult
where
    B: HttpBody<Data: Send, Error: Into<BoxError> + Send> + Send + Unpin + 'static,
{
    const SERVER: &str = "server";
    const CLIENT: &str = "client";
    const PORT: u16 = 9999;

    const SERVER_AUTHORITY: &str = formatcp!("{SERVER}:{PORT}");

    let server_up = Arc::new(Notify::new());

    let mut sim = turmoil::Builder::new().build();
    sim.host(SERVER, {
        let server_up = Arc::clone(&server_up);
        let current_time = time.timestamp();
        move || {
            let server_up = Arc::clone(&server_up);
            async move {
                let router = crate::create_router(SimPlatform { current_time });

                let listener =
                    connector::listen(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), PORT))
                        .await?;
                server_up.notify_one();
                axum::serve(listener, router.into_make_service()).await?;
                Ok(())
            }
        }
    });

    sim.client(CLIENT, async move {
        server_up.notified().await;
        let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .timer(TokioTimer::new())
            .build(connector::connector());

        let mut req = req;
        {
            let uri = req.uri().clone();
            let mut parts = uri.into_parts();
            parts.authority = Some(Authority::from_static(SERVER_AUTHORITY));
            *req.uri_mut() = Uri::from_parts(parts).unwrap();
        }
        let res = client.request(req).await?;
        let (parts, body) = res.into_parts();
        let parts = {
            let mut parts = parts;
            parts.headers.remove(http::header::DATE);
            parts
        };
        let body = body.collect().await?.to_bytes();
        let res = http::Response::from_parts(parts, body);
        expected.assert_debug_eq(&res);

        Ok(())
    });

    sim.run()?;
    Ok(())
}
