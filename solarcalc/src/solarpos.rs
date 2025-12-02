//! Solar position calculations.
//! Consult https://gml.noaa.gov/grad/solcalc/solareqns.PDF
//! for more information.
//!
//! Note: we always approximate the "time" in the calculation
//! as noon.

use core::f64::consts::PI;

use jiff::civil::Date;

/// Calculate the fractional year in radians,
/// named gamma in the NOAA document.
fn fractional_year_radians(date: Date) -> f64 {
    let multiplier = 2.0 * PI / date.days_in_year() as f64;
    multiplier * (date.day_of_year() - 1) as f64
}

/// Estimate of the equation of time in minutes,
/// named eqtime in the NOAA document.
pub fn equation_of_time(date: Date) -> f64 {
    let gamma = fractional_year_radians(date);
    229.18
        * (0.000075 + 0.001868 * gamma.cos()
            - 0.032077 * gamma.sin()
            - 0.014615 * (2.0 * gamma).cos()
            - 0.040849 * (2.0 * gamma).sin())
}

/// Estimate of the declination angle in radians,
/// named decl in the NOAA document.
fn declination_angle(date: Date) -> f64 {
    let gamma = fractional_year_radians(date);
    0.006918 - 0.399912 * gamma.cos() + 0.070257 * gamma.sin() - 0.006758 * (2.0 * gamma).cos()
        + 0.000907 * (2.0 * gamma).sin()
        - 0.002697 * (3.0 * gamma).cos()
        + 0.00148 * (3.0 * gamma).sin()
}

/// The solar hour angle in degrees, named ha in the NOAA document.
pub fn hour_angle(date: Date, latitude: f64) -> f64 {
    let fixed_numerator: f64 = 90.833f64.to_radians().cos();
    let decl = declination_angle(date);
    let cos_hour_angle =
        fixed_numerator / (latitude.cos() * decl.cos()) - latitude.tan() * decl.tan();
    cos_hour_angle.acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use expect_test::{Expect, expect};
    use jiff::civil::Date;

    #[test]
    fn equation_of_time() {
        check(
            jiff::civil::date(2025, 11, 19),
            expect![[r#"
            14.312655254932228
        "#]],
        );
        check(
            jiff::civil::date(2025, 7, 4),
            expect![[r#"
            -4.049227066974504
        "#]],
        );
    }

    #[track_caller]
    fn check(date: Date, expected: Expect) {
        let eqtime = super::equation_of_time(date);
        expected.assert_debug_eq(&eqtime);
    }
}
