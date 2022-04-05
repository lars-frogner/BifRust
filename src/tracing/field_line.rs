//! Field lines in vector fields.

pub mod basic;

use super::{
    ftr,
    stepping::{Stepper3, StepperFactory3},
};
use crate::{
    field::{ScalarField3, VectorField3},
    geometry::{Dim3, Point3, Vec3},
    grid::Grid3,
    interpolation::Interpolator3,
    io::{
        snapshot::{fdt, SnapshotCacher3, SnapshotProvider3},
        utils, Endianness, Verbose,
    },
    num::BFloat,
    seeding::Seeder3,
};
use rayon::prelude::*;
use std::{collections::HashMap, fs, io, mem, path::Path};

#[cfg(feature = "serialization")]
use serde::{
    ser::{SerializeStruct, Serializer},
    Serialize,
};

#[cfg(feature = "hdf5")]
use crate::io_result;
#[cfg(feature = "hdf5")]
use hdf5_rs as hdf5;

type FieldLinePath3 = (Vec<ftr>, Vec<ftr>, Vec<ftr>);
type FixedScalarValues = HashMap<String, Vec<ftr>>;
type FixedVector3Values = HashMap<String, Vec<Vec3<ftr>>>;
type VaryingScalarValues = HashMap<String, Vec<Vec<ftr>>>;
type VaryingVector3Values = HashMap<String, Vec<Vec<Vec3<ftr>>>>;

/// Defines the properties of a field line tracer for a 3D vector field.
pub trait FieldLineTracer3 {
    type Data;

    /// Traces a field line through a 3D vector field.
    ///
    /// # Parameters
    ///
    /// - `field_name`: Name of the vector field to trace.
    /// - `snapshot`: Snapshot cacher where the vector field to trace is cached.
    /// - `interpolator`: Interpolator to use.
    /// - `stepper`: Stepper to use (will be consumed).
    /// - `start_position`: Position where the tracing should start.
    ///
    /// # Returns
    ///
    /// An `Option` which is either:
    ///
    /// - `Some`: Contains a `FieldLineData3` object representing the traced field line.
    /// - `None`: No field line was traced.
    ///
    /// # Type parameters
    ///
    /// - `G`: Type of grid.
    /// - `I`: Type of interpolator.
    /// - `St`: Type of stepper.
    fn trace<G, P, I, St>(
        &self,
        field_name: &str,
        snapshot: &SnapshotCacher3<G, P>,
        interpolator: &I,
        stepper: St,
        start_position: &Point3<ftr>,
    ) -> Option<Self::Data>
    where
        G: Grid3<fdt>,
        P: SnapshotProvider3<G>,
        I: Interpolator3,
        St: Stepper3;
}

/// Collection of 3D field lines.
#[derive(Clone, Debug)]
pub struct FieldLineSet3 {
    lower_bounds: Vec3<ftr>,
    upper_bounds: Vec3<ftr>,
    properties: FieldLineSetProperties3,
    verbose: Verbose,
}

/// Holds the data associated with a set of 3D field lines.
#[derive(Clone, Debug)]
pub struct FieldLineSetProperties3 {
    /// Number of field lines in the set.
    pub number_of_field_lines: usize,
    /// Scalar values defined at the start positions of the field lines.
    pub fixed_scalar_values: FixedScalarValues,
    /// Vector values defined at the start positions of the field lines.
    pub fixed_vector_values: FixedVector3Values,
    /// Scalar values defined along the paths of the field lines.
    pub varying_scalar_values: VaryingScalarValues,
    /// Vector values defined along the paths of the field lines.
    pub varying_vector_values: VaryingVector3Values,
}

impl FieldLineSet3 {
    /// Creates a new 3D field line set with the given bounds and properties.
    pub fn new(
        lower_bounds: Vec3<ftr>,
        upper_bounds: Vec3<ftr>,
        properties: FieldLineSetProperties3,
        verbose: Verbose,
    ) -> Self {
        Self {
            lower_bounds,
            upper_bounds,
            properties,
            verbose,
        }
    }

