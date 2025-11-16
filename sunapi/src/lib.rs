use std::sync::Arc;

use axum::{Extension, Json, Router, body::HttpBody, extract::Query, routing::get};
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use solarcalc::Dms;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// A platform that the Sun API runs on.
pub trait Platform {
    /// Retrieves the current timestamp.
    fn now(&self) -> jiff::Timestamp;

    /// Schedules an outbound webhook to be sent at a specific time.
    fn schedule_webhook_at<B>(&self, when: jiff::Timestamp, req: http::Request<B>)
    where
        B: HttpBody;
}

/// Creates an Axum router for the Sun API.
pub fn create_router<P>(plat: P) -> Router
where
    P: Platform + Send + Sync + 'static,
{
    let plat = Arc::new(plat);
    Router::new()
        .route("/sun-state", get(sun_state::<P>))
        .layer(Extension(plat))
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", Api::openapi()))
}

/// Represents the current state of the sun at a given location.
#[derive(Debug, Serialize, ToSchema)]
struct SunState {
    /// Whether the sun is currently up.
    sun_up: bool,

    /// The current time at the provided location.
    time: jiff::Zoned,
}

#[derive(OpenApi)]
#[openapi(paths(sun_state))]
struct Api;

#[derive(Deserialize, IntoParams)]
struct SunStateParams {
    lat: f64,
    lon: f64,
}

/// Retrieves the current state of the sun at a given location.
#[utoipa::path(
    get,
    path = "/sun-state",
    params(SunStateParams),
    responses(
        (status = OK, description = "Sun state information", body = SunState)
    )
)]
async fn sun_state<P>(
    Query(SunStateParams { lat, lon }): Query<SunStateParams>,
    Extension(platform): Extension<Arc<P>>,
) -> Json<SunState>
where
    P: Platform,
{
    let time = platform.now();
    let lat = Dms::from_decimal_degrees(lat);
    let lon = Dms::from_decimal_degrees(lon);
    let location = solarcalc::Location {
        latitude: lat,
        longitude: lon,
    };
    // TODO the UTC assumption is wrong here
    let date = time.to_zoned(TimeZone::UTC).date();
    let time_at_location = time.to_zoned(TimeZone::UTC);

    let sunrise_time = solarcalc::sunrise(date, &location);
    let sunset_time = solarcalc::sunset(date, &location);
    Json(SunState {
        sun_up: time >= sunrise_time && time < sunset_time,
        time: time_at_location,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::Arc,
    };

    use axum::body::{Bytes, HttpBody};
    use const_format::formatcp;
    use expect_test::{Expect, expect};
    use http::{Request, Uri, uri::Authority};
    use http_body_util::{BodyExt, Empty};
    use hyper_util::rt::{TokioExecutor, TokioTimer};
    use jiff::civil::date;
    use testresult::TestResult;
    use tokio::sync::Notify;
    use tower::BoxError;

    #[test]
    fn sun_up_for_san_francisco() -> TestResult {
        const SF_LAT: f64 = 37.7749;
        const SF_LON: f64 = -122.4194;
        let uri = Uri::builder()
            .scheme("http")
            .authority("localhost")
            .path_and_query(format!("/sun-state?lat={SF_LAT}&lon={SF_LON}"))
            .build()?;
        check_api(
            date(2025, 11, 22)
                .at(23, 55, 16, 0)
                .in_tz("America/Los_Angeles")?,
            Request::builder()
                .uri(uri.clone())
                .body(Empty::<Bytes>::new())?,
            expect![[r#"
                Response {
                    status: 200,
                    version: HTTP/1.1,
                    headers: {
                        "content-type": "application/json",
                        "content-length": "56",
                    },
                    body: b"{\"sun_up\":false,\"time\":\"2025-11-23T07:55:16+00:00[UTC]\"}",
                }
            "#]],
        )?;
        check_api(
            date(2025, 11, 22)
                .at(13, 20, 0, 0)
                .in_tz("America/Los_Angeles")?,
            Request::builder().uri(uri).body(Empty::<Bytes>::new())?,
            expect![[r#"
                Response {
                    status: 200,
                    version: HTTP/1.1,
                    headers: {
                        "content-type": "application/json",
                        "content-length": "55",
                    },
                    body: b"{\"sun_up\":true,\"time\":\"2025-11-22T21:20:00+00:00[UTC]\"}",
                }
            "#]],
        )
    }

    #[track_caller]
    fn check_api<B>(time: jiff::Zoned, req: Request<B>, expected: Expect) -> TestResult
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
                    let router = super::create_router(SimPlatform { current_time });

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

    struct SimPlatform {
        current_time: jiff::Timestamp,
    }

    impl super::Platform for SimPlatform {
        fn now(&self) -> jiff::Timestamp {
            self.current_time
        }

        fn schedule_webhook_at<B>(&self, _when: jiff::Timestamp, _req: http::Request<B>)
        where
            B: axum::body::HttpBody,
        {
        }
    }

    mod connector {
        use std::{
            io,
            net::SocketAddr,
            pin::Pin,
            task::{Context, Poll},
        };

        use http::Uri;
        use hyper_util::rt::TokioIo;
        use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
        use tower::Service;
        use turmoil::net::{TcpListener, TcpStream};

        pub struct Listener(pub TcpListener);
        pub struct Stream(pub TcpStream);

        pub async fn listen(addr: SocketAddr) -> io::Result<Listener> {
            Ok(Listener(TcpListener::bind(addr).await?))
        }

        pub type ConnectFuture = Pin<Box<dyn Future<Output = io::Result<TokioIo<Stream>>> + Send>>;
        pub fn connector()
        -> impl Service<
            Uri,
            Response = TokioIo<Stream>,
            Error = io::Error,
            Future = ConnectFuture,
        > + Clone {
            let connector = |uri: Uri| {
                Box::pin(async move {
                    let stream = TcpStream::connect(uri.authority().unwrap().as_str()).await?;
                    Ok(TokioIo::new(Stream(stream)))
                }) as ConnectFuture
            };
            tower::service_fn(connector)
        }

        impl axum::serve::Listener for Listener {
            type Io = Stream;
            type Addr = SocketAddr;

            async fn accept(&mut self) -> (Self::Io, Self::Addr) {
                self.0.accept().await.map(|(x, y)| (Stream(x), y)).unwrap()
            }

            fn local_addr(&self) -> io::Result<SocketAddr> {
                self.0.local_addr()
            }
        }

        impl AsyncRead for Stream {
            fn poll_read(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                Pin::new(&mut self.0).poll_read(cx, buf)
            }
        }

        impl AsyncWrite for Stream {
            fn poll_write(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<io::Result<usize>> {
                Pin::new(&mut self.0).poll_write(cx, buf)
            }

            fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                Pin::new(&mut self.0).poll_flush(cx)
            }

            fn poll_shutdown(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<io::Result<()>> {
                Pin::new(&mut self.0).poll_shutdown(cx)
            }
        }

        impl hyper_util::client::legacy::connect::Connection for Stream {
            fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
                hyper_util::client::legacy::connect::Connected::new()
            }
        }
    }
}
