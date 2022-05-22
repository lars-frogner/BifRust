//! Command line interface for generating of seed points in a volume of a field.

pub mod pdf;
pub mod random;
pub mod regular;
pub mod stratified;

use self::{
    pdf::{create_value_pdf_subcommand, create_volume_pdf_seeder_from_arguments},
    random::{create_random_subcommand, create_random_volume_seeder_from_arguments},
    regular::{create_regular_subcommand, create_regular_volume_seeder_from_arguments},
    stratified::{create_stratified_subcommand, create_stratified_volume_seeder_from_arguments},
};
use crate::{
    cli::utils as cli_utils,
    exit_on_false, exit_with_error,
    geometry::{
        Dim3::{X, Y, Z},
        Point3, Vec3,
    },
    grid::Grid3,
    interpolation::Interpolator3,
    io::snapshot::{fdt, SnapshotProvider3},
    seeding::volume::VolumeSeeder3,
    update_command_graph,
};
use clap::{Arg, ArgMatches, Command};

/// Creates a subcommand for using a volume seeder.
pub fn create_volume_seeder_subcommand(_parent_command_name: &'static str) -> Command<'static> {
    let command_name = "volume_seeder";

    update_command_graph!(_parent_command_name, command_name);

    Command::new(command_name)
        .about("Use a volume seeder")
        .subcommand_required(true)
        .arg(
            Arg::new("x-bounds")
                .long("x-bounds")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .allow_hyphen_values(true)
                .value_names(&["LOWER", "UPPER"])
                .help(
                    "Limits for the x-coordinates of the volume in which\n\
                     to generate seed points [default: full snapshot extent]",
                )
                .takes_value(true),
        )
        .arg(
            Arg::new("y-bounds")
                .long("y-bounds")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .allow_hyphen_values(true)
                .value_names(&["LOWER", "UPPER"])
                .help(
                    "Limits for the y-coordinates of the volume in which\n\
                     to generate seed points [default: full snapshot extent]",
                )
                .takes_value(true),
        )
        .arg(
            Arg::new("z-bounds")
                .long("z-bounds")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .allow_hyphen_values(true)
                .value_names(&["LOWER", "UPPER"])
                .help(
                    "Limits for the z-coordinates of the volume in which\n\
                     to generate seed points [default: full snapshot extent]",
                )
                .takes_value(true),
        )
        .subcommand(create_regular_subcommand(command_name))
        .subcommand(create_random_subcommand(command_name))
        .subcommand(create_stratified_subcommand(command_name))
        .subcommand(create_value_pdf_subcommand(command_name))
}

/// Creates a volume seeder based on the provided arguments.
pub fn create_volume_seeder_from_arguments<G, P, I>(
    arguments: &ArgMatches,
    provider: &mut P,
    interpolator: &I,
) -> VolumeSeeder3
where
    G: Grid3<fdt>,
    P: SnapshotProvider3<G>,
    I: Interpolator3,
{
    let original_grid = provider.grid();

    let original_lower_bounds = original_grid.lower_bounds();
    let original_upper_bounds = original_grid.upper_bounds();

    let x_bounds = cli_utils::get_values_from_parseable_argument_with_custom_defaults(
        arguments,
        "x-bounds",
        &|| vec![original_lower_bounds[X], original_upper_bounds[X]],
        Some(2),
    );
    exit_on_false!(
        x_bounds[1] > x_bounds[0],
        "Error: Upper bound for x must exceed lower bound"
    );
    let y_bounds = cli_utils::get_values_from_parseable_argument_with_custom_defaults(
        arguments,
        "y-bounds",
        &|| vec![original_lower_bounds[Y], original_upper_bounds[Y]],
        Some(2),
    );
    exit_on_false!(
        y_bounds[1] > y_bounds[0],
        "Error: Upper bound for y must exceed lower bound"
    );
    let z_bounds = cli_utils::get_values_from_parseable_argument_with_custom_defaults(
        arguments,
        "z-bounds",
        &|| vec![original_lower_bounds[Z], original_upper_bounds[Z]],
        Some(2),
    );
    exit_on_false!(
        z_bounds[1] > z_bounds[0],
        "Error: Upper bound for z must exceed lower bound"
    );
    let lower_bounds = Vec3::new(x_bounds[0], y_bounds[0], z_bounds[0]);
    let upper_bounds = Vec3::new(x_bounds[1], y_bounds[1], z_bounds[1]);

    let satisifes_constraints = |_: &Point3<fdt>| true;

    if let Some(seeder_arguments) = arguments.subcommand_matches("regular") {
        create_regular_volume_seeder_from_arguments(
            seeder_arguments,
            lower_bounds,
            upper_bounds,
            &satisifes_constraints,
        )
    } else if let Some(seeder_arguments) = arguments.subcommand_matches("random") {
        create_random_volume_seeder_from_arguments(
            seeder_arguments,
            lower_bounds,
            upper_bounds,
            &satisifes_constraints,
        )
    } else if let Some(seeder_arguments) = arguments.subcommand_matches("stratified") {
        create_stratified_volume_seeder_from_arguments(
            seeder_arguments,
            lower_bounds,
            upper_bounds,
            &satisifes_constraints,
        )
    } else if let Some(seeder_arguments) = arguments.subcommand_matches("value_pdf") {
        create_volume_pdf_seeder_from_arguments(
            seeder_arguments,
            lower_bounds,
            upper_bounds,
            provider,
            interpolator,
            &satisifes_constraints,
        )
    } else {
        exit_with_error!("Error: No seeder specified")
    }
}