    /// Traces all the field lines in the set from positions generated by the given seeder.
    ///
    /// # Parameters
    ///
    /// - `field_name`: Name of the vector field to trace.
    /// - `snapshot`: Snapshot cacher where the vector field to trace is cached.
    /// - `seeder`: Seeder to use for generating start positions.
    /// - `tracer`: Field line tracer to use.
    /// - `interpolator`: Interpolator to use.
    /// - `stepper_factory`: Factory structure to use for producing steppers.
    /// - `verbose`: Whether to print status messages.
    ///
    /// # Returns
    ///
    /// A new `FieldLineSet3` with traced field lines.
    ///
    /// # Type parameters
    ///
    /// - `Sd`: Type of seeder.
    /// - `Tr`: Type of field line tracer.
    /// - `G`: Type of grid.
    /// - `I`: Type of interpolator.
    /// - `StF`: Type of stepper factory.
    pub fn trace<Sd, Tr, G, P, I, StF>(
        field_name: &str,
        snapshot: &SnapshotCacher3<G, P>,
        seeder: Sd,
        tracer: &Tr,
        interpolator: &I,
        stepper_factory: &StF,
        verbose: Verbose,
    ) -> Self
    where
        Sd: Seeder3,
        Tr: FieldLineTracer3 + Sync,
        <Tr as FieldLineTracer3>::Data: Send,
        FieldLineSetProperties3: FromParallelIterator<<Tr as FieldLineTracer3>::Data>,
        G: Grid3<fdt>,
        P: SnapshotProvider3<G> + Sync,
        I: Interpolator3,
        StF: StepperFactory3 + Sync,
    {
        if verbose.is_yes() {
            println!("Found {} start positions", seeder.number_of_points());
        }

        let properties: FieldLineSetProperties3 = seeder
            .into_par_iter()
            .filter_map(|start_position| {
                tracer.trace(
                    field_name,
                    snapshot,
                    interpolator,
                    stepper_factory.produce(),
                    &Point3::from(&start_position),
                )
            })
            .collect();

        if verbose.is_yes() {
            println!(
                "Successfully traced {} field lines",
                properties.number_of_field_lines
            );
        }

        let lower_bounds = Vec3::from(snapshot.grid().lower_bounds());
        let upper_bounds = Vec3::from(snapshot.grid().upper_bounds());

        Self::new(lower_bounds, upper_bounds, properties, verbose)
    }

    /// Whether the field line set is verbose.
    pub fn verbose(&self) -> Verbose {
        self.verbose
    }

    /// Returns the number of field lines making up the field line set.
    pub fn number_of_field_lines(&self) -> usize {
        self.properties.number_of_field_lines
    }

    /// Extracts and stores the value of the given scalar field at the initial position for each field line.
    pub fn extract_fixed_scalars<F, G, I>(&mut self, field: &ScalarField3<F, G>, interpolator: &I)
    where
        F: BFloat,
        G: Grid3<F>,
        I: Interpolator3,
    {
        if self.verbose.is_yes() {
            println!("Extracting {} at initial positions", field.name());
        }
        let initial_coords_x = &self.properties.fixed_scalar_values["x0"];
        let initial_coords_y = &self.properties.fixed_scalar_values["y0"];
        let initial_coords_z = &self.properties.fixed_scalar_values["z0"];
        let values = initial_coords_x
            .into_par_iter()
            .zip(initial_coords_y)
            .zip(initial_coords_z)
            .map(|((&field_line_x0, &field_line_y0), &field_line_z0)| {
                let acceleration_position =
                    Point3::from_components(field_line_x0, field_line_y0, field_line_z0);
                let value = interpolator
                    .interp_scalar_field(field, &acceleration_position)
                    .expect_inside();
                num::NumCast::from(value).expect("Conversion failed")
            })
            .collect();
        self.properties
            .fixed_scalar_values
            .insert(format!("{}0", field.name()), values);
    }

    /// Extracts and stores the value of the given vector field at the initial position for each field line.
    pub fn extract_fixed_vectors<F, G, I>(&mut self, field: &VectorField3<F, G>, interpolator: &I)
    where
        F: BFloat,
        G: Grid3<F>,
        I: Interpolator3,
    {
        if self.verbose.is_yes() {
            println!("Extracting {} at initial positions", field.name());
        }
        let initial_coords_x = &self.properties.fixed_scalar_values["x0"];
        let initial_coords_y = &self.properties.fixed_scalar_values["y0"];
        let initial_coords_z = &self.properties.fixed_scalar_values["z0"];
        let vectors = initial_coords_x
            .into_par_iter()
            .zip(initial_coords_y)
            .zip(initial_coords_z)
            .map(|((&field_line_x0, &field_line_y0), &field_line_z0)| {
                let acceleration_position =
                    Point3::from_components(field_line_x0, field_line_y0, field_line_z0);
                let vector = interpolator
                    .interp_vector_field(field, &acceleration_position)
                    .expect_inside();
                Vec3::from(&vector)
            })
            .collect();
        self.properties
            .fixed_vector_values
            .insert(format!("{}0", field.name()), vectors);
    }

