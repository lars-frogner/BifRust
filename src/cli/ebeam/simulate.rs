//! Command line interface for simulating electron beams.

use super::{
    accelerator::simple_power_law::{
        construct_simple_power_law_accelerator_config_from_options,
        create_simple_power_law_accelerator_subcommand,
    },
    detection::{
        manual::{
            construct_manual_reconnection_site_detector_from_options,
            create_manual_reconnection_site_detector_subcommand,
        },
        simple::{
            construct_simple_reconnection_site_detector_config_from_options,
            create_simple_reconnection_site_detector_subcommand,
        },
    },
    distribution::power_law::create_power_law_distribution_subcommand,
    propagator::analytical::{
        construct_analytical_propagator_config_from_options,
        create_analytical_propagator_subcommand,
    },
};
use crate::{
    add_subcommand_combinations,
    cli::{
        interpolation::poly_fit::{
            construct_poly_fit_interpolator_config_from_options,
            create_poly_fit_interpolator_subcommand,
        },
        tracing::stepping::rkf::{
            construct_rkf_stepper_config_from_options, create_rkf_stepper_subcommand,
        },
        utils as cli_utils,
    },
    ebeam::{
        accelerator::Accelerator,
        detection::{
            simple::{SimpleReconnectionSiteDetector, SimpleReconnectionSiteDetectorConfig},
            DynReconnectionSiteDetector,
        },
        distribution::{
            power_law::acceleration::simple::{
                SimplePowerLawAccelerationConfig, SimplePowerLawAccelerator,
            },
            Distribution,
        },
        propagation::{
            analytical::{AnalyticalPropagator, AnalyticalPropagatorConfig},
            Propagator,
        },
        BeamPropertiesCollection, ElectronBeamSwarm,
    },
    exit_on_error, exit_with_error,
    field::{DynCachingScalarFieldProvider3, DynScalarFieldProvider3, ScalarFieldCacher3},
    interpolation::{
        poly_fit::{PolyFitInterpolator3, PolyFitInterpolatorConfig},
        InterpGridVerifier3, Interpolator3,
    },
    io::{
        snapshot::{self, fdt, SnapshotMetadata},
        utils::{AtomicOutputFile, IOContext},
    },
    tracing::stepping::rkf::{
        rkf23::RKF23Stepper3, rkf45::RKF45Stepper3, RKFStepperConfig, RKFStepperType,
    },
    update_command_graph,
};
use clap::{Arg, ArgMatches, Command};
use rayon::prelude::*;
use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Builds a representation of the `ebeam-simulate` command line subcommand.
pub fn create_simulate_subcommand(_parent_command_name: &'static str) -> Command<'static> {
    let command_name = "simulate";

    update_command_graph!(_parent_command_name, command_name);

    let command = Command::new(command_name)
        .about("Simulate electron beams in the snapshot")
        .long_about(
            "Simulate electron beams in the snapshot.\n\
             Each beam originates at a reconnection site, where a non-thermal electron\n\
             distribution is generated by an acceleration mechanism. The distribution\n\
             propagates along the magnetic field and deposits its energy through interactions\n\
             with the surrounding plasma.",
        )
        .after_help(
            "You can use subcommands to configure each action. The subcommands must be specified\n\
             in the order detector -> distribution -> accelerator -> interpolator -> stepper,\n\
             with options for each action directly following the subcommand. Any action(s) can be\n\
             left unspecified, in which case the default implementation and parameters are used\n\
             for that action.",
        )
        .arg(
            Arg::new("output-file")
                .value_name("OUTPUT_FILE")
                .help(
                    "Path of the file where the beam data should be saved\n\
                       Writes in the following format based on the file extension:\
                       \n    *.fl: Creates a binary file readable by the backstaff Python package\
                       \n    *.pickle: Creates a Python pickle file (requires the pickle feature)\
                       \n    *.json: Creates a JSON file (requires the json feature)\
                       \n    *.h5part: Creates a H5Part file (requires the hdf5 feature)",
                )
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("overwrite")
                .long("overwrite")
                .help("Automatically overwrite any existing files (unless listed as protected)")
                .conflicts_with("no-overwrite"),
        )
        .arg(
            Arg::new("no-overwrite")
                .long("no-overwrite")
                .help("Do not overwrite any existing files")
                .conflicts_with("overwrite"),
        )
        .arg(
            Arg::new("generate-only")
                .short('g')
                .long("generate-only")
                .help("Do not propagate the generated beams"),
        )
        .arg(
            Arg::new("extra-fixed-scalars")
                .long("extra-fixed-scalars")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .value_name("NAMES")
                .help(
                    "List of scalar fields to extract at acceleration sites\n \
                     (comma-separated)",
                )
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("extra-fixed-vectors")
                .long("extra-fixed-vectors")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .value_name("NAMES")
                .help(
                    "List of vector fields to extract at acceleration sites\n \
                     (comma-separated)",
                )
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("extra-varying-scalars")
                .long("extra-varying-scalars")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .value_name("NAMES")
                .help(
                    "List of scalar fields to extract along beam trajectories\n \
                     (comma-separated)",
                )
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("extra-varying-vectors")
                .long("extra-varying-vectors")
                .require_equals(true)
                .use_value_delimiter(true)
                .require_value_delimiter(true)
                .value_name("NAMES")
                .help(
                    "List of vector fields to extract along beam trajectories\n \
                     (comma-separated)",
                )
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(Arg::new("drop-h5part-id").long("drop-h5part-id").help(
            "Reduce H5Part file size by excluding particle IDs required by some tools\n\
                     (e.g. VisIt)",
        ))
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Print status messages while simulating electron beams"),
        )
        .arg(
            Arg::new("progress")
                .short('p')
                .long("progress")
                .help("Show progress bar for simulation (also implies `verbose`)"),
        )
        .arg(
            Arg::new("print-parameter-values")
                .long("print-parameter-values")
                .help("Prints the values of all the parameters that will be used")
                .hide(true),
        )
        .subcommand(create_simple_reconnection_site_detector_subcommand(
            command_name,
        ))
        .subcommand(create_manual_reconnection_site_detector_subcommand(
            command_name,
        ))
        .subcommand(create_power_law_distribution_subcommand(command_name))
        .subcommand(create_simple_power_law_accelerator_subcommand(command_name))
        .subcommand(create_analytical_propagator_subcommand(command_name));

    add_subcommand_combinations!(command, command_name, false; poly_fit_interpolator, rkf_stepper)
}

/// Runs the actions for the `ebeam-simulate` subcommand using the given arguments.
pub fn run_simulate_subcommand(
    arguments: &ArgMatches,
    metadata: &dyn SnapshotMetadata,
    provider: DynScalarFieldProvider3<fdt>,
    io_context: &mut IOContext,
) {
    let verbosity = cli_utils::parse_verbosity(arguments, false);
    let snapshot = Box::new(ScalarFieldCacher3::new_manual_cacher(provider, verbosity));
    run_with_selected_detector(arguments, metadata, snapshot, io_context);
}

#[derive(Copy, Clone, Debug)]
enum OutputType {
    Fl,
    #[cfg(feature = "pickle")]
    Pickle,
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "hdf5")]
    H5Part,
}

