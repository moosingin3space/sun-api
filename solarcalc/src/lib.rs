#![doc = "../README.md"]
#![no_std]

mod solarpos;

use core::fmt;

pub use jiff::civil::Date;
use jiff::tz::TimeZone;

/// Represents a degree-minute-second (DMS) coordinate.
#[derive(Clone, Copy)]
pub struct Dms {
    /// Integer degrees of the coordinate.
    pub degrees: i8,
    /// Integer minutes of the coordinate.
    pub minutes: u8,
    /// Integer seconds of the coordinate.
    pub seconds: u8,
}

impl Dms {
    /// Creates a new DMS coordinate from decimal degrees.
    pub fn from_decimal_degrees(degrees: f64) -> Self {
        let sign = degrees.signum() as i8;
        let degrees = degrees.abs();
        let minutes = (degrees * 60.0).floor() as u8;
        let seconds = ((degrees * 3600.0) % 60.0).floor() as u8;
        Self {
            degrees: sign * (degrees - minutes as f64 / 60.0 - seconds as f64 / 3600.0) as i8,
            minutes,
            seconds,
        }
    }

    /// Returns the coordinate in radians.
    pub fn radians(&self) -> f64 {
        let sign = self.degrees.signum() as f64;
        sign * (self.degrees.abs() as f64 * core::f64::consts::PI / 180.0
            + self.minutes as f64 * core::f64::consts::PI / 180.0 / 60.0
            + self.seconds as f64 * core::f64::consts::PI / 180.0 / 3600.0)
    }

    /// Returns the coordinate in fractional degrees.
    pub fn degrees(&self) -> f64 {
        let sign = self.degrees.signum() as f64;
        sign * (self.degrees.abs() as f64
            + self.minutes as f64 / 60.0
            + self.seconds as f64 / 3600.0)
    }
}

impl fmt::Debug for Dms {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}°{}'{}\"", self.degrees, self.minutes, self.seconds)
    }
}

impl fmt::Display for Dms {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}°{}'{}\"", self.degrees, self.minutes, self.seconds)
    }
}

/// Represents a location on Earth.
#[derive(Debug, Clone)]
pub struct Location {
    /// Latitude coordinate.
    pub latitude: Dms,
    /// Longitude coordinate.
    pub longitude: Dms,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} by {}", self.latitude, self.longitude)
    }
}

/// Returns the solar noon timestamp for the given date.
pub fn solar_noon(date: Date, location: &Location) -> jiff::Timestamp {
    let long_degrees = location.longitude.degrees();
    let solar_noon_minutes = 720.0 - 4.0 * long_degrees - solarpos::equation_of_time(date);
    let solar_noon_hours = solar_noon_minutes / 60.0;
    let solar_noon_hour = solar_noon_hours.floor() as i8;
    let solar_noon_minute = (solar_noon_hours.fract() * 60.0).floor() as i8;
    let solar_noon_second = ((solar_noon_hours.fract() * 60.0).fract() * 60.0).floor() as i8;
    let time = jiff::civil::Time::midnight()
        .with()
        .hour(solar_noon_hour)
        .minute(solar_noon_minute)
        .second(solar_noon_second)
        .build()
        .unwrap();
    let datetime = jiff::civil::DateTime::from_parts(date, time);
    datetime.to_zoned(TimeZone::UTC).unwrap().timestamp()
}

/// Returns the sunrise timestamp for the given date.
pub fn sunrise(date: Date, location: &Location) -> jiff::Timestamp {
    let long_degrees = location.longitude.degrees();
    let ha = solarpos::hour_angle(date, location.latitude.radians());
    let sunrise_minutes = 720.0 - 4.0 * (long_degrees + ha) - solarpos::equation_of_time(date);
    let sunrise_hours = sunrise_minutes / 60.0;
    let sunrise_hour = sunrise_hours.floor() as i8;
    let sunrise_minute = (sunrise_hours.fract() * 60.0).floor() as i8;
    let sunrise_second = ((sunrise_hours.fract() * 60.0).fract() * 60.0).floor() as i8;
    let time = jiff::civil::Time::midnight()
        .with()
        .hour(sunrise_hour)
        .minute(sunrise_minute)
        .second(sunrise_second)
        .build()
        .unwrap();
    let datetime = jiff::civil::DateTime::from_parts(date, time);
    datetime.to_zoned(TimeZone::UTC).unwrap().timestamp()
}

/// Returns the sunset timestamp for the given date.
pub fn sunset(date: Date, location: &Location) -> jiff::Timestamp {
    let long_degrees = location.longitude.degrees();
    let ha = solarpos::hour_angle(date, location.latitude.radians());
    let sunset_minutes = 720.0 - 4.0 * (long_degrees - ha) - solarpos::equation_of_time(date);
    let (sunset_minutes, next_day) = if sunset_minutes > 1440.0 {
        (sunset_minutes - 1440.0, true)
    } else {
        (sunset_minutes, false)
    };
    let sunset_hours = sunset_minutes / 60.0;
    let sunset_hour = sunset_hours.floor() as i8;
    let sunset_minute = (sunset_hours.fract() * 60.0).floor() as i8;
    let sunset_second = ((sunset_hours.fract() * 60.0).fract() * 60.0).floor() as i8;
    let time = jiff::civil::Time::midnight()
        .with()
        .hour(sunset_hour)
        .minute(sunset_minute)
        .second(sunset_second)
        .build()
        .unwrap();
    let date = if next_day {
        date.tomorrow().unwrap()
    } else {
        date
    };
    let datetime = jiff::civil::DateTime::from_parts(date, time);
    datetime.to_zoned(TimeZone::UTC).unwrap().timestamp()
}

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};
    use jiff::civil::Date;

    use crate::{Dms, Location};

    #[test]
    fn solar_calculations() {
        const SAN_FRANCISCO: Location = Location {
            latitude: Dms {
                degrees: 37,
                minutes: 48,
                seconds: 0,
            },
            longitude: Dms {
                degrees: -122,
                minutes: 24,
                seconds: 0,
            },
        };

        // In November, the days are short...
        check(
            jiff::civil::date(2025, 11, 19),
            SAN_FRANCISCO,
            Check {
                noon: expect![[r#"
                    2025-11-19T19:55:17Z
                "#]],
                sunrise: expect![[r#"
                    2025-11-19T14:53:41Z
                "#]],
                sunset: expect![[r#"
                    2025-11-20T00:56:52Z
                "#]],
            },
        );

        // ... and in July, they are long.
        check(
            jiff::civil::date(2025, 7, 4),
            SAN_FRANCISCO,
            Check {
                noon: expect![[r#"
                    2025-07-04T20:13:38Z
                "#]],
                sunrise: expect![[r#"
                    2025-07-04T12:52:02Z
                "#]],
                sunset: expect![[r#"
                    2025-07-05T03:35:15Z
                "#]],
            },
        );
    }

    struct Check {
        noon: Expect,
        sunrise: Expect,
        sunset: Expect,
    }

    #[track_caller]
    fn check(date: Date, location: super::Location, expected: Check) {
        let noon = super::solar_noon(date, &location);
        let sunrise = super::sunrise(date, &location);
        let sunset = super::sunset(date, &location);
        expected.noon.assert_debug_eq(&noon);
        expected.sunrise.assert_debug_eq(&sunrise);
        expected.sunset.assert_debug_eq(&sunset);
    }
}
