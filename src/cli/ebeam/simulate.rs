//! Command line interface for simulating electron beams.

use crate::cli;
use crate::ebeam::accelerator::Accelerator;
use crate::ebeam::detection::simple::{
    SimpleReconnectionSiteDetector, SimpleReconnectionSiteDetectorConfig,
};
use crate::ebeam::detection::ReconnectionSiteDetector;
use crate::ebeam::distribution::power_law::acceleration::simple::{
    SimplePowerLawAccelerationConfig, SimplePowerLawAccelerator,
};
use crate::ebeam::distribution::power_law::PowerLawDistributionConfig;
use crate::ebeam::distribution::Distribution;
use crate::ebeam::{BeamPropertiesCollection, ElectronBeamSwarm};
use crate::grid::Grid3;
use crate::interpolation::poly_fit::{PolyFitInterpolator3, PolyFitInterpolatorConfig};
use crate::interpolation::Interpolator3;
use crate::io::snapshot::{fdt, SnapshotCacher3};
use crate::tracing::stepping::rkf::rkf23::RKF23StepperFactory3;
use crate::tracing::stepping::rkf::rkf45::RKF45StepperFactory3;
use crate::tracing::stepping::rkf::{RKFStepperConfig, RKFStepperType};
use crate::tracing::stepping::StepperFactory3;
use clap::{App, Arg, ArgMatches, SubCommand};
use rayon::prelude::*;

/// Builds a representation of the `ebeam-simulate` command line subcommand.
pub fn build_subcommand_simulate<'a, 'b>() -> App<'a, 'b> {
    let app = SubCommand::with_name("simulate")
        .about("Simulate electron beams in the snapshot")
        .after_help(
            "You can use subcommands to configure each stage. The subcommands must be specified\n\
             in the order detector -> distribution -> accelerator -> interpolator -> stepper,\n\
             with options for each stage directly following the subcommand. Any stage(s) can be\n\
             left unspecified, in which case the default implementation and parameters are used\n\
             for that stage.",
        )
        .arg(
            Arg::with_name("OUTPUT_PATH")
                .help("Path where the beam data should be saved")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output-format")
                .short("f")
                .long("output-format")
                .value_name("FORMAT")
                .long_help("Format to use for saving beam data")
                .next_line_help(true)
                .takes_value(true)
                .possible_values(&["pickle", "json"])
                .default_value("pickle"),
        )
        .arg(
            Arg::with_name("generate-only")
                .short("g")
                .long("generate-only")
                .help("Do not propagate the generated beams"),
        )
        .arg(
            Arg::with_name("extra-fixed-scalars")
                .long("extra-fixed-scalars")
                .value_name("NAMES")
                .long_help("List of scalar fields to extract at acceleration sites")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("extra-varying-scalars")
                .long("extra-varying-scalars")
                .value_name("NAMES")
                .long_help("List of scalar fields to extract along beam trajectories")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Print status messages while simulating electron beams"),
        );

    let simple_reconnection_site_detector_subcommand =
        super::detection::simple::create_simple_reconnection_site_detector_subcommand();
    let power_law_distribution_subcommand =
        super::distribution::power_law::create_power_law_distribution_subcommand();
    let simple_power_law_accelerator_subcommand =
        super::accelerator::simple_power_law::create_simple_power_law_accelerator_subcommand();
    let poly_fit_interpolator_subcommand =
        cli::interpolation::poly_fit::create_poly_fit_interpolator_subcommand();
    let rkf_stepper_subcommand = cli::tracing::stepping::rkf::create_rkf_stepper_subcommand();

    let poly_fit_interpolator_subcommand =
        poly_fit_interpolator_subcommand.subcommand(rkf_stepper_subcommand.clone());
    let simple_power_law_accelerator_subcommand = simple_power_law_accelerator_subcommand
        .subcommand(poly_fit_interpolator_subcommand.clone())
        .subcommand(rkf_stepper_subcommand.clone());
    let power_law_distribution_subcommand = power_law_distribution_subcommand
        .subcommand(simple_power_law_accelerator_subcommand.clone())
        .subcommand(poly_fit_interpolator_subcommand.clone())
        .subcommand(rkf_stepper_subcommand.clone());
    let simple_reconnection_site_detector_subcommand = simple_reconnection_site_detector_subcommand
        .subcommand(power_law_distribution_subcommand.clone())
        .subcommand(simple_power_law_accelerator_subcommand.clone())
        .subcommand(poly_fit_interpolator_subcommand.clone())
        .subcommand(rkf_stepper_subcommand.clone());

    app.subcommand(simple_reconnection_site_detector_subcommand)
        .subcommand(power_law_distribution_subcommand)
        .subcommand(simple_power_law_accelerator_subcommand)
        .subcommand(poly_fit_interpolator_subcommand)
        .subcommand(rkf_stepper_subcommand)
}