impl OutputType {
    fn from_path(file_path: &Path) -> Self {
        Self::from_extension(
            file_path
                .extension()
                .unwrap_or_else(|| {
                    exit_with_error!(
                        "Error: Missing extension for output file\n\
                         Valid extensions are: {}",
                        Self::valid_extensions_string()
                    )
                })
                .to_string_lossy()
                .as_ref(),
        )
    }

    fn from_extension(extension: &str) -> Self {
        match extension {
            "fl" => Self::Fl,
            "pickle" => {
                #[cfg(feature = "pickle")]
                {
                    Self::Pickle
                }
                #[cfg(not(feature = "pickle"))]
                exit_with_error!(
                    "Error: Compile with pickle feature in order to write Pickle files\n\
                     Tip: Use cargo flag --features=pickle"
                );
            }
            "json" => {
                #[cfg(feature = "json")]
                {
                    Self::Json
                }
                #[cfg(not(feature = "json"))]
                exit_with_error!(
                    "Error: Compile with json feature in order to write JSON files\n\
                     Tip: Use cargo flag --features=json"
                );
            }
            "h5part" => {
                #[cfg(feature = "hdf5")]
                {
                    Self::H5Part
                }
                #[cfg(not(feature = "hdf5"))]
                exit_with_error!("Error: Compile with hdf5 feature in order to write H5Part files\n\
                                  Tip: Use cargo flag --features=hdf5 and make sure the HDF5 library is available");
            }
            invalid => exit_with_error!(
                "Error: Invalid extension {} for output file\n\
                 Valid extensions are: {}",
                invalid,
                Self::valid_extensions_string()
            ),
        }
    }