    /// Extracts and stores the value of the given scalar field at each position for each field line.
    pub fn extract_varying_scalars<F, G, I>(&mut self, field: &ScalarField3<F, G>, interpolator: &I)
    where
        F: BFloat,
        G: Grid3<F>,
        I: Interpolator3,
    {
        if self.verbose.is_yes() {
            println!("Extracting {} along field line paths", field.name());
        }
        let coords_x = &self.properties.varying_scalar_values["x"];
        let coords_y = &self.properties.varying_scalar_values["y"];
        let coords_z = &self.properties.varying_scalar_values["z"];
        let values = coords_x
            .into_par_iter()
            .zip(coords_y)
            .zip(coords_z)
            .map(
                |((field_line_coords_x, field_line_coords_y), field_line_coords_z)| {
                    field_line_coords_x
                        .iter()
                        .zip(field_line_coords_y)
                        .zip(field_line_coords_z)
                        .map(|((&field_line_x, &field_line_y), &field_line_z)| {
                            let position =
                                Point3::from_components(field_line_x, field_line_y, field_line_z);
                            let value = interpolator
                                .interp_scalar_field(field, &position)
                                .expect_inside();
                            num::NumCast::from(value).expect("Conversion failed")
                        })
                        .collect()
                },
            )
            .collect();
        self.properties
            .varying_scalar_values
            .insert(field.name().to_string(), values);
    }

    /// Extracts and stores the value of the given vector field at each position for each field line.
    pub fn extract_varying_vectors<F, G, I>(&mut self, field: &VectorField3<F, G>, interpolator: &I)
    where
        F: BFloat,
        G: Grid3<F>,
        I: Interpolator3,
    {
        if self.verbose.is_yes() {
            println!("Extracting {} along field line paths", field.name());
        }
        let coords_x = &self.properties.varying_scalar_values["x"];
        let coords_y = &self.properties.varying_scalar_values["y"];
        let coords_z = &self.properties.varying_scalar_values["z"];
        let vectors = coords_x
            .into_par_iter()
            .zip(coords_y)
            .zip(coords_z)
            .map(
                |((field_line_coords_x, field_line_coords_y), field_line_coords_z)| {
                    field_line_coords_x
                        .iter()
                        .zip(field_line_coords_y)
                        .zip(field_line_coords_z)
                        .map(|((&field_line_x, &field_line_y), &field_line_z)| {
                            let position =
                                Point3::from_components(field_line_x, field_line_y, field_line_z);
                            let vector = interpolator
                                .interp_vector_field(field, &position)
                                .expect_inside();
                            Vec3::from(&vector)
                        })
                        .collect()
                },
            )
            .collect();
        self.properties
            .varying_vector_values
            .insert(field.name().to_string(), vectors);
    }

    /// Extracts and stores the magnitude of the given vector field at each position for each field line.
    pub fn extract_varying_vector_magnitudes<F, G, I>(
        &mut self,
        field: &VectorField3<F, G>,
        interpolator: &I,
    ) where
        F: BFloat,
        G: Grid3<F>,
        I: Interpolator3,
    {
        if self.verbose.is_yes() {
            println!("Extracting |{}| along field line paths", field.name());
        }
        let coords_x = &self.properties.varying_scalar_values["x"];
        let coords_y = &self.properties.varying_scalar_values["y"];
        let coords_z = &self.properties.varying_scalar_values["z"];
        let values = coords_x
            .into_par_iter()
            .zip(coords_y)
            .zip(coords_z)
            .map(
                |((field_line_coords_x, field_line_coords_y), field_line_coords_z)| {
                    field_line_coords_x
                        .iter()
                        .zip(field_line_coords_y)
                        .zip(field_line_coords_z)
                        .map(|((&field_line_x, &field_line_y), &field_line_z)| {
                            let position =
                                Point3::from_components(field_line_x, field_line_y, field_line_z);
                            let value = interpolator
                                .interp_vector_field(field, &position)
                                .expect_inside()
                                .length();
                            num::NumCast::from(value).expect("Conversion failed")
                        })
                        .collect()
                },
            )
            .collect();
        self.properties
            .varying_scalar_values
            .insert(field.name().to_string(), values);
    }

