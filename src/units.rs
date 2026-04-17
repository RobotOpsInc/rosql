//! Unit system for ROSQL — 13 categories with SI normalisation.
//!
//! Engineers write comparisons in physical units (e.g. `WHERE duration > 500 ms`).
//! The parser normalises to SI base units for execution and preserves the
//! engineer's chosen unit for display.

use crate::error::ROSQLError;
use crate::span::SourceLocation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The 13 unit categories supported by ROSQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitCategory {
    Frequency,
    Time,
    Distance,
    Velocity,
    Angle,
    AngularVelocity,
    Electrical,
    Temperature,
    Memory,
    Bandwidth,
    Pressure,
    ForceTorque,
    Geographic,
}

/// How to convert a value from this unit to SI base.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Conversion {
    /// `si_value = raw * factor`
    Linear { factor: f64 },
    /// `si_value = raw * factor + offset` (for temperature)
    Affine { factor: f64, offset: f64 },
}

/// Definition of a single unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitDef {
    /// The symbol as written in ROSQL (e.g. "ms", "km/h", "°C").
    pub symbol: String,
    /// Which category this unit belongs to.
    pub category: UnitCategory,
    /// The SI base unit symbol for this category (e.g. "s", "m", "K").
    pub si_unit: String,
    /// Conversion function to SI.
    pub to_si: Conversion,
}

