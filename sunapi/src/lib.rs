#![doc = "../README.md"]

use std::sync::Arc;

use axum::{Extension, Json, Router, body::HttpBody, extract::Query, routing::get};

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
    #[serde(default)]
    tz: Option<String>,
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
    Query(SunStateParams { lat, lon, tz }): Query<SunStateParams>,
    Extension(platform): Extension<Arc<P>>,
) -> Json<SunState>
where
    P: Platform,
{
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

    use crate::test_utils::check_api;

    #[test]
    fn sun_up_for_san_francisco() -> TestResult {
        const SF_LAT: f64 = 37.7749;
        const SF_LON: f64 = -122.4194;
        const SF_TZ: &str = "America/Los_Angeles";
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
}