    /// Serializes the field line data into JSON format and writes to the given writer.
    #[cfg(feature = "json")]
    pub fn write_as_json<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        utils::write_data_as_json(writer, &self)
    }

    /// Serializes the field line data into JSON format and saves at the given path.
    #[cfg(feature = "json")]
    pub fn save_as_json<P: AsRef<Path>>(&self, output_file_path: P) -> io::Result<()> {
        utils::save_data_as_json(output_file_path, &self)
    }

    /// Serializes the field line data into pickle format and writes to the given writer.
    ///
    /// All the field line data is saved as a single pickled structure.
    #[cfg(feature = "pickle")]
    pub fn write_as_pickle<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        utils::write_data_as_pickle(writer, &self)
    }

    /// Serializes the field line data into pickle format and saves at the given path.
    ///
    /// All the field line data is saved as a single pickled structure.
    #[cfg(feature = "pickle")]
    pub fn save_as_pickle<P: AsRef<Path>>(&self, output_file_path: P) -> io::Result<()> {
        utils::save_data_as_pickle(output_file_path, &self)
    }

    /// Serializes the field line data fields in parallel into pickle format and writes to the given writer.
    ///
    /// The data fields are written as separate pickle objects.
    #[cfg(feature = "pickle")]
    pub fn write_as_combined_pickles<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut buffer_1 = Vec::new();
        utils::write_data_as_pickle(&mut buffer_1, &self.lower_bounds)?;
        let mut buffer_2 = Vec::new();
        utils::write_data_as_pickle(&mut buffer_2, &self.upper_bounds)?;
        let mut buffer_3 = Vec::new();
        utils::write_data_as_pickle(&mut buffer_3, &self.number_of_field_lines())?;

        let (mut result_4, mut result_5, mut result_6, mut result_7) =
            (Ok(()), Ok(()), Ok(()), Ok(()));
        let (mut buffer_4, mut buffer_5, mut buffer_6, mut buffer_7) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        rayon::scope(|s| {
            s.spawn(|_| {
                result_4 =
                    utils::write_data_as_pickle(&mut buffer_4, &self.properties.fixed_scalar_values)
            });
            s.spawn(|_| {
                result_5 =
                    utils::write_data_as_pickle(&mut buffer_5, &self.properties.fixed_vector_values)
            });
            s.spawn(|_| {
                result_6 = utils::write_data_as_pickle(
                    &mut buffer_6,
                    &self.properties.varying_scalar_values,
                )
            });
            s.spawn(|_| {
                result_7 = utils::write_data_as_pickle(
                    &mut buffer_7,
                    &self.properties.varying_vector_values,
                )
            });
        });
        result_4?;
        result_5?;
        result_6?;
        result_7?;

        writer.write_all(
            &[
                buffer_1, buffer_2, buffer_3, buffer_4, buffer_5, buffer_6, buffer_7,
            ]
            .concat(),
        )?;
        Ok(())
    }

    /// Serializes the field line data fields in parallel into pickle format and saves at the given path.
    ///
    /// The data fields are saved as separate pickle objects in the same file.
    #[cfg(feature = "pickle")]
    pub fn save_as_combined_pickles<P: AsRef<Path>>(&self, output_file_path: P) -> io::Result<()> {
        let mut file = utils::create_file_and_required_directories(output_file_path)?;
        self.write_as_combined_pickles(&mut file)
    }

    /// Serializes the field line data into a custom binary format and writes to the given writer.
    pub fn write_as_custom_binary<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        write_field_line_data_as_custom_binary(
            writer,
            &self.lower_bounds,
            &self.upper_bounds,
            self.properties.clone(),
        )
    }

    /// Serializes the field line data into a custom binary format and saves at the given path.
    pub fn save_as_custom_binary<P: AsRef<Path>>(&self, output_file_path: P) -> io::Result<()> {
        save_field_line_data_as_custom_binary(
            output_file_path,
            &self.lower_bounds,
            &self.upper_bounds,
            self.properties.clone(),
        )
        .map(|_| ())
    }

    /// Serializes the field line data into a H5Part format and saves to the given path.
    #[cfg(feature = "hdf5")]
    pub fn save_as_h5part<P: AsRef<Path>>(
        &self,
        output_file_path: P,
        output_seed_file_path: P,
        drop_id: bool,
    ) -> io::Result<()> {
        save_field_line_data_as_h5part(
            output_file_path,
            output_seed_file_path,
            self.properties.clone(),
            drop_id,
        )
    }

    /// Serializes the field line data into a custom binary format and writes to the given writer,
    /// consuming the field line set in the process.
    pub fn write_into_custom_binary<W: io::Write>(self, writer: &mut W) -> io::Result<()> {
        write_field_line_data_as_custom_binary(
            writer,
            &self.lower_bounds,
            &self.upper_bounds,
            self.properties,
        )
    }

    /// Serializes the field line data into a custom binary format and saves at the given path,
    /// consuming the field line set in the process.
    pub fn save_into_custom_binary<P: AsRef<Path>>(self, output_file_path: P) -> io::Result<()> {
        save_field_line_data_as_custom_binary(
            output_file_path,
            &self.lower_bounds,
            &self.upper_bounds,
            self.properties,
        )
        .map(|_| ())
    }

    /// Serializes the field line data into a H5Part format and saves to the given path,
    /// consuming the field line set in the process.
    #[cfg(feature = "hdf5")]
    pub fn save_into_h5part<P: AsRef<Path>>(
        self,
        output_file_path: P,
        output_seed_file_path: P,
        drop_id: bool,
    ) -> io::Result<()> {
        save_field_line_data_as_h5part(
            output_file_path,
            output_seed_file_path,
            self.properties,
            drop_id,
        )
    }
}

