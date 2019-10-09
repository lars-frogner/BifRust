//! Command line interface for creating Bifrost mesh files.

mod regular;

use crate::grid::Grid3;
use crate::io::mesh;
use crate::io::snapshot::fdt;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

/// Creates a subcommand for generating a Bifrost mesh file.
pub fn create_create_mesh_subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("create_mesh")
        .about("Create a Bifrost mesh file")
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("OUTPUT_PATH")
                .help("Path where the resample field should be saved in pickle format")
                .required(true)
                .takes_value(true),
        )
        .subcommand(regular::create_regular_mesh_subcommand())
}

/// Runs the actions for the `create_mesh` subcommand using the given arguments.
pub fn run_create_mesh_subcommand(arguments: &ArgMatches) {
    if let Some(regular_arguments) = arguments.subcommand_matches("regular") {
        regular::run_regular_subcommand(arguments, regular_arguments);
    } else {
        panic!("No resampling mode specified.")
    };
}

fn write_mesh_file<G: Grid3<fdt>>(root_arguments: &ArgMatches, grid: G) {
    let output_path = root_arguments
        .value_of("OUTPUT_PATH")
        .expect("No value for required argument.");

    mesh::write_mesh_file_from_grid(&grid, output_path)
        .unwrap_or_else(|err| panic!("Could not write mesh file: {}", err));
}