/// Runs the actions for the `ebeam-simulate` subcommand using the given arguments.
pub fn run_subcommand_simulate<G: Grid3<fdt>>(
    arguments: &ArgMatches,
    snapshot: &mut SnapshotCacher3<G>,
) {
    run_with_selected_detector(arguments, snapshot);
}

fn run_with_selected_detector<G>(arguments: &ArgMatches, snapshot: &mut SnapshotCacher3<G>)
where
    G: Grid3<fdt>,
{
    let (detector_config, detector_arguments) = if let Some(detector_arguments) =
        arguments.subcommand_matches("simple_detector")
    {
        println!("Using specified simple_detector");
        (super::detection::simple::construct_simple_reconnection_site_detector_config_from_options(
            snapshot.reader(),
            detector_arguments,
        ), detector_arguments)
    } else {
        println!("Using default simple_detector");
        (
            SimpleReconnectionSiteDetectorConfig::with_defaults_from_param_file(snapshot.reader()),
            arguments,
        )
    };
    let detector = SimpleReconnectionSiteDetector::new(detector_config);

    run_with_selected_accelerator(arguments, detector_arguments, snapshot, detector);
}

fn run_with_selected_accelerator<G, D>(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    snapshot: &mut SnapshotCacher3<G>,
    detector: D,
) where
    G: Grid3<fdt>,
    D: ReconnectionSiteDetector,
{
    let (distribution_config, distribution_arguments) = if let Some(distribution_arguments) =
        arguments.subcommand_matches("power_law_distribution")
    {
        println!("Using specified power_law_distribution");
        (
            super::distribution::power_law::construct_power_law_distribution_config_from_options(
                snapshot.reader(),
                distribution_arguments,
            ),
            distribution_arguments,
        )
    } else {
        println!("Using default power_law_distribution");
        (
            PowerLawDistributionConfig::with_defaults_from_param_file(snapshot.reader()),
            arguments,
        )
    };

    let (accelerator_config, accelerator_arguments) = if let Some(accelerator_arguments) =
        distribution_arguments.subcommand_matches("simple_power_law_accelerator")
    {
        println!("Using specified simple_power_law_accelerator");
        (super::accelerator::simple_power_law::construct_simple_power_law_accelerator_config_from_options(snapshot.reader(), accelerator_arguments), accelerator_arguments)
    } else {
        println!("Using default simple_power_law_accelerator");
        (
            SimplePowerLawAccelerationConfig::with_defaults_from_param_file(snapshot.reader()),
            distribution_arguments,
        )
    };

    let accelerator = SimplePowerLawAccelerator::new(distribution_config, accelerator_config);

    run_with_selected_interpolator(
        root_arguments,
        accelerator_arguments,
        snapshot,
        detector,
        accelerator,
    );
}

fn run_with_selected_interpolator<G, D, A>(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    snapshot: &mut SnapshotCacher3<G>,
    detector: D,
    accelerator: A)
where G: Grid3<fdt>,
      D: ReconnectionSiteDetector,
      A: Accelerator + Sync + Send,
      <A::DistributionType as Distribution>::PropertiesCollectionType: ParallelExtend<<<A::DistributionType as Distribution>::PropertiesCollectionType as BeamPropertiesCollection>::Item>,
      A::DistributionType: Send,
{
    let (interpolator_config, interpolator_arguments) = if let Some(interpolator_arguments) =
        arguments.subcommand_matches("poly_fit_interpolator")
    {
        println!("Using specified poly_fit_interpolator");
        (
            cli::interpolation::poly_fit::construct_poly_fit_interpolator_config_from_options(
                interpolator_arguments,
            ),
            interpolator_arguments,
        )
    } else {
        println!("Using default poly_fit_interpolator");
        (PolyFitInterpolatorConfig::default(), arguments)
    };
    let interpolator = PolyFitInterpolator3::new(interpolator_config);

    if root_arguments.is_present("generate-only") {
        run_generation(
            root_arguments,
            snapshot,
            detector,
            accelerator,
            interpolator,
        );
    } else {
        run_with_selected_stepper_factory(
            root_arguments,
            interpolator_arguments,
            snapshot,
            detector,
            accelerator,
            interpolator,
        );
    }
}

fn run_with_selected_stepper_factory<G, D, A, I>(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    snapshot: &mut SnapshotCacher3<G>,
    detector: D,
    accelerator: A,
    interpolator: I)