/// Unit support tier for field validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldTier {
    /// Robot Ops-controlled columns — full unit support, storage unit in field registry.
    Tier1,
    /// ROS2 standard message fields — SI assumed per REP-103.
    Tier2,
    /// Custom fields — unit suffix rejected at parse time.
    Tier3,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl Conversion {
    /// Apply this conversion to a raw value, producing the SI-normalised value.
    pub fn apply(&self, raw: f64) -> f64 {
        match self {
            Conversion::Linear { factor } => raw * factor,
            Conversion::Affine { factor, offset } => raw * factor + offset,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit registry (static)
// ---------------------------------------------------------------------------

static UNIT_TABLE: LazyLock<HashMap<&'static str, UnitDef>> = LazyLock::new(build_unit_table);

fn lin(symbol: &'static str, cat: UnitCategory, si: &str, factor: f64) -> (&'static str, UnitDef) {
    (
        symbol,
        UnitDef {
            symbol: symbol.to_owned(),
            category: cat,
            si_unit: si.to_owned(),
            to_si: Conversion::Linear { factor },
        },
    )
}

fn affine(
    symbol: &'static str,
    cat: UnitCategory,
    si: &str,
    factor: f64,
    offset: f64,
) -> (&'static str, UnitDef) {
    (
        symbol,
        UnitDef {
            symbol: symbol.to_owned(),
            category: cat,
            si_unit: si.to_owned(),
            to_si: Conversion::Affine { factor, offset },
        },
    )
}

fn build_unit_table() -> HashMap<&'static str, UnitDef> {
    use UnitCategory::*;

    let entries: Vec<(&str, UnitDef)> = vec![
        // ── Frequency ───────────────────────────────────────────────
        lin("Hz", Frequency, "Hz", 1.0),
        lin("kHz", Frequency, "Hz", 1_000.0),
        // rpm appears in both Frequency and AngularVelocity.
        // We place it under AngularVelocity (rad/s) since that is the
        // more common robotics interpretation. A separate "rpm" alias
        // for Frequency would need disambiguation and is deferred.

        // ── Time ────────────────────────────────────────────────────
        lin("ns", Time, "s", 1e-9),
        lin("us", Time, "s", 1e-6),
        lin("ms", Time, "s", 1e-3),
        lin("s", Time, "s", 1.0),
        lin("min", Time, "s", 60.0),
        lin("h", Time, "s", 3_600.0),
        lin("days", Time, "s", 86_400.0),
        // ── Distance ────────────────────────────────────────────────
        lin("mm", Distance, "m", 1e-3),
        lin("cm", Distance, "m", 1e-2),
        lin("m", Distance, "m", 1.0),
        lin("km", Distance, "m", 1_000.0),
        lin("in", Distance, "m", 0.0254),
        lin("ft", Distance, "m", 0.3048),
        lin("mi", Distance, "m", 1_609.344),
        lin("nmi", Distance, "m", 1_852.0),
        // ── Velocity ────────────────────────────────────────────────
        lin("m/s", Velocity, "m/s", 1.0),
        lin("km/h", Velocity, "m/s", 1.0 / 3.6),
        lin("mph", Velocity, "m/s", 0.447_04),
        lin("knots", Velocity, "m/s", 0.514_444),
        lin("mm/s", Velocity, "m/s", 1e-3),
        lin("cm/s", Velocity, "m/s", 1e-2),
        // ── Angle ───────────────────────────────────────────────────
        lin("deg", Angle, "rad", PI / 180.0),
        lin("rad", Angle, "rad", 1.0),
        lin("mrad", Angle, "rad", 1e-3),
        lin("\u{00b0}", Angle, "rad", PI / 180.0), // ° symbol
        // ── Angular velocity ────────────────────────────────────────
        lin("rad/s", AngularVelocity, "rad/s", 1.0),
        lin("deg/s", AngularVelocity, "rad/s", PI / 180.0),
        lin("rpm", AngularVelocity, "rad/s", 2.0 * PI / 60.0),
        // ── Electrical (Voltage / Current / Power) ──────────────────
        lin("mV", Electrical, "V", 1e-3),
        lin("V", Electrical, "V", 1.0),
        lin("kV", Electrical, "V", 1_000.0),
        lin("mA", Electrical, "A", 1e-3),
        lin("A", Electrical, "A", 1.0),
        lin("mW", Electrical, "W", 1e-3),
        lin("W", Electrical, "W", 1.0),
        lin("kW", Electrical, "W", 1_000.0),
        // ── Temperature ─────────────────────────────────────────────
        // SI base: Kelvin
        // °C → K: K = C * 1.0 + 273.15
        // °F → K: K = F * (5/9) + (−32 * 5/9 + 273.15) = F * 5/9 + 255.372…
        affine("\u{00b0}C", Temperature, "K", 1.0, 273.15),
        affine("degC", Temperature, "K", 1.0, 273.15),
        affine(
            "\u{00b0}F",
            Temperature,
            "K",
            5.0 / 9.0,
            -32.0 * 5.0 / 9.0 + 273.15,
        ),
        affine(
            "degF",
            Temperature,
            "K",
            5.0 / 9.0,
            -32.0 * 5.0 / 9.0 + 273.15,
        ),
        lin("K", Temperature, "K", 1.0),
        // ── Memory ──────────────────────────────────────────────────
        lin("B", Memory, "B", 1.0),
        lin("KB", Memory, "B", 1_000.0),
        lin("MB", Memory, "B", 1_000_000.0),
        lin("GB", Memory, "B", 1_000_000_000.0),
        // ── Bandwidth ───────────────────────────────────────────────
        lin("B/s", Bandwidth, "B/s", 1.0),
        lin("KB/s", Bandwidth, "B/s", 1_000.0),
        lin("MB/s", Bandwidth, "B/s", 1_000_000.0),
        lin("GB/s", Bandwidth, "B/s", 1_000_000_000.0),
        // ── Pressure ────────────────────────────────────────────────
        lin("Pa", Pressure, "Pa", 1.0),
        lin("kPa", Pressure, "Pa", 1_000.0),
        lin("MPa", Pressure, "Pa", 1_000_000.0),
        lin("bar", Pressure, "Pa", 100_000.0),
        lin("psi", Pressure, "Pa", 6_894.757),
        // ── Force / Torque ──────────────────────────────────────────
        lin("N", ForceTorque, "N", 1.0),
        lin("kN", ForceTorque, "N", 1_000.0),
        lin("Nm", ForceTorque, "Nm", 1.0),
        lin("lb-ft", ForceTorque, "Nm", 1.355_818),
        // ── Geographic ──────────────────────────────────────────────
        // lat/lon are dimensionless coordinates; they don't convert to
        // another unit. We register them so the parser recognises the
        // suffix and Haversine distance can be applied at the driver level.
        lin("lat", Geographic, "lat", 1.0),
        lin("lon", Geographic, "lon", 1.0),
        lin("lng", Geographic, "lon", 1.0),
    ];

    entries.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Look up a unit definition by its symbol string.
pub fn lookup_unit(symbol: &str) -> Option<&'static UnitDef> {
    UNIT_TABLE.get(symbol)
}

/// Convert a raw value from `from_unit` to its SI base unit.
/// Returns `(si_value, si_unit_symbol)`.
pub fn convert_to_si(
    value: f64,
    from_unit: &str,
    location: Option<SourceLocation>,
) -> Result<(f64, String), ROSQLError> {
    let def = lookup_unit(from_unit).ok_or_else(|| ROSQLError::UnitError {
        message: format!("unknown unit '{from_unit}'"),
        location,
    })?;
    Ok((def.to_si.apply(value), def.si_unit.clone()))
}

/// Check whether two unit symbols belong to the same category.
pub fn are_compatible(a: &str, b: &str) -> bool {
    match (lookup_unit(a), lookup_unit(b)) {
        (Some(da), Some(db)) => da.category == db.category,
        _ => false,
    }
}

/// Return all registered unit symbols (for lexer/parser awareness).
pub fn all_unit_symbols() -> Vec<&'static str> {
    UNIT_TABLE.keys().copied().collect()
}

/// Haversine distance between two geographic coordinates in metres.
pub fn haversine_distance(lat1_deg: f64, lon1_deg: f64, lat2_deg: f64, lon2_deg: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;

    let lat1 = lat1_deg.to_radians();
    let lat2 = lat2_deg.to_radians();
    let dlat = (lat2_deg - lat1_deg).to_radians();
    let dlon = (lon2_deg - lon1_deg).to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_M * c
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_si(unit: &str, raw: f64, expected_si: f64, expected_unit: &str) {
        let (si_val, si_unit) = convert_to_si(raw, unit, None).unwrap();
        assert!(
            (si_val - expected_si).abs() < 1e-6,
            "{raw} {unit} → expected {expected_si} {expected_unit}, got {si_val} {si_unit}"
        );
        assert_eq!(si_unit, expected_unit);
    }

    // ── Frequency ───────────────────────────────────────────────────
    #[test]
    fn hz() {
        assert_si("Hz", 1.0, 1.0, "Hz");
    }
    #[test]
    fn khz() {
        assert_si("kHz", 2.5, 2_500.0, "Hz");
    }

    // ── Time ────────────────────────────────────────────────────────
    #[test]
    fn ns() {
        assert_si("ns", 1.0, 1e-9, "s");
    }
    #[test]
    fn us() {
        assert_si("us", 1.0, 1e-6, "s");
    }
    #[test]
    fn ms_to_s() {
        assert_si("ms", 500.0, 0.5, "s");
    }
    #[test]
    fn seconds() {
        assert_si("s", 1.0, 1.0, "s");
    }
    #[test]
    fn minutes() {
        assert_si("min", 2.0, 120.0, "s");
    }
    #[test]
    fn hours() {
        assert_si("h", 1.0, 3_600.0, "s");
    }
    #[test]
    fn days() {
        assert_si("days", 1.0, 86_400.0, "s");
    }

    // ── Distance ────────────────────────────────────────────────────
    #[test]
    fn mm() {
        assert_si("mm", 1_000.0, 1.0, "m");
    }
    #[test]
    fn cm() {
        assert_si("cm", 100.0, 1.0, "m");
    }
    #[test]
    fn meters() {
        assert_si("m", 1.0, 1.0, "m");
    }
    #[test]
    fn km() {
        assert_si("km", 1.0, 1_000.0, "m");
    }
    #[test]
    fn inches() {
        assert_si("in", 1.0, 0.0254, "m");
    }
    #[test]
    fn feet() {
        assert_si("ft", 1.0, 0.3048, "m");
    }
    #[test]
    fn miles() {
        assert_si("mi", 1.0, 1_609.344, "m");
    }
    #[test]
    fn nautical_miles() {
        assert_si("nmi", 1.0, 1_852.0, "m");
    }

    // ── Velocity ────────────────────────────────────────────────────
    #[test]
    fn m_per_s() {
        assert_si("m/s", 1.0, 1.0, "m/s");
    }
    #[test]
    fn km_per_h() {
        assert_si("km/h", 3.6, 1.0, "m/s");
    }
    #[test]
    fn mph() {
        assert_si("mph", 1.0, 0.447_04, "m/s");
    }
    #[test]
    fn knots() {
        assert_si("knots", 1.0, 0.514_444, "m/s");
    }

    // ── Angle ───────────────────────────────────────────────────────
    #[test]
    fn degrees() {
        assert_si("deg", 180.0, PI, "rad");
    }
    #[test]
    fn radians() {
        assert_si("rad", 1.0, 1.0, "rad");
    }
    #[test]
    fn milliradians() {
        assert_si("mrad", 1_000.0, 1.0, "rad");
    }
    #[test]
    fn degree_symbol() {
        assert_si("\u{00b0}", 180.0, PI, "rad");
    }

    // ── Angular velocity ────────────────────────────────────────────
    #[test]
    fn rad_per_s() {
        assert_si("rad/s", 1.0, 1.0, "rad/s");
    }
    #[test]
    fn deg_per_s() {
        assert_si("deg/s", 180.0, PI, "rad/s");
    }
    #[test]
    fn rpm_to_rad_s() {
        assert_si("rpm", 60.0, 2.0 * PI, "rad/s");
    }

    // ── Electrical ──────────────────────────────────────────────────
    #[test]
    fn millivolts() {
        assert_si("mV", 1_000.0, 1.0, "V");
    }
    #[test]
    fn volts() {
        assert_si("V", 1.0, 1.0, "V");
    }
    #[test]
    fn kilovolts() {
        assert_si("kV", 1.0, 1_000.0, "V");
    }
    #[test]
    fn milliamps() {
        assert_si("mA", 1_000.0, 1.0, "A");
    }
    #[test]
    fn amps() {
        assert_si("A", 1.0, 1.0, "A");
    }
    #[test]
    fn milliwatts() {
        assert_si("mW", 1_000.0, 1.0, "W");
    }
    #[test]
    fn watts() {
        assert_si("W", 1.0, 1.0, "W");
    }
    #[test]
    fn kilowatts() {
        assert_si("kW", 1.0, 1_000.0, "W");
    }

    // ── Temperature ─────────────────────────────────────────────────
    #[test]
    fn celsius_zero() {
        assert_si("\u{00b0}C", 0.0, 273.15, "K");
    }
    #[test]
    fn celsius_100() {
        assert_si("\u{00b0}C", 100.0, 373.15, "K");
    }
    #[test]
    fn fahrenheit_32() {
        assert_si("\u{00b0}F", 32.0, 273.15, "K");
    }
    #[test]
    fn fahrenheit_212() {
        assert_si("\u{00b0}F", 212.0, 373.15, "K");
    }
    #[test]
    fn kelvin() {
        assert_si("K", 300.0, 300.0, "K");
    }
    #[test]
    fn degc_alias_zero() {
        assert_si("degC", 0.0, 273.15, "K");
    }
    #[test]
    fn degc_alias_100() {
        assert_si("degC", 100.0, 373.15, "K");
    }
    #[test]
    fn degf_alias_32() {
        assert_si("degF", 32.0, 273.15, "K");
    }
    #[test]
    fn degf_alias_212() {
        assert_si("degF", 212.0, 373.15, "K");
    }

    // ── Memory ──────────────────────────────────────────────────────
    #[test]
    fn bytes() {
        assert_si("B", 1.0, 1.0, "B");
    }
    #[test]
    fn kilobytes() {
        assert_si("KB", 1.0, 1_000.0, "B");
    }
    #[test]
    fn megabytes() {
        assert_si("MB", 1.0, 1_000_000.0, "B");
    }
    #[test]
    fn gigabytes() {
        assert_si("GB", 1.0, 1_000_000_000.0, "B");
    }

    // ── Bandwidth ───────────────────────────────────────────────────
    #[test]
    fn bytes_per_s() {
        assert_si("B/s", 1.0, 1.0, "B/s");
    }
    #[test]
    fn gb_per_s() {
        assert_si("GB/s", 1.0, 1_000_000_000.0, "B/s");
    }

    // ── Pressure ────────────────────────────────────────────────────
    #[test]
    fn pascals() {
        assert_si("Pa", 1.0, 1.0, "Pa");
    }
    #[test]
    fn bar() {
        assert_si("bar", 1.0, 100_000.0, "Pa");
    }
    #[test]
    fn psi() {
        assert_si("psi", 1.0, 6_894.757, "Pa");
    }

    // ── Force / Torque ──────────────────────────────────────────────
    #[test]
    fn newtons() {
        assert_si("N", 1.0, 1.0, "N");
    }
    #[test]
    fn kilonewtons() {
        assert_si("kN", 1.0, 1_000.0, "N");
    }
    #[test]
    fn newton_metres() {
        assert_si("Nm", 1.0, 1.0, "Nm");
    }
    #[test]
    fn lb_ft() {
        assert_si("lb-ft", 1.0, 1.355_818, "Nm");
    }

    // ── Geographic ──────────────────────────────────────────────────
    #[test]
    fn geographic_passthrough() {
        assert_si("lat", 42.0, 42.0, "lat");
        assert_si("lon", -71.0, -71.0, "lon");
        assert_si("lng", -71.0, -71.0, "lon");
    }

    // ── Compatibility ───────────────────────────────────────────────
    #[test]
    fn compatible_same_category() {
        assert!(are_compatible("km", "mi"));
        assert!(are_compatible("ms", "h"));
        assert!(are_compatible("deg", "rad"));
    }

    #[test]
    fn incompatible_different_category() {
        assert!(!are_compatible("km", "s"));
        assert!(!are_compatible("V", "Pa"));
    }

    #[test]
    fn unknown_unit() {
        assert!(!are_compatible("km", "furlongs"));
    }

    // ── Haversine ───────────────────────────────────────────────────
    #[test]
    fn haversine_known_distance() {
        // New York (40.7128, -74.0060) to London (51.5074, -0.1278)
        // Expected: ~5,570 km
        let d = haversine_distance(40.7128, -74.0060, 51.5074, -0.1278);
        assert!((d - 5_570_000.0).abs() < 50_000.0, "got {d}");
    }

    #[test]
    fn haversine_same_point() {
        let d = haversine_distance(0.0, 0.0, 0.0, 0.0);
        assert!(d.abs() < 1e-6);
    }

    // ── Error case ──────────────────────────────────────────────────
    #[test]
    fn unknown_unit_error() {
        let err = convert_to_si(1.0, "furlongs", None).unwrap_err();
        match err {
            ROSQLError::UnitError { message, .. } => {
                assert!(message.contains("furlongs"));
            }
            _ => panic!("expected UnitError"),
        }
    }
}
