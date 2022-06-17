//! Detection of reconnection sites by reading positions from an input file.

use super::ReconnectionSiteDetector;
use crate::{
    field::CachingScalarFieldProvider3,
    geometry::Idx3,
    io::{snapshot::fdt, Verbosity},
    seeding::{manual::ManualSeeder3, Seeder3},
};
use std::io;
use std::path::Path;

/// Detector reading the reconnection site positions from an input file.
pub struct ManualReconnectionSiteDetector {
    seeder: ManualSeeder3,
}

impl ManualReconnectionSiteDetector {
    /// Creates a new manual reconnection site detector reading positions from the given file.
    ///
    /// The input file is assumed to be in CSV format, with each line consisting
    /// of the three comma-separated coordinates of a single position.
    pub fn new(input_file_path: &Path) -> io::Result<Self> {
        Ok(Self {
            seeder: ManualSeeder3::new(input_file_path)?,
        })
    }
}

impl ReconnectionSiteDetector for ManualReconnectionSiteDetector {
    type Seeder = Vec<Idx3<usize>>;

    fn detect_reconnection_sites<P>(&self, snapshot: &mut P, _verbosity: &Verbosity) -> Self::Seeder
    where
        P: CachingScalarFieldProvider3<fdt>,
    {
        self.seeder.to_index_seeder(snapshot.grid())
    }
}