    fn valid_extensions_string() -> String {
        format!(
            "fl, pickle, json{}",
            if cfg!(feature = "hdf5") {
                ", h5part"
            } else {
                ""
            }
        )
    }
}

impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Fl => "fl",
                #[cfg(feature = "pickle")]
                Self::Pickle => "pickle",
                #[cfg(feature = "json")]
                Self::Json => "json",
                #[cfg(feature = "hdf5")]
                Self::H5Part => "h5part",
            }
        )
    }
}

fn run_with_selected_detector(
    root_arguments: &ArgMatches,
    metadata: &dyn SnapshotMetadata,
    snapshot: DynCachingScalarFieldProvider3<fdt>,
    io_context: &mut IOContext,
) {
    let (detector, detector_arguments) =
        if let Some(detector_arguments) = root_arguments.subcommand_matches("manual_detector") {
            (
                Box::new(construct_manual_reconnection_site_detector_from_options(
                    detector_arguments,
                )) as DynReconnectionSiteDetector,
                detector_arguments,
            )
        } else {
            let (detector_config, detector_arguments) = if let Some(detector_arguments) =
                root_arguments.subcommand_matches("simple_detector")
            {
                (
                    construct_simple_reconnection_site_detector_config_from_options(
                        detector_arguments,
                        metadata.parameters(),
                    ),
                    detector_arguments,
                )
            } else {
                (
                    SimpleReconnectionSiteDetectorConfig::with_defaults_from_param_file(
                        metadata.parameters(),
                    ),
                    root_arguments,
                )
            };

            if root_arguments.is_present("print-parameter-values") {
                println!("{:#?}", detector_config);
            }

            (
                Box::new(SimpleReconnectionSiteDetector::new(detector_config))
                    as DynReconnectionSiteDetector,
                detector_arguments,
            )
        };
    run_with_selected_accelerator(
        root_arguments,
        detector_arguments,
        metadata,
        snapshot,
        detector,
        io_context,
    );
}

fn run_with_selected_accelerator(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    metadata: &dyn SnapshotMetadata,
    snapshot: DynCachingScalarFieldProvider3<fdt>,
    detector: DynReconnectionSiteDetector,
    io_context: &mut IOContext,
) {
    let distribution_arguments = arguments
        .subcommand_matches("power_law_distribution")
        .unwrap_or(arguments);

    if let Some(accelerator_arguments) =
        distribution_arguments.subcommand_matches("simple_power_law_accelerator")
    {
        let accelerator_config = construct_simple_power_law_accelerator_config_from_options(
            accelerator_arguments,
            metadata.parameters(),
        );
        if root_arguments.is_present("print-parameter-values") {
            println!("{:#?}", accelerator_config);
        }
        let accelerator = SimplePowerLawAccelerator::new(accelerator_config);
        run_with_simple_accelerator_and_selected_propagator(
            root_arguments,
            accelerator_arguments,
            metadata,
            snapshot,
            detector,
            accelerator,
            io_context,
        );
    } else {
        let accelerator_config =
            SimplePowerLawAccelerationConfig::with_defaults_from_param_file(metadata.parameters());
        if root_arguments.is_present("print-parameter-values") {
            println!("{:#?}", accelerator_config);
        }
        let accelerator = SimplePowerLawAccelerator::new(accelerator_config);
        run_with_simple_accelerator_and_selected_propagator(
            root_arguments,
            distribution_arguments,
            metadata,
            snapshot,
            detector,
            accelerator,
            io_context,
        );
    };
}

