#![allow(mixed_script_confusables)]

#[cfg_attr(not(test), allow(unused_imports))]
#[macro_use]
extern crate approx;

// Organized modules
pub mod analysis;
pub mod geometry;
pub mod math;
pub mod optimization;

// Re-exports for backwards compatibility
pub use geometry::circle;
pub use geometry::ellipses;
pub use geometry::polygon;
pub use geometry::r2;
pub use geometry::rotate;
pub use geometry::shape;
pub use geometry::transform;

pub use optimization::history;
pub use optimization::model;
pub use optimization::step;
pub use optimization::targets;
pub use optimization::tiered;
pub use optimization::trace;

pub use analysis::component;
pub use analysis::contains;
pub use analysis::distance;
pub use analysis::edge;
pub use analysis::gap;
pub use analysis::hull;
pub use analysis::intersect;
pub use analysis::intersection;
pub use analysis::node;
pub use analysis::region;
pub use analysis::regions;
pub use analysis::scene;
pub use analysis::segment;
pub use analysis::set;
pub use analysis::boundary_coord;

// Re-export math utilities for backwards compatibility
pub use math::d5;
pub use math::float_arr;
pub use math::float_wrap;
pub use math::roots;
pub use math::sqrt;
pub use math::trig;
pub use math::zero;

// Utility modules
pub mod coord_getter;
pub mod dual;
pub mod duals;
pub mod error;
pub mod fmt;
pub mod to;

// Re-export key types for external use
pub use dual::D;
pub use model::Model;
pub use step::Step;
pub use targets::{Targets, TargetsMap};
pub use tiered::{TieredConfig, seek_from_keyframe};
pub use trace::{TraceStorage, TraceMetadata, StorageStrategy, create_storage};
pub use trace::{DenseStorage, TieredStorage, BtdStorage, HybridStorage};
pub use shape::{Shape, InputSpec};
pub use ellipses::xyrr::XYRR;
pub use polygon::Polygon;
pub use r2::R2;

/// Parse a log level string into LevelFilter.
pub fn parse_log_level(level: Option<&str>) -> log::LevelFilter {
    match level {
        Some("error") => log::LevelFilter::Error,
        Some("warn") => log::LevelFilter::Warn,
        Some("info") | Some("") | None => log::LevelFilter::Info,
        Some("debug") => log::LevelFilter::Debug,
        Some("trace") => log::LevelFilter::Trace,
        Some(level) => panic!("invalid log level: {}", level),
    }
}
