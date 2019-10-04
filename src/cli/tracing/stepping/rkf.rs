//! Command line interface for Runge-Kutta-Fehlberg steppers.

use crate::cli;
use crate::tracing::stepping::rkf::{RKFStepperConfig, RKFStepperType};
use clap::{App, Arg, ArgMatches, SubCommand};

/// Creates a subcommand for using a Runge-Kutta-Fehlberg stepper.
pub fn create_rkf_stepper_subcommand<'a, 'b>() -> App<'a, 'b> {
    let app = SubCommand::with_name("rkf_stepper").about("Use a Runge-Kutta-Fehlberg stepper");
    add_rkf_stepper_options_to_subcommand(app)
}

/// Adds arguments for parameters used by Runge-Kutta-Fehlberg steppers.
pub fn add_rkf_stepper_options_to_subcommand<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(
        Arg::with_name("dense-step-length")
            .long("dense-step-length")
            .value_name("VALUE")
            .long_help("Step length to use for dense (uniform) output positions [Mm]")
            .next_line_help(true)
            .takes_value(true)
            .default_value("0.01"),
    )
    .arg(
        Arg::with_name("max-step-attempts")
            .long("max-step-attempts")
            .value_name("NUMBER")
            .long_help("Maximum number of step attempts before terminating")
            .next_line_help(true)
            .takes_value(true)
            .default_value("16"),
    )
    .arg(
        Arg::with_name("stepping-absolute-tolerance")
            .long("stepping-absolute-tolerance")
            .value_name("VALUE")
            .long_help("Absolute error tolerance for stepping")
            .next_line_help(true)
            .takes_value(true)
            .default_value("1e-6"),
    )
    .arg(
        Arg::with_name("stepping-relative-tolerance")
            .long("stepping-relative-tolerance")
            .value_name("VALUE")
            .long_help("Relative error tolerance for stepping")
            .next_line_help(true)
            .takes_value(true)
            .default_value("1e-6"),
    )
    .arg(
        Arg::with_name("stepping-safety-factor")
            .long("stepping-safety-factor")
            .value_name("VALUE")
            .long_help("Scaling factor for the error to reduce step length oscillations")
            .next_line_help(true)
            .takes_value(true)
            .default_value("0.9"),
    )
    .arg(
        Arg::with_name("min-step-scale")
            .long("min-step-scale")
            .value_name("VALUE")
            .long_help("Smallest allowed scaling of the step size in one step")
            .next_line_help(true)
            .takes_value(true)
            .default_value("0.2"),
    )
    .arg(
        Arg::with_name("max-step-scale")
            .long("max-step-scale")
            .value_name("VALUE")
            .long_help("Largest allowed scaling of the step size in one step")
            .next_line_help(true)
            .takes_value(true)
            .default_value("10.0"),
    )
    .arg(
        Arg::with_name("stepping-initial-error")
            .long("stepping-initial-error")
            .value_name("VALUE")
            .long_help("Start value for stepping error")
            .next_line_help(true)
            .takes_value(true)
            .default_value("1e-4"),
    )
    .arg(
        Arg::with_name("initial-step-length")
            .long("stepping-initial-step-length")
            .value_name("VALUE")
            .long_help("Initial step size")
            .next_line_help(true)
            .takes_value(true)
            .default_value("1e-4"),
    )
    .arg(
        Arg::with_name("sudden-reversals-for-sink")
            .long("sudden-reversals-for-sink")
            .value_name("NUMBER")
            .long_help("Number of sudden direction reversals before the area is considered a sink")
            .next_line_help(true)
            .takes_value(true)
            .default_value("3"),
    )
    .arg(
        Arg::with_name("disable-pi-control")
            .long("disable-pi-control")
            .help("Disable Proportional Integral (PI) control used for stabilizing the stepping"),
    )
    .arg(
        Arg::with_name("stepping-scheme")
            .long("stepping-scheme")
            .value_name("NAME")
            .long_help("Which Runge-Kutta-Fehlberg stepping scheme to use")
            .next_line_help(true)
            .takes_value(true)
            .possible_values(&["rkf23", "rkf45"])
            .default_value("rkf45"),
    )
}

/// Determines Runge-Kutta-Fehlberg stepper parameters based on
/// provided options.
pub fn construct_rkf_stepper_config_from_options(
    arguments: &ArgMatches,
) -> (RKFStepperType, RKFStepperConfig) {
    let dense_step_length =
        cli::get_value_from_required_parseable_argument(arguments, "dense-step-length");
    let max_step_attempts =
        cli::get_value_from_required_parseable_argument(arguments, "max-step-attempts");
    let absolute_tolerance =
        cli::get_value_from_required_parseable_argument(arguments, "stepping-absolute-tolerance");
    let relative_tolerance =
        cli::get_value_from_required_parseable_argument(arguments, "stepping-relative-tolerance");
    let safety_factor =
        cli::get_value_from_required_parseable_argument(arguments, "stepping-safety-factor");
    let min_step_scale =
        cli::get_value_from_required_parseable_argument(arguments, "min-step-scale");
    let max_step_scale =
        cli::get_value_from_required_parseable_argument(arguments, "max-step-scale");
    let initial_error =
        cli::get_value_from_required_parseable_argument(arguments, "stepping-initial-error");
    let initial_step_length =
        cli::get_value_from_required_parseable_argument(arguments, "initial-step-length");
    let sudden_reversals_for_sink =
        cli::get_value_from_required_parseable_argument(arguments, "sudden-reversals-for-sink");
    let use_pi_control = !arguments.is_present("disable-pi-control");

    let stepper_type = cli::get_value_from_required_constrained_argument(
        arguments,
        "stepping-scheme",
        &["peaked", "isotropic"],
        &[RKFStepperType::RKF23, RKFStepperType::RKF45],
    );

    (
        stepper_type,
        RKFStepperConfig {
            dense_step_length,
            max_step_attempts,
            absolute_tolerance,
            relative_tolerance,
            safety_factor,
            min_step_scale,
            max_step_scale,
            initial_error,
            initial_step_length,
            sudden_reversals_for_sink,
            use_pi_control,
        },
    )
}