where G: Grid3<fdt>,
      D: ReconnectionSiteDetector,
      A: Accelerator + Sync + Send,
      A::DistributionType: Send,
      <A::DistributionType as Distribution>::PropertiesCollectionType: ParallelExtend<<<A::DistributionType as Distribution>::PropertiesCollectionType as BeamPropertiesCollection>::Item>,
      I: Interpolator3
{
    let (stepper_type, stepper_config) = if let Some(stepper_arguments) =
        arguments.subcommand_matches("rkf_stepper")
    {
        println!("Using specified rkf_stepper");
        cli::tracing::stepping::rkf::construct_rkf_stepper_config_from_options(stepper_arguments)
    } else {
        println!("Using default rkf_stepper");
        (RKFStepperType::RKF45, RKFStepperConfig::default())
    };

    match stepper_type {
        RKFStepperType::RKF23 => {
            run_propagation(
                root_arguments,
                snapshot,
                detector,
                accelerator,
                interpolator,
                RKF23StepperFactory3::new(stepper_config),
            );
        }
        RKFStepperType::RKF45 => {
            run_propagation(
                root_arguments,
                snapshot,
                detector,
                accelerator,
                interpolator,
                RKF45StepperFactory3::new(stepper_config),
            );
        }
    }
}

fn run_generation<G, D, A, I>(root_arguments: &ArgMatches, snapshot: &mut SnapshotCacher3<G>, detector: D, accelerator: A, interpolator: I)
where G: Grid3<fdt>,
      D: ReconnectionSiteDetector,
      A: Accelerator + Sync,
      A::DistributionType: Send,
      <A::DistributionType as Distribution>::PropertiesCollectionType: ParallelExtend<<<A::DistributionType as Distribution>::PropertiesCollectionType as BeamPropertiesCollection>::Item>,
      I: Interpolator3
{
    let beams = ElectronBeamSwarm::generate_unpropagated(
        snapshot,
        detector,
        accelerator,
        &interpolator,
        root_arguments.is_present("verbose").into(),
    );
    snapshot.drop_all_fields();
    perform_post_simulation_actions(root_arguments, snapshot, interpolator, beams);
}

fn run_propagation<G, D, A, I, StF>(root_arguments: &ArgMatches, snapshot: &mut SnapshotCacher3<G>, detector: D, accelerator: A, interpolator: I, stepper_factory: StF)
where G: Grid3<fdt>,
      D: ReconnectionSiteDetector,
      A: Accelerator + Sync + Send,
      A::DistributionType: Send,
      <A::DistributionType as Distribution>::PropertiesCollectionType: ParallelExtend<<<A::DistributionType as Distribution>::PropertiesCollectionType as BeamPropertiesCollection>::Item>,
      I: Interpolator3,
      StF: StepperFactory3 + Sync
{
    let beams = ElectronBeamSwarm::generate_propagated(
        snapshot,
        detector,
        accelerator,
        &interpolator,
        stepper_factory,
        root_arguments.is_present("verbose").into(),
    );
    snapshot.drop_all_fields();
    perform_post_simulation_actions(root_arguments, snapshot, interpolator, beams);
}

fn perform_post_simulation_actions<G, A, I>(
    root_arguments: &ArgMatches,
    snapshot: &mut SnapshotCacher3<G>,
    interpolator: I,
    mut beams: ElectronBeamSwarm<A>,
) where
    G: Grid3<fdt>,
    A: Accelerator,
    I: Interpolator3,
{
    let output_path = root_arguments
        .value_of("output-path")
        .expect("No value for required argument.");

    if let Some(extra_fixed_scalars) = root_arguments
        .values_of("extra-fixed-scalars")
        .map(|values| values.collect::<Vec<_>>())
    {
        for name in extra_fixed_scalars {
            beams.extract_fixed_scalars(
                snapshot
                    .obtain_scalar_field(name)
                    .unwrap_or_else(|err| panic!("Could not read {} from snapshot: {}", name, err)),
                &interpolator,
            );
            snapshot.drop_scalar_field(name);
        }
    }
    if let Some(extra_varying_scalars) = root_arguments
        .values_of("extra-varying-scalars")
        .map(|values| values.collect::<Vec<_>>())
    {
        for name in extra_varying_scalars {
            beams.extract_varying_scalars(
                snapshot
                    .obtain_scalar_field(name)
                    .unwrap_or_else(|err| panic!("Could not read {} from snapshot: {}", name, err)),
                &interpolator,
            );
            snapshot.drop_scalar_field(name);
        }
    }

    match root_arguments
        .value_of("output-format")
        .expect("No value for argument with default.")
    {
        "pickle" => {
            beams
                .save_as_combined_pickles(output_path)
                .unwrap_or_else(|err| panic!("Could not save output data: {}", err));
        }
        "json" => {
            beams
                .save_as_json(output_path)
                .unwrap_or_else(|err| panic!("Could not save output data: {}", err));
        }
        invalid => panic!("Invalid output format {}.", invalid),
    }
}