fn run_with_simple_accelerator_and_selected_propagator(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    metadata: &dyn SnapshotMetadata,
    snapshot: DynCachingScalarFieldProvider3<fdt>,
    detector: DynReconnectionSiteDetector,
    accelerator: SimplePowerLawAccelerator,
    io_context: &mut IOContext,
) {
    if let Some(propagator_arguments) = arguments.subcommand_matches("analytical_propagator") {
        let propagator_config = construct_analytical_propagator_config_from_options(
            propagator_arguments,
            metadata.parameters(),
        );
        if root_arguments.is_present("print-parameter-values") {
            println!("{:#?}", propagator_config);
        }
        run_with_selected_interpolator::<_, AnalyticalPropagator>(
            root_arguments,
            propagator_arguments,
            snapshot,
            detector,
            accelerator,
            propagator_config,
            io_context,
        );
    } else {
        let propagator_config =
            AnalyticalPropagatorConfig::with_defaults_from_param_file(metadata.parameters());
        if root_arguments.is_present("print-parameter-values") {
            println!("{:#?}", propagator_config);
        }
        run_with_selected_interpolator::<_, AnalyticalPropagator>(
            root_arguments,
            arguments,
            snapshot,
            detector,
            accelerator,
            propagator_config,
            io_context,
        );
    }
}

fn run_with_selected_interpolator<A, P>(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    snapshot: DynCachingScalarFieldProvider3<fdt>,
    detector: DynReconnectionSiteDetector,
    accelerator: A,
    propagator_config: P::Config,
    io_context: &mut IOContext)
where A: Accelerator + Sync + Send,
      P: Propagator<<A as Accelerator>::DistributionType>,
      <A::DistributionType as Distribution>::PropertiesCollectionType: ParallelExtend<<<A::DistributionType as Distribution>::PropertiesCollectionType as BeamPropertiesCollection>::Item>,
      A::DistributionType: Send,
    {
    let (interpolator_config, interpolator_arguments) = if let Some(interpolator_arguments) =
        arguments.subcommand_matches("poly_fit_interpolator")
    {
        (
            construct_poly_fit_interpolator_config_from_options(interpolator_arguments),
            interpolator_arguments,
        )
    } else {
        (PolyFitInterpolatorConfig::default(), arguments)
    };

    if root_arguments.is_present("print-parameter-values") {
        println!("{:#?}", interpolator_config);
    }

    let interpolator = Box::new(PolyFitInterpolator3::new(interpolator_config));

    exit_on_error!(
        interpolator.verify_grid(snapshot.grid()),
        "Invalid input grid for simulating electron beams: {}"
    );

    run_with_selected_stepper::<A, P>(
        root_arguments,
        interpolator_arguments,
        snapshot,
        detector,
        accelerator,
        propagator_config,
        interpolator.as_ref(),
        io_context,
    );
}

fn run_with_selected_stepper<A, P>(
    root_arguments: &ArgMatches,
    arguments: &ArgMatches,
    mut snapshot: DynCachingScalarFieldProvider3<fdt>,
    detector: DynReconnectionSiteDetector,
    accelerator: A,
    propagator_config: P::Config,
    interpolator: &dyn Interpolator3<fdt>,
    io_context: &mut IOContext)
