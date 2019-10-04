//! Command line interface for interpolation by polynomial fitting.

use crate::cli;
use crate::interpolation::poly_fit::PolyFitInterpolatorConfig;
use clap::{App, Arg, ArgMatches, SubCommand};

/// Creates a subcommand for using the polynomial fitting interpolator.
pub fn create_poly_fit_interpolator_subcommand<'a, 'b>() -> App<'a, 'b> {
    let app = SubCommand::with_name("poly_fit_interpolator")
        .about("Use the polynomial fitting interpolator");
    add_poly_fit_interpolator_options_to_subcommand(app)
}

/// Adds arguments for parameters used by the polynomial fitting interpolator.
pub fn add_poly_fit_interpolator_options_to_subcommand<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(
        Arg::with_name("interpolation-order")
            .long("interpolation-order")
            .value_name("ORDER")
            .long_help("Order of the polynomials to fit when interpolating field values\n")
            .next_line_help(true)
            .takes_value(true)
            .possible_values(&["1", "2", "3", "4", "5"])
            .default_value("3"),
    )
    .arg(
        Arg::with_name("variation-threshold-for-linear-interpolation")
            .long("variation-threshold-for-linear-interpolation")
            .value_name("VALUE")
            .long_help(
                "Linear interpolation is used when a normalized variance of the values\n\
                 surrounding the interpolation point exceeds this",
            )
            .next_line_help(true)
            .takes_value(true)
            .default_value("0.3"),
    )
}

/// Determines polynomial fitting interpolator parameters based on
/// provided options.
pub fn construct_poly_fit_interpolator_config_from_options(
    arguments: &ArgMatches,
) -> PolyFitInterpolatorConfig {
    let order = cli::get_value_from_required_parseable_argument(arguments, "interpolation-order");
    let variation_threshold_for_linear = cli::get_value_from_required_parseable_argument(
        arguments,
        "variation-threshold-for-linear-interpolation",
    );
    PolyFitInterpolatorConfig {
        order,
        variation_threshold_for_linear,
    }
}
