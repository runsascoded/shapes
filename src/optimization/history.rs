
use serde::{Serialize, Deserialize};
use tsify::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{shape::Shape, step::Step, model::Model};

/// Namespacing apparently necessary to avoid colliding with step::Step in apvd.d.ts: https://github.com/madonoharu/tsify/issues/36
#[derive(Debug, Clone, PartialEq, Tsify, Serialize, Deserialize)]
pub struct HistoryStep {
    pub error: f64,
    pub shapes: Vec<Shape<f64>>,
}

impl From<Step> for HistoryStep {
    fn from(s: Step) -> Self {
        HistoryStep {
            error: s.error.v(),
            shapes: s.shapes.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl HistoryStep {
    pub fn names(&self) -> Vec<String> {
        self
        .shapes
        .iter()
        .enumerate()
        .flat_map(|(shape_idx, shape)|
            shape
            .names()
            .iter()
            .map(|name| format!("{}.{}", shape_idx, name))
            .collect::<Vec<_>>()
        )
        .collect()
    }
    pub fn vals(&self) -> Vec<f64> {
        self.shapes.iter().flat_map(|s| s.vals()).collect()
    }
}

#[derive(Debug, Clone, derive_more::Deref, Tsify, Serialize, Deserialize)]
pub struct History(pub Vec<HistoryStep>);

impl From<Model> for History {
    fn from(m: Model) -> Self {
        Self(m.steps.into_iter().map(|s| s.into()).collect())
    }
}

#[cfg(test)]
pub use csv_io::*;

// CSV load/save functionality - only available in tests (polars is a dev-dependency)
#[cfg(test)]
mod csv_io {
    /// Count decimal places in a numeric string (for precision-aware comparison)
    pub fn decimal_places(s: &str) -> usize {
        if let Some(dot_pos) = s.find('.') {
            s.len() - dot_pos - 1
        } else {
            0
        }
    }

    /// Round f64 to specified decimal places, return as string
    pub fn round_to_string(val: f64, places: usize) -> String {
        format!("{:.prec$}", val, prec = places)
    }

    /// Compare f64 to expected string, rounding actual to same precision as expected
    pub fn precision_eq(actual: f64, expected_str: &str) -> bool {
        let places = decimal_places(expected_str);
        let rounded_actual = round_to_string(actual, places);
        rounded_actual == expected_str
    }
    use std::collections::BTreeMap;
    use std::path::Path;
    use super::*;
    use polars::{prelude::*, io::SerReader, series::SeriesIter};
    use anyhow::Result;
    use AnyValue::Float64;

    static ERROR_COL: &str = "error";

    #[derive(Debug, thiserror::Error)]
    pub enum LoadErr {
        #[error("Expected first col \"error\", found {0}")]
        UnexpectedFirstCol(String),
        #[error("Expected col of form \"<shape_idx>.<coord>\", found {0}")]
        InvalidCol(String),
        #[error("Expected Float64 in col {0}, got {1}")]
        InvalidVal(usize, String),
    }
    use LoadErr::{UnexpectedFirstCol, InvalidCol, InvalidVal};

    impl History {
        pub fn load(path: &str) -> Result<History> {
            let mut df = CsvReadOptions::default()
                .with_has_header(true)
                .try_into_reader_with_file_path(Some(path.into()))?
                .finish()?;
            df.as_single_chunk_par();
            let mut iters = df.iter().map(|s| (s.name(), s.iter()));
            let (err_name, mut err_iter) = iters.next().unwrap();
            if err_name != ERROR_COL {
                return Err(UnexpectedFirstCol(err_name.to_string()).into());
            }
            let (names, mut val_iters): (Vec<_>, Vec<_>) = iters.unzip();
            let num_coords = names.len();
            let mut shape_coord_specs: BTreeMap<usize, Vec<_>> = BTreeMap::new();
            for (coord_idx, name) in names.iter().enumerate() {
                let parts: Vec<_> = name.split('.').collect();
                if parts.len() != 2 {
                    return Err(InvalidCol(name.to_string()).into());
                }
                let shape_idx = parts[0].parse::<usize>()?;
                let coord = parts[1];
                shape_coord_specs.entry(shape_idx).or_insert_with(std::vec::Vec::new).push((coord_idx, coord));
            }

            let num_rows = df.height();
            let mut steps: Vec<HistoryStep> = Vec::new();
            let next = |row_idx: usize, col_idx: usize, iter: &mut SeriesIter| -> Result<f64, LoadErr> {
                match iter.next().unwrap_or_else(|| panic!("col {} should have at least {} rows, found {}", col_idx, num_rows, row_idx)) {
                    Float64(f) => Ok(f),
                    v => Err(InvalidVal(col_idx, format!("{:?}", v))),
                }
            };
            for row_idx in 0..num_rows {
                let error = next(row_idx, 0, &mut err_iter)?;
                let mut vals: Vec<f64> = Vec::new();
                for (j, iter) in val_iters.iter_mut().enumerate() {
                    let val = next(row_idx, j + 1, iter)?;
                    vals.push(val);
                }
                if vals.len() != num_coords {
                    panic!("Expected {} columns, got {}: {:?}", num_coords, vals.len(), vals);
                }
                let shapes: Vec<Shape<f64>> = shape_coord_specs.values().map(|coord_specs| {
                    let coords: Vec<(&str, f64)> = coord_specs.iter().map(|(coord_idx, coord)| {
                        (*coord, vals[*coord_idx])
                    }).collect();
                    Shape::from_coords(coords)
                }).collect::<Result<Vec<_>, _>>()?;
                steps.push(HistoryStep { error, shapes });
            }
            Ok(History(steps))
        }

        pub fn save(self, path: &str) -> Result<DataFrame> {
            let mut cols: Vec<Vec<f64>> = vec![];
            let first = &self[0];
            let mut col_names = first.names();
            col_names.insert(0, ERROR_COL.to_string());
            let num_columns = col_names.len();
            for _ in 0..num_columns {
                cols.push(vec![]);
            }
            let path = Path::new(&path);
            let dir = path.parent().unwrap();
            std::fs::create_dir_all(dir)?;
            for step in self.0 {
                cols[0].push(step.error);
                let vals = step.vals();
                for (j, val) in vals.into_iter().enumerate() {
                    cols[j + 1].push(val);
                }
            }

            let series: Vec<Column> = cols.into_iter().enumerate().map(|(j, col)| {
                let col_name = col_names.get(j).unwrap_or_else(|| panic!("Expected {} columns, indexing {}; {:?}", num_columns, j, col_names));
                Column::new(col_name.as_str().into(), col)
            }).collect();
            let mut df = DataFrame::new(series)?;
            let mut file = std::fs::File::create(path)?;
            CsvWriter::new(&mut file).include_header(true).finish(&mut df)?;
            Ok(df)
        }
    }

    /// Expected values loaded as strings to preserve precision info
    #[derive(Debug)]
    pub struct ExpectedStep {
        pub error: String,
        pub shape_vals: Vec<String>,
    }

    #[derive(Debug)]
    pub struct ExpectedHistory {
        pub col_names: Vec<String>,
        pub steps: Vec<ExpectedStep>,
    }

    impl ExpectedHistory {
        /// Load expected values from CSV, preserving string representations
        pub fn load(path: &str) -> Result<ExpectedHistory> {
            use std::io::{BufRead, BufReader};
            use std::fs::File;

            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            // Parse header
            let header = lines.next().ok_or_else(|| anyhow::anyhow!("Empty CSV"))??;
            let col_names: Vec<String> = header.split(',').map(|s| s.to_string()).collect();

            if col_names.first().map(|s| s.as_str()) != Some(ERROR_COL) {
                return Err(UnexpectedFirstCol(col_names.first().cloned().unwrap_or_default()).into());
            }

            // Parse rows
            let mut steps = Vec::new();
            for line in lines {
                let line = line?;
                let vals: Vec<&str> = line.split(',').collect();
                if vals.len() != col_names.len() {
                    return Err(anyhow::anyhow!(
                        "Row has {} columns, expected {}", vals.len(), col_names.len()
                    ));
                }
                steps.push(ExpectedStep {
                    error: vals[0].to_string(),
                    shape_vals: vals[1..].iter().map(|s| s.to_string()).collect(),
                });
            }

            Ok(ExpectedHistory { col_names, steps })
        }

        /// Compare actual HistoryStep against expected, using precision from expected strings
        pub fn check_step(&self, step_idx: usize, actual: &HistoryStep) -> Result<(), String> {
            let expected = &self.steps[step_idx];

            // Check error
            if !precision_eq(actual.error, &expected.error) {
                return Err(format!(
                    "Step {} error mismatch:\n  expected: {}\n  actual:   {} (rounded: {})",
                    step_idx,
                    expected.error,
                    actual.error,
                    round_to_string(actual.error, decimal_places(&expected.error))
                ));
            }

            // Check shape values
            let actual_vals = actual.vals();
            if actual_vals.len() != expected.shape_vals.len() {
                return Err(format!(
                    "Step {} value count mismatch: expected {}, got {}",
                    step_idx, expected.shape_vals.len(), actual_vals.len()
                ));
            }

            for (i, (actual_val, expected_str)) in actual_vals.iter().zip(expected.shape_vals.iter()).enumerate() {
                if !precision_eq(*actual_val, expected_str) {
                    let col_name = self.col_names.get(i + 1).map(|s| s.as_str()).unwrap_or("?");
                    return Err(format!(
                        "Step {} column {} ({}) mismatch:\n  expected: {}\n  actual:   {} (rounded: {})",
                        step_idx,
                        i,
                        col_name,
                        expected_str,
                        actual_val,
                        round_to_string(*actual_val, decimal_places(expected_str))
                    ));
                }
            }

            Ok(())
        }
    }
}