where A: Accelerator + Sync + Send,
      P: Propagator<<A as Accelerator>::DistributionType>,
      A::DistributionType: Send,
      <A::DistributionType as Distribution>::PropertiesCollectionType: ParallelExtend<<<A::DistributionType as Distribution>::PropertiesCollectionType as BeamPropertiesCollection>::Item>,
{
    let (stepper_type, stepper_config) =
        if let Some(stepper_arguments) = arguments.subcommand_matches("rkf_stepper") {
            construct_rkf_stepper_config_from_options(stepper_arguments)
        } else {
            (RKFStepperType::RKF45, RKFStepperConfig::default())
        };

    if root_arguments.is_present("print-parameter-values") {
        println!("{:#?}\nstepper_type: {:?}", stepper_config, stepper_type);
    }
    let mut output_file_path = exit_on_error!(
        PathBuf::from_str(
            root_arguments
                .value_of("output-file")
                .expect("No value for required argument"),
        ),
        "Error: Could not interpret path to output file: {}"
    );

    let output_type = OutputType::from_path(&output_file_path);

    if let Some(snap_num_in_range) = io_context.get_snap_num_in_range() {
        output_file_path.set_file_name(snapshot::create_new_snapshot_file_name_from_path(
            &output_file_path,
            snap_num_in_range.offset(),
            &output_type.to_string(),
            true,
        ));
    }

    let overwrite_mode = cli_utils::overwrite_mode_from_arguments(arguments);
    let verbosity = cli_utils::parse_verbosity(root_arguments, true);

    io_context.set_overwrite_mode(overwrite_mode);

    let atomic_output_file = exit_on_error!(
        io_context.create_atomic_output_file(output_file_path),
        "Error: Could not create temporary output file: {}"
    );

    if !atomic_output_file.check_if_write_allowed(io_context, &verbosity) {
        return;
    }

    let extra_atomic_output_file = match output_type {
        #[cfg(feature = "hdf5")]
        OutputType::H5Part => {
            let extra_atomic_output_file = exit_on_error!(
                io_context.create_atomic_output_file(
                    atomic_output_file
                        .target_path()
                        .with_extension("sites.h5part")
                ),
                "Error: Could not create temporary output file: {}"
            );
            if !extra_atomic_output_file.check_if_write_allowed(io_context, &verbosity) {
                return;
            }
            Some(extra_atomic_output_file)
        }
        _ => None,
    };

    let beams = match stepper_type {
        RKFStepperType::RKF23 => {
            let stepper = Box::new(RKF23Stepper3::new(stepper_config));
            if root_arguments.is_present("generate-only") {
                ElectronBeamSwarm::generate_unpropagated::<P>(
                    &mut *snapshot,
                    &*detector,
                    accelerator,
                    propagator_config,
                    interpolator,
                    stepper,
                    verbosity,
                )
            } else {
                ElectronBeamSwarm::generate_propagated::<P>(
                    &mut *snapshot,
                    &*detector,
                    accelerator,
                    propagator_config,
                    interpolator,
                    stepper,
                    verbosity,
                )
            }
        }
        RKFStepperType::RKF45 => {
            let stepper = Box::new(RKF45Stepper3::new(stepper_config));
            if root_arguments.is_present("generate-only") {
                ElectronBeamSwarm::generate_unpropagated::<P>(
                    &mut *snapshot,
                    &*detector,
                    accelerator,
                    propagator_config,
                    interpolator,
                    stepper,
                    verbosity,
                )
            } else {
                ElectronBeamSwarm::generate_propagated::<P>(
                    &mut *snapshot,
                    &*detector,
                    accelerator,
                    propagator_config,
                    interpolator,
                    stepper,
                    verbosity,
                )
            }
        }
    };
    perform_post_simulation_actions(
        root_arguments,
        output_type,
        atomic_output_file,
        extra_atomic_output_file,
        io_context,
        snapshot,
        interpolator,
        beams,
    );
}

