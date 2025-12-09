#![doc = "../README.md"]

use std::sync::Arc;

use axum::{
    Json, Router,
    body::HttpBody,
    extract::Query,
    routing::{get, post},
};
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use solarcalc::Coordinate;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[cfg(feature = "tokio")]
pub mod scheduler;

/// A platform that the Sun API runs on.
pub trait Platform {
    /// Retrieves the current timestamp.
    fn now(&self) -> jiff::Timestamp;

    /// Schedules an outbound webhook to be sent at a specific time.
    fn schedule_webhook_at<B>(&self, when: jiff::Timestamp, req: http::Request<B>)
    where
        B: HttpBody + Send + 'static,
        B::Data: Send,
        B::Error: std::error::Error + Send + Sync;
}

/// Creates an Axum router for the Sun API.
pub fn create_router(plat: impl Platform + Send + Sync + 'static) -> Router {
    // Create a shared platform instance, which can be threaded through the
    // ad-hoc handlers created below.
    let plat = Arc::new(plat);

    Router::new()
        .route(
            "/sun-state",
            get({
                let plat = Arc::clone(&plat);
                move |Query(params): Query<SunStateParams>| {
                    let plat = Arc::clone(&plat);
                    async move { sun_state_inner(plat, params.lat, params.lon, params.tz).await }
                }
            }),
        )
        .route(
            "/set-webhook",
            post({
                let plat = Arc::clone(&plat);
                move |Json(body): Json<SetWebhookRequest>| {
                    let plat = Arc::clone(&plat);
                    async move { set_webhook_inner(plat, body.url, body.lat, body.lon).await }
                }
            }),
        )
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
#[openapi(paths(sun_state_inner, set_webhook_inner))]
struct Api;

#[derive(Deserialize, IntoParams)]
struct SunStateParams {
    lat: f64,
    lon: f64,
    #[serde(default)]
    tz: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct SetWebhookRequest {
    /// The webhook URL to be called at the next sun state change.
    url: String,
    /// Latitude of the location.
    lat: f64,
    /// Longitude of the location.
    lon: f64,
}

#[derive(Serialize, ToSchema)]
struct SetWebhookResponse {
    /// Success message.
    message: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
enum SunEvent {
    /// The sun has risen.
    Sunrise,
    /// The sun has set.
    Sunset,
}

#[derive(Serialize, ToSchema)]
struct WebhookPayload {
    /// The type of sun event that occurred.
    event: SunEvent,
    /// Latitude of the location.
    lat: f64,
    /// Longitude of the location.
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
async fn sun_state_inner<P: Platform + ?Sized>(
    platform: Arc<P>,
    lat: f64,
    lon: f64,
    tz: Option<String>,
) -> Json<SunState> {
    let time = platform.now();
    let location = solarcalc::Location {
        latitude: Coordinate::from(lat),
        longitude: Coordinate::from(lon),
    };
    let tz = tz
        .as_deref()
        .and_then(|s| TimeZone::get(s).ok())
        .unwrap_or(TimeZone::UTC);
    let date = time.to_zoned(tz.clone()).date();
    let time_at_location = time.to_zoned(tz);

    let sunrise_time = solarcalc::sunrise(date, &location);
    let sunset_time = solarcalc::sunset(date, &location);
    Json(SunState {
        sun_up: time >= sunrise_time && time < sunset_time,
        time: time_at_location,
    })
}

/// Schedules a webhook to be called at the next sun state change.
#[utoipa::path(
    post,
    path = "/set-webhook",
    request_body = SetWebhookRequest,
    responses(
        (status = OK, description = "Webhook scheduled successfully", body = SetWebhookResponse)
    )
)]
async fn set_webhook_inner<P: Platform + ?Sized>(
    platform: Arc<P>,
    url: String,
    lat: f64,
    lon: f64,
) -> Json<SetWebhookResponse> {
    let time = platform.now();
    let location = solarcalc::Location {
        latitude: Coordinate::from(lat),
        longitude: Coordinate::from(lon),
    };
    let date = time.to_zoned(TimeZone::UTC).date();

    let sunrise_time = solarcalc::sunrise(date, &location);
    let sunset_time = solarcalc::sunset(date, &location);

    // Determine the next sun state change
    let (next_change, event) = if time < sunrise_time {
        (sunrise_time, SunEvent::Sunrise)
    } else if time < sunset_time {
        (sunset_time, SunEvent::Sunset)
    } else {
        // Next change is tomorrow's sunrise
        let tomorrow = date.tomorrow().unwrap();
        (solarcalc::sunrise(tomorrow, &location), SunEvent::Sunrise)
    };

    // Create the webhook request body as bytes
    let payload = WebhookPayload { event, lat, lon };
    let body_bytes: axum::body::Body = serde_json::to_string(&payload).unwrap().into();

    let webhook_request = http::Request::builder()
        .method(http::Method::POST)
        .uri(&url)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body_bytes)
        .unwrap();

    platform.schedule_webhook_at(next_change, webhook_request);

    Json(SetWebhookResponse {
        message: "Webhook scheduled successfully".to_string(),
    })
}

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod tests {
    use axum::body::Bytes;
    use expect_test::expect;
    use http::Request;
    use http_body_util::Empty;
    use jiff::civil::date;
    use testresult::TestResult;

    use crate::test_utils::{check_api, check_webhook};

    const SF_LAT: f64 = 37.7749;
    const SF_LON: f64 = -122.4194;
    const SF_TZ: &str = "America/Los_Angeles";

    #[test]
    fn sun_up_for_san_francisco() -> TestResult {
        let uri = http::Uri::builder()
            .scheme("http")
            .authority("localhost")
            .path_and_query(format!("/sun-state?lat={SF_LAT}&lon={SF_LON}&tz={SF_TZ}"))
            .build()?;
        check_api(
            date(2025, 11, 22).at(23, 55, 16, 0).in_tz(SF_TZ)?,
            Request::builder()
                .uri(uri.clone())
                .body(Empty::<Bytes>::new())?,
            expect![[r#"
                Response {
                    status: 200,
                    version: HTTP/1.1,
                    headers: {
                        "content-type": "application/json",
                        "content-length": "72",
                    },
                    body: b"{\"sun_up\":false,\"time\":\"2025-11-22T23:55:16-08:00[America/Los_Angeles]\"}",
                }
            "#]],
        )?;
        check_api(
            date(2025, 11, 22).at(13, 20, 0, 0).in_tz(SF_TZ)?,
            Request::builder().uri(uri).body(Empty::<Bytes>::new())?,
            expect![[r#"
                Response {
                    status: 200,
                    version: HTTP/1.1,
                    headers: {
                        "content-type": "application/json",
                        "content-length": "71",
                    },
                    body: b"{\"sun_up\":true,\"time\":\"2025-11-22T13:20:00-08:00[America/Los_Angeles]\"}",
                }
            "#]],
        )
    }

    #[test]
    fn sunrise_calculation_with_extreme_longitude() -> TestResult {
        // Tokyo has longitude +139.69, which causes negative hours in sunrise calculations
        const TOKYO_LAT: f64 = 35.6762;
        const TOKYO_LON: f64 = 139.6503;
        const TOKYO_TZ: &str = "Asia/Tokyo";
        let uri = http::Uri::builder()
            .scheme("http")
            .authority("localhost")
            .path_and_query(format!(
                "/sun-state?lat={TOKYO_LAT}&lon={TOKYO_LON}&tz={TOKYO_TZ}"
            ))
            .build()?;

        // This request should not panic - it should handle the negative hour case
        check_api(
            date(2025, 1, 2).at(0, 0, 0, 0).in_tz(TOKYO_TZ)?,
            Request::builder().uri(uri).body(Empty::<Bytes>::new())?,
            expect![[r#"
                Response {
                    status: 200,
                    version: HTTP/1.1,
                    headers: {
                        "content-type": "application/json",
                        "content-length": "63",
                    },
                    body: b"{\"sun_up\":false,\"time\":\"2025-01-02T00:00:00+09:00[Asia/Tokyo]\"}",
                }
            "#]],
        )?;
        Ok(())
    }

    #[test]
    fn sunset_webhook_fires_at_sunset() -> TestResult {
        let uri = http::Uri::builder()
            .scheme("http")
            .authority("localhost")
            .path_and_query(format!("/set-webhook?lat={SF_LAT}&lon={SF_LON}",))
            .build()?;

        // Set time to early afternoon on Nov 22, 2025
        let request_time = date(2025, 11, 22).at(14, 0, 0, 0).in_tz(SF_TZ)?;
        let req = Request::builder()
            .uri(uri)
            .method("POST")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                r#"{"url":"http://webhook:9998/webhook","lat":37.7749,"lon":-122.4194}"#,
            ))?;

        // On Nov 22, 2025, sunset in SF is around 17:08:51
        // So we skip to sunset time (about 3 hours 8 minutes after request)
        let time_until_webhook = jiff::Span::new().hours(3).minutes(9);

        check_webhook(
            request_time,
            time_until_webhook,
            req,
            expect![[r#"
                Response {
                    status: 200,
                    version: HTTP/1.1,
                    headers: {
                        "content-type": "application/json",
                        "content-length": "44",
                    },
                    body: b"{\"message\":\"Webhook scheduled successfully\"}",
                }
            "#]],
            expect![[r#"
                Object {
                    "event": String("sunset"),
                    "lat": Number(37.7749),
                    "lon": Number(-122.4194),
                }
            "#]],
        )
    }

    #[test]
    fn sunset_webhook_doesnt_fire_before_sunset() -> TestResult {
        let uri = http::Uri::builder()
            .scheme("http")
            .authority("localhost")
            .path_and_query(format!("/set-webhook?lat={SF_LAT}&lon={SF_LON}",))
            .build()?;

        // Set time to early afternoon on Nov 22, 2025
        let request_time = date(2025, 11, 22).at(14, 0, 0, 0).in_tz(SF_TZ)?;
        let req = Request::builder()
            .uri(uri)
            .method("POST")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                r#"{"url":"http://webhook:9998/webhook","lat":37.7749,"lon":-122.4194}"#,
            ))?;

        // Skip to only 1 hour after request (before sunset at ~17:08)
        let _time_until_webhook = jiff::Span::new().hours(1);

        check_api(
            request_time,
            req,
            expect![[r#"
            Response {
                status: 200,
                version: HTTP/1.1,
                headers: {
                    "content-type": "application/json",
                    "content-length": "44",
                },
                body: b"{\"message\":\"Webhook scheduled successfully\"}",
            }
        "#]],
        )?;

        Ok(())
    }
}