impl Default for FieldLineSetProperties3 {
    fn default() -> Self {
        Self {
            number_of_field_lines: 0,
            fixed_scalar_values: FixedScalarValues::default(),
            fixed_vector_values: FixedVector3Values::default(),
            varying_scalar_values: VaryingScalarValues::default(),
            varying_vector_values: VaryingVector3Values::default(),
        }
    }
}

#[cfg(feature = "serialization")]
impl Serialize for FieldLineSet3 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("FieldLineSet3", 7)?;
        s.serialize_field("lower_bounds", &self.lower_bounds)?;
        s.serialize_field("upper_bounds", &self.upper_bounds)?;
        s.serialize_field("number_of_field_lines", &self.number_of_field_lines())?;
        s.serialize_field("fixed_scalar_values", &self.properties.fixed_scalar_values)?;
        s.serialize_field("fixed_vector_values", &self.properties.fixed_vector_values)?;
        s.serialize_field(
            "varying_scalar_values",
            &self.properties.varying_scalar_values,
        )?;
        s.serialize_field(
            "varying_vector_values",
            &self.properties.varying_vector_values,
        )?;
        s.end()
    }
}

/// Writes the given field line data in a custom binary format at the
/// given path.
pub fn save_field_line_data_as_custom_binary<P: AsRef<Path>>(
    output_file_path: P,
    lower_bounds: &Vec3<ftr>,
    upper_bounds: &Vec3<ftr>,
    properties: FieldLineSetProperties3,
) -> io::Result<fs::File> {
    let mut file = utils::create_file_and_required_directories(output_file_path)?;
    write_field_line_data_as_custom_binary(&mut file, lower_bounds, upper_bounds, properties)?;
    Ok(file)
}