fn perform_post_simulation_actions<A>(
    root_arguments: &ArgMatches,
    output_type: OutputType,
    atomic_output_file: AtomicOutputFile,
    extra_atomic_output_file: Option<AtomicOutputFile>,
    io_context: &IOContext,
    mut snapshot: DynCachingScalarFieldProvider3<fdt>,
    interpolator: &dyn Interpolator3<fdt>,
    mut beams: ElectronBeamSwarm<A>,
) where
    A: Accelerator,
{
    if let Some(extra_fixed_scalars) = root_arguments
        .values_of("extra-fixed-scalars")
        .map(|values| values.collect::<Vec<_>>())
    {
        for name in extra_fixed_scalars {
            let name = name.to_lowercase();
            beams.extract_fixed_scalars(
                exit_on_error!(
                    snapshot.provide_scalar_field(&name).as_ref(),
                    "Error: Could not read quantity {0} from snapshot: {1}",
                    &name
                ),
                interpolator,
            );
        }
    }
    if let Some(extra_fixed_vectors) = root_arguments
        .values_of("extra-fixed-vectors")
        .map(|values| values.collect::<Vec<_>>())
    {
        for name in extra_fixed_vectors {
            let name = name.to_lowercase();
            beams.extract_fixed_vectors(
                exit_on_error!(
                    snapshot.provide_vector_field(&name).as_ref(),
                    "Error: Could not read quantity {0} from snapshot: {1}",
                    &name
                ),
                interpolator,
            );
        }
    }
    if let Some(extra_varying_scalars) = root_arguments
        .values_of("extra-varying-scalars")
        .map(|values| values.collect::<Vec<_>>())
    {
        for name in extra_varying_scalars {
            let name = name.to_lowercase();
            beams.extract_varying_scalars(
                exit_on_error!(
                    snapshot.provide_scalar_field(&name).as_ref(),
                    "Error: Could not read quantity {0} from snapshot: {1}",
                    &name
                ),
                interpolator,
            );
        }
    }
    if let Some(extra_varying_vectors) = root_arguments
        .values_of("extra-varying-vectors")
        .map(|values| values.collect::<Vec<_>>())
    {
        for name in extra_varying_vectors {
            let name = name.to_lowercase();
            beams.extract_varying_vectors(
                exit_on_error!(
                    snapshot.provide_vector_field(&name).as_ref(),
                    "Error: Could not read quantity {0} from snapshot: {1}",
                    &name
                ),
                interpolator,
            );
        }
    }

    if beams.verbosity().print_messages() {
        println!(
            "Saving beams in {}",
            atomic_output_file
                .target_path()
                .file_name()
                .unwrap()
                .to_string_lossy()
        );
    }

    exit_on_error!(
        match output_type {
            OutputType::Fl => beams.save_into_custom_binary(atomic_output_file.temporary_path()),
            #[cfg(feature = "pickle")]
            OutputType::Pickle =>
                beams.save_as_combined_pickles(atomic_output_file.temporary_path()),
            #[cfg(feature = "json")]
            OutputType::Json => beams.save_as_json(atomic_output_file.temporary_path()),
            #[cfg(feature = "hdf5")]
            OutputType::H5Part => beams.save_as_h5part(
                atomic_output_file.temporary_path(),
                extra_atomic_output_file.as_ref().unwrap().temporary_path(),
                root_arguments.is_present("drop-h5part-id"),
            ),
        },
        "Error: Could not save output data: {}"
    );

    exit_on_error!(
        io_context.close_atomic_output_file(atomic_output_file),
        "Error: Could not move temporary output file to target path: {}"
    );
    if let Some(extra_atomic_output_file) = extra_atomic_output_file {
        exit_on_error!(
            io_context.close_atomic_output_file(extra_atomic_output_file),
            "Error: Could not move temporary output file to target path: {}"
        );
    }
}
