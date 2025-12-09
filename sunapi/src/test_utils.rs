use std::{fmt::Debug, sync::Arc, time::Duration};

use axum::{Json, body::HttpBody};
use const_format::formatcp;
use expect_test::Expect;
use http::{Request, StatusCode, Uri, uri::Authority};
use http_body_util::BodyExt;
use hyper_util::rt::{TokioExecutor, TokioTimer};
use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use testresult::TestResult;
use tokio::sync::{Notify, mpsc};
use tower::BoxError;
use tracing::{debug, info, instrument};
use tracing_subscriber::fmt::TestWriter;

pub use self::platform::SimPlatform;
pub mod connector;

mod platform;

struct SimElapsedTime;

impl tracing_subscriber::fmt::time::FormatTime for SimElapsedTime {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        // Prints sim elapsed time. Example: [76ms]
        write!(w, "[{:?}]", turmoil::sim_elapsed().unwrap_or_default())
    }
}

fn with_tracing<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let subscriber = tracing_subscriber::fmt()
        .with_writer(TestWriter::new())
        .with_max_level(tracing::Level::DEBUG)
        .with_timer(SimElapsedTime)
        .finish();

    tracing::subscriber::with_default(subscriber, f)
}

#[track_caller]
pub fn check_api<B>(time: jiff::Zoned, req: Request<B>, expected: Expect) -> TestResult
where
    B: HttpBody<Data: Send, Error: Into<BoxError> + Send> + Send + Unpin + Debug + 'static,
{
    with_tracing(|| {
        const SERVER: &str = "server";
        const CLIENT: &str = "client";
        const PORT: u16 = 9999;

        const SERVER_AUTHORITY: &str = formatcp!("{SERVER}:{PORT}");

        let server_up = Arc::new(Notify::new());

        let mut sim = turmoil::Builder::new().build();
        setup_server(&mut sim, time, Arc::clone(&server_up));

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
            debug!(?req, "Sending client request");
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
    })
}

#[track_caller]
#[instrument(skip_all)]
pub fn check_webhook<B>(
    time: jiff::Zoned,
    time_until_webhook: jiff::Span,
    req: Request<B>,
    response_expected: Expect,
    webhook_body_expected: Expect,
) -> TestResult
where
    B: HttpBody<Data: Send, Error: Into<BoxError> + Send> + Send + Unpin + Debug + 'static,
{
    with_tracing(|| {
        const SERVER: &str = "server";
        const WEBHOOK: &str = "webhook";
        const CLIENT: &str = "client";
        const PORT: u16 = 9999;
        const WEBHOOK_PORT: u16 = 9998;

        const SERVER_AUTHORITY: &str = formatcp!("{SERVER}:{PORT}");

        let server_up = Arc::new(Notify::new());
        let webhook_up = Arc::new(Notify::new());
        let (wh_body_tx, mut wh_body_rx) = mpsc::channel(1);

        // This will require some custom simulation duration finicking.
        // Hmm, turmoil doesn't support simulations with long durations very well.
        let time_to_skip = Duration::try_from(time_until_webhook)?;
        let total_sim_time = time_to_skip + Duration::from_secs(100);
        let tick_duration = total_sim_time / 2000;
        debug!(
            ?time_to_skip,
            ?total_sim_time,
            ?tick_duration,
            "Simulation timing configured"
        );
        let mut sim = turmoil::Builder::new()
            .simulation_duration(total_sim_time)
            .tick_duration(tick_duration)
            .build();
        setup_server(&mut sim, time, Arc::clone(&server_up));

        sim.host(WEBHOOK, {
            let webhook_up = Arc::clone(&webhook_up);
            let wh_body_tx = wh_body_tx.clone();
            move || {
                let webhook_up = Arc::clone(&webhook_up);
                let wh_body_tx = wh_body_tx.clone();
                async move {
                    debug!("Webhook host starting");
                    let router = axum::Router::new().route(
                        "/webhook",
                        axum::routing::post(async move |Json(payload): Json<Value>| {
                            info!(?payload, "Webhook received");
                            let _ = wh_body_tx.send(payload).await;
                            (StatusCode::OK, "")
                        }),
                    );
                    let listener = connector::listen(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        WEBHOOK_PORT,
                    ))
                    .await?;
                    debug!("Webhook listener ready");
                    webhook_up.notify_one();
                    axum::serve(listener, router.into_make_service()).await?;
                    Ok(())
                }
            }
        });

        sim.client(CLIENT, async move {
            debug!("Client waiting for server to start");
            tokio::time::timeout(Duration::from_secs(10), server_up.notified())
                .await
                .map_err(|_| "Server startup timeout")?;
            debug!("Server is up");

            debug!("Client waiting for webhook to start");
            tokio::time::timeout(Duration::from_secs(10), webhook_up.notified())
                .await
                .map_err(|_| "Webhook startup timeout")?;
            debug!("Webhook is up");

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
            debug!(?req, "Sending client request");
            let res = client.request(req).await?;
            debug!("Got response from server");

            let (parts, body) = res.into_parts();
            let parts = {
                let mut parts = parts;
                parts.headers.remove(http::header::DATE);
                parts
            };
            let body = body.collect().await?.to_bytes();
            let res = http::Response::from_parts(parts, body);
            response_expected.assert_debug_eq(&res);

            debug!("Sleeping before webhook check");
            tokio::time::sleep(time_to_skip).await;
            debug!("Sleep completed, waiting for webhook body");

            let wh_body = wh_body_rx.recv().await.ok_or("Webhook body not received")?;
            info!(?wh_body, "Webhook body received");
            webhook_body_expected.assert_debug_eq(&wh_body);

            Ok(())
        });

        info!("Running simulation");
        sim.run()?;
        info!("Simulation completed successfully");
        Ok(())
    })
}

fn setup_server(sim: &mut turmoil::Sim, time: jiff::Zoned, server_up: Arc<Notify>) {
    const SERVER: &str = "server";
    const PORT: u16 = 9999;

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
}