/// Writes the given field line data in a custom binary format into
/// the given writer.
pub fn write_field_line_data_as_custom_binary<W: io::Write>(
    writer: &mut W,
    lower_bounds: &Vec3<ftr>,
    upper_bounds: &Vec3<ftr>,
    properties: FieldLineSetProperties3,
) -> io::Result<()> {
    // Field line file format:
    // [HEADER]
    // float_size: u64
    // number_of_field_lines: u64
    // number_of_field_line_elements: u64
    // number_of_fixed_scalar_quantities: u64
    // number_of_fixed_vector_quantities: u64
    // number_of_varying_scalar_quantities: u64
    // number_of_varying_vector_quantities: u64
    // bounds: [ftr; 6]
    // names: string with each name followed by a newline
    // start_indices_of_field_line_elements: [u64; number_of_field_lines]
    // [BODY]
    // flat_fixed_scalar_values:   [ftr: number_of_fixed_scalar_quantities*number_of_field_lines  ]
    // flat_fixed_vector_values:   [ftr: number_of_fixed_vector_quantities*number_of_field_lines*3]
    // flat_varying_scalar_values: [ftr: number_of_varying_scalar_quantities*number_of_field_line_elements  ]
    // flat_varying_vector_values: [ftr: number_of_varying_vector_quantities*number_of_field_line_elements*3]

    const ENDIANNESS: Endianness = Endianness::Little;

    let FieldLineSetProperties3 {
        number_of_field_lines,
        fixed_scalar_values,
        fixed_vector_values,
        varying_scalar_values,
        varying_vector_values,
    } = properties;

    let number_of_fixed_scalar_quantities = fixed_scalar_values.len();
    let number_of_fixed_vector_quantities = fixed_vector_values.len();
    let number_of_varying_scalar_quantities = varying_scalar_values.len();
    let number_of_varying_vector_quantities = varying_vector_values.len();

    let (number_of_field_line_elements, start_indices_of_field_line_elements) =
        if varying_scalar_values.is_empty() {
            if varying_vector_values.is_empty() {
                (0, Vec::new())
            } else {
                let (_, varying_vectors) = varying_vector_values.iter().next().unwrap();

                let number_of_field_line_elements: usize =
                    varying_vectors.iter().map(|vec| vec.len()).sum();

                let start_indices_of_field_line_elements: Vec<_> = varying_vectors
                    .iter()
                    .scan(0, |count, vec| {
                        let idx = *count;
                        *count += vec.len();
                        Some(idx as u64)
                    })
                    .collect();

                (
                    number_of_field_line_elements,
                    start_indices_of_field_line_elements,
                )
            }
        } else {
            let (_, varying_scalars) = varying_scalar_values.iter().next().unwrap();

            let number_of_field_line_elements: usize =
                varying_scalars.iter().map(|vec| vec.len()).sum();

            let start_indices_of_field_line_elements: Vec<_> = varying_scalars
                .iter()
                .scan(0, |count, vec| {
                    let idx = *count;
                    *count += vec.len();
                    Some(idx as u64)
                })
                .collect();

            (
                number_of_field_line_elements,
                start_indices_of_field_line_elements,
            )
        };

    let mut fixed_scalar_names = Vec::new();
    let mut flat_fixed_scalar_values = Vec::new();

    let set_fixed_scalar_variables =
        |fixed_scalar_names: &mut Vec<_>, flat_fixed_scalar_values: &mut Vec<_>| {
            fixed_scalar_names.reserve_exact(number_of_fixed_scalar_quantities);
            flat_fixed_scalar_values
                .reserve_exact(number_of_fixed_scalar_quantities * number_of_field_lines);
            for (name, values) in fixed_scalar_values {
                fixed_scalar_names.push(name);
                flat_fixed_scalar_values.extend(values.into_iter());
            }
        };

    let mut fixed_vector_names = Vec::new();
    let mut flat_fixed_vector_values = Vec::new();

    let set_fixed_vector_variables =
        |fixed_vector_names: &mut Vec<_>, flat_fixed_vector_values: &mut Vec<ftr>| {
            fixed_vector_names.reserve_exact(number_of_fixed_vector_quantities);
            flat_fixed_vector_values
                .reserve_exact(number_of_fixed_vector_quantities * number_of_field_lines * 3);
            for (name, values) in fixed_vector_values {
                fixed_vector_names.push(name);
                for vec3 in values {
                    flat_fixed_vector_values.extend(vec3.into_iter());
                }
            }
        };

    let mut varying_scalar_names = Vec::new();
    let mut flat_varying_scalar_values = Vec::new();

    let set_varying_scalar_variables =
        |varying_scalar_names: &mut Vec<_>, flat_varying_scalar_values: &mut Vec<_>| {
            varying_scalar_names.reserve_exact(number_of_varying_scalar_quantities);
            flat_varying_scalar_values
                .reserve_exact(number_of_varying_scalar_quantities * number_of_field_line_elements);
            for (name, values) in varying_scalar_values {
                varying_scalar_names.push(name);
                for vec in values {
                    flat_varying_scalar_values.extend(vec.into_iter());
                }
            }
        };

    let mut varying_vector_names = Vec::new();
    let mut flat_varying_vector_values = Vec::new();

    let set_varying_vector_variables =
        |varying_vector_names: &mut Vec<_>, flat_varying_vector_values: &mut Vec<ftr>| {
            varying_vector_names.reserve_exact(number_of_varying_vector_quantities);
            flat_varying_vector_values.reserve_exact(
                number_of_varying_vector_quantities * number_of_field_line_elements * 3,
            );
            for (name, values) in varying_vector_values {
                varying_vector_names.push(name);
                for vec in values {
                    for vec3 in vec {
                        flat_varying_vector_values.extend(vec3.into_iter());
                    }
                }
            }
        };

    rayon::scope(|s| {
        s.spawn(|_| {
            if number_of_fixed_scalar_quantities > 0 {
                set_fixed_scalar_variables(&mut fixed_scalar_names, &mut flat_fixed_scalar_values);
            }
        });
        s.spawn(|_| {
            if number_of_fixed_vector_quantities > 0 {
                set_fixed_vector_variables(&mut fixed_vector_names, &mut flat_fixed_vector_values);
            }
        });
        s.spawn(|_| {
            if number_of_varying_scalar_quantities > 0 {
                set_varying_scalar_variables(
                    &mut varying_scalar_names,
                    &mut flat_varying_scalar_values,
                );
            }
        });
        s.spawn(|_| {
            if number_of_varying_vector_quantities > 0 {
                set_varying_vector_variables(
                    &mut varying_vector_names,
                    &mut flat_varying_vector_values,
                );
            }
        });
    });

    let mut names = Vec::with_capacity(
        number_of_fixed_scalar_quantities
            + number_of_fixed_vector_quantities
            + number_of_varying_scalar_quantities
            + number_of_varying_vector_quantities,
    );
    names.extend(fixed_scalar_names.into_iter());
    names.extend(fixed_vector_names.into_iter());
    names.extend(varying_scalar_names.into_iter());
    names.extend(varying_vector_names.into_iter());
    let mut names = names.join("\n");
    names.push('\n');

    let u64_size = mem::size_of::<u64>();
    let u8_size = mem::size_of::<u8>();
    let float_size = mem::size_of::<ftr>();

    let section_sizes = [
        7 * u64_size,
        6 * float_size,
        names.len() * u8_size,
        number_of_field_lines * u64_size,
        number_of_fixed_scalar_quantities * number_of_field_lines * float_size,
        number_of_fixed_vector_quantities * number_of_field_lines * 3 * float_size,
        number_of_varying_scalar_quantities * number_of_field_line_elements * float_size,
        number_of_varying_vector_quantities * number_of_field_line_elements * 3 * float_size,
    ];

    let byte_buffer_size = *section_sizes.iter().max().unwrap();
    let mut byte_buffer = vec![0_u8; byte_buffer_size];

    let byte_offset = utils::write_into_byte_buffer(
        &[
            float_size as u64,
            number_of_field_lines as u64,
            number_of_field_line_elements as u64,
            number_of_fixed_scalar_quantities as u64,
            number_of_fixed_vector_quantities as u64,
            number_of_varying_scalar_quantities as u64,
            number_of_varying_vector_quantities as u64,
        ],
        &mut byte_buffer,
        0,
        ENDIANNESS,
    );
    writer.write_all(&byte_buffer[..byte_offset])?;

    let byte_offset = utils::write_into_byte_buffer(
        &[
            lower_bounds[Dim3::X],
            upper_bounds[Dim3::X],
            lower_bounds[Dim3::Y],
            upper_bounds[Dim3::Y],
            lower_bounds[Dim3::Z],
            upper_bounds[Dim3::Z],
        ],
        &mut byte_buffer,
        0,
        ENDIANNESS,
    );
    writer.write_all(&byte_buffer[..byte_offset])?;

    write!(writer, "{}", names)?;

    if number_of_field_line_elements > 0 {
        let byte_offset = utils::write_into_byte_buffer(
            &start_indices_of_field_line_elements,
            &mut byte_buffer,
            0,
            ENDIANNESS,
        );
        mem::drop(start_indices_of_field_line_elements);
        writer.write_all(&byte_buffer[..byte_offset])?;
    }

    if number_of_fixed_scalar_quantities > 0 {
        let byte_offset = utils::write_into_byte_buffer(
            &flat_fixed_scalar_values,
            &mut byte_buffer,
            0,
            ENDIANNESS,
        );
        mem::drop(flat_fixed_scalar_values);
        writer.write_all(&byte_buffer[..byte_offset])?;
    }

    if number_of_fixed_vector_quantities > 0 {
        let byte_offset = utils::write_into_byte_buffer(
            &flat_fixed_vector_values,
            &mut byte_buffer,
            0,
            ENDIANNESS,
        );
        mem::drop(flat_fixed_vector_values);
        writer.write_all(&byte_buffer[..byte_offset])?;
    }

    if number_of_varying_scalar_quantities > 0 {
        let byte_offset = utils::write_into_byte_buffer(
            &flat_varying_scalar_values,
            &mut byte_buffer,
            0,
            ENDIANNESS,
        );
        mem::drop(flat_varying_scalar_values);
        writer.write_all(&byte_buffer[..byte_offset])?;
    }

    if number_of_varying_vector_quantities > 0 {
        let byte_offset = utils::write_into_byte_buffer(
            &flat_varying_vector_values,
            &mut byte_buffer,
            0,
            ENDIANNESS,
        );
        mem::drop(flat_varying_vector_values);
        writer.write_all(&byte_buffer[..byte_offset])?;
    }

    Ok(())
}

/// Saves the given field line data as a H5Part file at the given path.
#[cfg(feature = "hdf5")]
pub fn save_field_line_data_as_h5part<P: AsRef<Path>>(
    file_path: P,
    seed_file_path: P,
    properties: FieldLineSetProperties3,
    drop_id: bool,
) -> io::Result<()> {
    let FieldLineSetProperties3 {
        number_of_field_lines,
        fixed_scalar_values,
        varying_scalar_values,
        ..
    } = properties;

    if number_of_field_lines == 0 {
        eprintln!("Warning: No data to write to H5Part file");
        return Ok(());
    }

    let number_of_fixed_scalar_quantities = fixed_scalar_values.len();
    let number_of_varying_scalar_quantities = varying_scalar_values.len();

    if number_of_fixed_scalar_quantities == 0 && number_of_varying_scalar_quantities == 0 {
        eprintln!("Warning: No data to write to H5Part file");
        return Ok(());
    }

    if number_of_varying_scalar_quantities > 0 {
        utils::create_directory_if_missing(&file_path)?;
        let group = io_result!(io_result!(hdf5::File::create(file_path))?.create_group("Step#0"))?;

        let number_of_field_line_elements: usize = varying_scalar_values
            .iter()
            .next()
            .unwrap()
            .1
            .iter()
            .map(|vec| vec.len())
            .sum();

        if number_of_field_line_elements > 0 {
            let mut concatenated_values = Vec::with_capacity(number_of_field_line_elements);

            for (name, values) in varying_scalar_values {
                for vec in values {
                    concatenated_values.extend(vec.into_iter());
                }
                let name = if name == "r" { "rho" } else { &name }; // `r` is reserved for radial distance
                io_result!(group
                    .new_dataset_builder()
                    .with_data(&concatenated_values)
                    .create(name))?;
                concatenated_values.clear();
            }

            if !drop_id {
                io_result!(group
                    .new_dataset_builder()
                    .with_data(&(0..number_of_field_line_elements as u64).collect::<Vec<_>>())
                    .create("id"))?;
            }
        }
    }

    if number_of_fixed_scalar_quantities > 0 {
        utils::create_directory_if_missing(&seed_file_path)?;
        let group =
            io_result!(io_result!(hdf5::File::create(seed_file_path))?.create_group("Step#0"))?;

        for (name, values) in fixed_scalar_values {
            io_result!(group
                .new_dataset_builder()
                .with_data(&values)
                .create(&*name))?;
        }

        if !drop_id {
            io_result!(group
                .new_dataset_builder()
                .with_data(&(0..number_of_field_lines as u64).collect::<Vec<_>>())
                .create("id"))?;
        }
    }

    Ok(())
}
