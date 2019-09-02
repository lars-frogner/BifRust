//! Geometric utility objects.

use num;
use std::ops::{Index, IndexMut, Add, Sub, Mul, Div};
use ndarray::prelude::*;
use serde::Serialize;

/// Denotes the x-, y- or z-dimension.
#[derive(Debug, Copy, Clone)]
pub enum Dim3 {
    X = 0,
    Y = 1,
    Z = 2
}

impl Dim3 {
    /// Creates an array for iterating over the x-, y- and z-dimensions.
    pub fn slice() -> [Self; 3] { [Dim3::X, Dim3::Y, Dim3::Z] }

    /// Creates an array for iterating over the x- and y-dimensions.
    pub fn slice_xy() -> [Self; 2] { [Dim3::X, Dim3::Y] }

    /// Creates an array for iterating over all three dimensions except the given one.
    pub fn slice_except(dim: Self) -> [Self; 2] {
        match dim {
            Dim3::X => [Dim3::Y, Dim3::Z],
            Dim3::Y => [Dim3::X, Dim3::Z],
            Dim3::Z => [Dim3::X, Dim3::Y]
        }
    }
}

use Dim3::{X, Y, Z};

/// Denotes the x- or y-dimension.
#[derive(Debug, Copy, Clone)]
pub enum Dim2 {
    X = 0,
    Y = 1
}

impl Dim2 {
    /// Creates an array for iterating over the x- and y-dimensions.
    pub fn slice() -> [Self; 2] { [Dim2::X, Dim2::Y] }
}

/// Represents any quantity with three dimensional components.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct In3D<T>([T; 3]);

impl<T> In3D<T> {
    /// Creates a new 3D quantity given the three components.
    pub fn new(x: T, y: T, z: T) -> Self { In3D([x, y, z]) }

    /// Creates a new 3D quantity with the given value copied into all components.
    pub fn same(a: T) -> Self where T: Copy { In3D([a, a, a]) }

    /// Creates a new 3D quantity with the given value cloned into all components.
    pub fn same_cloned(a: T) -> Self where T: Clone { In3D([a.clone(), a.clone(), a]) }
}

impl<T> Index<Dim3> for In3D<T> {
    type Output = T;
    fn index(&self, dim: Dim3) -> &Self::Output { &self.0[dim as usize] }
}

impl<T> IndexMut<Dim3> for In3D<T> {
    fn index_mut(&mut self, dim: Dim3) -> &mut Self::Output { &mut self.0[dim as usize] }
}

/// Represents any quantity with two dimensional components.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct In2D<T>([T; 2]);

impl<T> In2D<T> {
    /// Creates a new 2D quantity given the two components.
    pub fn new(x: T, y: T) -> Self { In2D([x, y]) }

    /// Creates a new 2D quantity with the given value copied into both components.
    pub fn same(a: T) -> Self where T: Copy { In2D([a, a]) }
}

impl<T> Index<Dim2> for In2D<T> {
    type Output = T;
    fn index(&self, dim: Dim2) -> &Self::Output { &self.0[dim as usize] }
}

impl<T> IndexMut<Dim2> for In2D<T> {
    fn index_mut(&mut self, dim: Dim2) -> &mut Self::Output { &mut self.0[dim as usize] }
}

/// A 3D vector.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vec3<F: num::Float>(In3D<F>);

impl<F: num::Float> Vec3<F> {
    /// Creates a new 3D vector given the three components.
    pub fn new(x: F, y: F, z: F) -> Self { Vec3(In3D::new(x, y, z)) }

    /// Creates a new zero vector.
    pub fn zero() -> Self { Vec3::new(F::zero(), F::zero(), F::zero()) }

    /// Creates a new vector from the given vector, which may have a different component type.
    pub fn from<U: num::Float>(other: &Vec3<U>) -> Self {
        Vec3::new(F::from(other[X]).unwrap(), F::from(other[Y]).unwrap(), F::from(other[Z]).unwrap())
    }

    /// Constructs a new point from the vector components.
    pub fn to_point3(&self) -> Point3<F> {
        Point3::new(self[X], self[Y], self[Z])
    }

    /// Constructs a new vector from the absolute values of the vector components.
    pub fn abs(&self) -> Self {
        Vec3::new(F::abs(self[X]), F::abs(self[Y]), F::abs(self[Z]))
    }

    /// Constructs a new vector by taking the component-wise max with the given vector.
    pub fn max_with(&self, other: &Self) -> Self {
        Vec3::new(F::max(self[X], other[X]), F::max(self[Y], other[Y]), F::max(self[Z], other[Z]))
    }

    /// Computes the squared length of the vector.
    pub fn squared_length(&self) -> F {
        self[X]*self[X] + self[Y]*self[Y] + self[Z]*self[Z]
    }

    /// Computes the length of the vector.
    pub fn length(&self) -> F {
        self.squared_length().sqrt()
    }

    /// Whether the vector is the zero vector.
    pub fn is_zero(&self) -> bool {
        self[X] == F::zero() && self[Y] == F::zero() && self[Z] == F::zero()
    }

    /// Computes the dot product of the vector with another vector.
    pub fn dot(&self, other: &Self) -> F {
        self[X]*other[X] +
        self[Y]*other[Y] +
        self[Z]*other[Z]
    }

    /// Normalizes the vector to have unit length.
    pub fn normalize(&mut self) {
        let length = self.length();
        assert!(length != F::zero());
        let inv_length = length.recip();
        self[X] = self[X]*inv_length;
        self[Y] = self[Y]*inv_length;
        self[Z] = self[Z]*inv_length;
    }

    /// Reverses the direction of the vector.
    pub fn reverse(&mut self) {
        self[X] = -self[X];
        self[Y] = -self[Y];
        self[Z] = -self[Z];
    }
}

impl<F: num::Float> Index<Dim3> for Vec3<F> {
    type Output = F;
    fn index(&self, dim: Dim3) -> &Self::Output { &self.0[dim] }
}

impl<F: num::Float> IndexMut<Dim3> for Vec3<F> {
    fn index_mut(&mut self, dim: Dim3) -> &mut Self::Output { &mut self.0[dim] }
}

impl<'a, F: num::Float> Add<&'a Vec3<F>> for &'a Vec3<F> {
    type Output = Vec3<F>;
    fn add(self, other: Self) -> Self::Output {
        Self::Output::new(self[X] + other[X],
                          self[Y] + other[Y],
                          self[Z] + other[Z])
    }
}

impl<F: num::Float> Add<Vec3<F>> for &Vec3<F> {
    type Output = Vec3<F>;
    fn add(self, other: Vec3<F>) -> Self::Output { self + &other }
}

impl<F: num::Float> Add<Vec3<F>> for Vec3<F> {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output { &self + &other }
}

impl<F: num::Float> Add<&Vec3<F>> for Vec3<F> {
    type Output = Self;
    fn add(self, other: &Self) -> Self::Output { &self + other }
}

impl<'a, F: num::Float> Sub<&'a Vec3<F>> for &'a Vec3<F> {
    type Output = Vec3<F>;
    fn sub(self, other: Self) -> Self::Output {
        Self::Output::new(self[X] - other[X],
                          self[Y] - other[Y],
                          self[Z] - other[Z])
    }
}

impl<F: num::Float> Sub<Vec3<F>> for &Vec3<F> {
    type Output = Vec3<F>;
    fn sub(self, other: Vec3<F>) -> Self::Output { self - &other }
}

impl<F: num::Float> Sub<Vec3<F>> for Vec3<F> {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output { &self - &other }
}

impl<F: num::Float> Sub<&Vec3<F>> for Vec3<F> {
    type Output = Self;
    fn sub(self, other: &Self) -> Self::Output { &self - other }
}

impl<F: num::Float> Mul<F> for &Vec3<F> {
    type Output = Vec3<F>;
    fn mul(self, factor: F) -> Self::Output {
        Self::Output::new(factor*self[X],
                          factor*self[Y],
                          factor*self[Z])
    }
}

impl<F: num::Float> Mul<F> for Vec3<F> {
    type Output = Self;
    fn mul(self, factor: F) -> Self::Output { &self*factor }
}

impl<F: num::Float> Div<F> for &Vec3<F> {
    type Output = Vec3<F>;
    fn div(self, divisor: F) -> Self::Output {
        #![allow(clippy::suspicious_arithmetic_impl)]
        let factor = divisor.recip();
        self*factor
    }
}

impl<F: num::Float> Div<F> for Vec3<F> {
    type Output = Self;
    fn div(self, divisor: F) -> Self::Output { &self/divisor }
}

/// A 2D vector.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vec2<F: num::Float>(In2D<F>);

impl<F: num::Float> Vec2<F> {
    /// Creates a new 2D vector given the three components.
    pub fn new(x: F, y: F) -> Self { Vec2(In2D::new(x, y)) }

    /// Creates a new zero vector.
    pub fn zero() -> Self { Vec2::new(F::zero(), F::zero()) }

    /// Creates a new vector from the given vector, which may have a different component type.
    pub fn from<U: num::Float>(other: &Vec2<U>) -> Self {
        Vec2::new(F::from(other[Dim2::X]).unwrap(), F::from(other[Dim2::Y]).unwrap())
    }

    /// Constructs a new point from the vector components.
    pub fn to_point2(&self) -> Point2<F> {
        Point2::new(self[Dim2::X], self[Dim2::Y])
    }

    /// Constructs a new vector from the absolute values of the vector components.
    pub fn abs(&self) -> Self {
        Vec2::new(F::abs(self[Dim2::X]), F::abs(self[Dim2::Y]))
    }

    /// Constructs a new vector by taking the component-wise max with the given vector.
    pub fn max_with(&self, other: &Self) -> Self {
        Vec2::new(F::max(self[Dim2::X], other[Dim2::X]), F::max(self[Dim2::Y], other[Dim2::Y]))
    }

    /// Computes the squared length of the vector.
    pub fn squared_length(&self) -> F {
        self[Dim2::X]*self[Dim2::X] + self[Dim2::Y]*self[Dim2::Y]
    }

    /// Computes the length of the vector.
    pub fn length(&self) -> F {
        self.squared_length().sqrt()
    }

    /// Whether the vector is the zero vector.
    pub fn is_zero(&self) -> bool {
        self[Dim2::X] == F::zero() && self[Dim2::Y] == F::zero()
    }

    /// Computes the dot product of the vector with another vector.
    pub fn dot(&self, other: &Self) -> F {
        self[Dim2::X]*other[Dim2::X] +
        self[Dim2::Y]*other[Dim2::Y]
    }

    /// Normalizes the vector to have unit length.
    pub fn normalize(&mut self) {
        let length = self.length();
        assert!(length != F::zero());
        let inv_length = length.recip();
        self[Dim2::X] = self[Dim2::X]*inv_length;
        self[Dim2::Y] = self[Dim2::Y]*inv_length;
    }

    /// Reverses the direction of the vector.
    pub fn reverse(&mut self) {
        self[Dim2::X] = -self[Dim2::X];
        self[Dim2::Y] = -self[Dim2::Y];
    }
}

impl<F: num::Float> Index<Dim2> for Vec2<F> {
    type Output = F;
    fn index(&self, dim: Dim2) -> &Self::Output { &self.0[dim] }
}

impl<F: num::Float> IndexMut<Dim2> for Vec2<F> {
    fn index_mut(&mut self, dim: Dim2) -> &mut Self::Output { &mut self.0[dim] }
}

impl<'a, F: num::Float> Add<&'a Vec2<F>> for &'a Vec2<F> {
    type Output = Vec2<F>;
    fn add(self, other: Self) -> Self::Output {
        Self::Output::new(self[Dim2::X] + other[Dim2::X],
                          self[Dim2::Y] + other[Dim2::Y])
    }
}

impl<F: num::Float> Add<Vec2<F>> for &Vec2<F> {
    type Output = Vec2<F>;
    fn add(self, other: Vec2<F>) -> Self::Output { self + &other }
}

impl<F: num::Float> Add<Vec2<F>> for Vec2<F> {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output { &self + &other }
}

impl<F: num::Float> Add<&Vec2<F>> for Vec2<F> {
    type Output = Self;
    fn add(self, other: &Self) -> Self::Output { &self + other }
}

impl<'a, F: num::Float> Sub<&'a Vec2<F>> for &'a Vec2<F> {
    type Output = Vec2<F>;
    fn sub(self, other: Self) -> Self::Output {
        Self::Output::new(self[Dim2::X] - other[Dim2::X],
                          self[Dim2::Y] - other[Dim2::Y])
    }
}

impl<F: num::Float> Sub<Vec2<F>> for &Vec2<F> {
    type Output = Vec2<F>;
    fn sub(self, other: Vec2<F>) -> Self::Output { self - &other }
}

impl<F: num::Float> Sub<Vec2<F>> for Vec2<F> {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output { &self - &other }
}

impl<F: num::Float> Sub<&Vec2<F>> for Vec2<F> {
    type Output = Self;
    fn sub(self, other: &Self) -> Self::Output { &self - other }
}

impl<F: num::Float> Mul<F> for &Vec2<F> {
    type Output = Vec2<F>;
    fn mul(self, factor: F) -> Self::Output {
        Self::Output::new(factor*self[Dim2::X],
                          factor*self[Dim2::Y])
    }
}

impl<F: num::Float> Mul<F> for Vec2<F> {
    type Output = Self;
    fn mul(self, factor: F) -> Self::Output { &self*factor }
}

impl<F: num::Float> Div<F> for &Vec2<F> {
    type Output = Vec2<F>;
    fn div(self, divisor: F) -> Self::Output {
        #![allow(clippy::suspicious_arithmetic_impl)]
        let factor = divisor.recip();
        self*factor
    }
}

impl<F: num::Float> Div<F> for Vec2<F> {
    type Output = Self;
    fn div(self, divisor: F) -> Self::Output { &self/divisor }
}

/// A 3D spatial coordinate.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Point3<F: num::Float>(In3D<F>);

impl<F: num::Float> Point3<F> {
    /// Creates a new 3D point given the three components.
    pub fn new(x: F, y: F, z: F) -> Self { Point3(In3D::new(x, y, z)) }

    /// Creates a new point from the given point, which may have a different component type.
    pub fn from<U: num::Float>(other: &Point3<U>) -> Self {
        Point3::new(F::from(other[X]).unwrap(), F::from(other[Y]).unwrap(), F::from(other[Z]).unwrap())
    }

    /// Constructs a new vector from the point components.
    pub fn to_vec3(&self) -> Vec3<F> {
        Vec3::new(self[X], self[Y], self[Z])
    }

    /// Creates a new 3D point with all components set to zero.
    pub fn origin() -> Self { Self::new(F::zero(), F::zero(), F::zero()) }
}

impl<F: num::Float> Index<Dim3> for Point3<F> {
    type Output = F;
    fn index(&self, dim: Dim3) -> &Self::Output { &self.0[dim] }
}

impl<F: num::Float> IndexMut<Dim3> for Point3<F> {
    fn index_mut(&mut self, dim: Dim3) -> &mut Self::Output { &mut self.0[dim] }
}

impl<'a, F: num::Float> Sub<&'a Self> for &'a Point3<F> {
    type Output = Vec3<F>;
    fn sub(self, other: &Self) -> Self::Output {
        Self::Output::new(self[X] - other[X],
                          self[Y] - other[Y],
                          self[Z] - other[Z])
    }
}

impl<F: num::Float> Sub<Self> for Point3<F> {
    type Output = Vec3<F>;
    fn sub(self, other: Self) -> Self::Output { &self - &other }
}

impl<F: num::Float> Sub<Self> for &Point3<F> {
    type Output = Vec3<F>;
    fn sub(self, other: Self) -> Self::Output { #![allow(clippy::op_ref)] self - &other }
}

impl<F: num::Float> Sub<&Self> for Point3<F> {
    type Output = Vec3<F>;
    fn sub(self, other: &Self) -> Self::Output { &self - other }
}

impl<'a, F: num::Float> Add<&'a Vec3<F>> for &'a Point3<F> {
    type Output = Point3<F>;
    fn add(self, vector: &Vec3<F>) -> Self::Output {
        Self::Output::new(self[X] + vector[X],
                          self[Y] + vector[Y],
                          self[Z] + vector[Z])
    }
}

impl<F: num::Float> Add<Vec3<F>> for Point3<F> {
    type Output = Self;
    fn add(self, vector: Vec3<F>) -> Self::Output { &self + &vector }
}

impl<F: num::Float> Add<Vec3<F>> for &Point3<F> {
    type Output = Point3<F>;
    fn add(self, vector: Vec3<F>) -> Self::Output { self + &vector }
}

impl<'a, F: num::Float> Sub<&'a Vec3<F>> for &'a Point3<F> {
    type Output = Point3<F>;
    fn sub(self, vector: &Vec3<F>) -> Self::Output {
        Self::Output::new(self[X] - vector[X],
                          self[Y] - vector[Y],
                          self[Z] - vector[Z])
    }
}

impl<F: num::Float> Sub<Vec3<F>> for Point3<F> {
    type Output = Self;
    fn sub(self, vector: Vec3<F>) -> Self::Output { &self - &vector }
}

impl<F: num::Float> Sub<Vec3<F>> for &Point3<F> {
    type Output = Point3<F>;
    fn sub(self, vector: Vec3<F>) -> Self::Output { self - &vector }
}

/// A 2D spatial coordinate.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Point2<F: num::Float>(In2D<F>);

impl<F: num::Float> Point2<F> {
    /// Creates a new 2D point given the three components.
    pub fn new(x: F, y: F) -> Self { Point2(In2D::new(x, y)) }

    /// Creates a new point from the given point, which may have a different component type.
    pub fn from<U: num::Float>(other: &Point2<U>) -> Self {
        Point2::new(F::from(other[Dim2::X]).unwrap(), F::from(other[Dim2::Y]).unwrap())
    }

    /// Constructs a new vector from the point components.
    pub fn to_vec2(&self) -> Vec2<F> {
        Vec2::new(self[Dim2::X], self[Dim2::Y])
    }

    /// Creates a new 2D point with all components set to zero.
    pub fn origin() -> Self { Self::new(F::zero(), F::zero()) }
}

impl<F: num::Float> Index<Dim2> for Point2<F> {
    type Output = F;
    fn index(&self, dim: Dim2) -> &Self::Output { &self.0[dim] }
}

impl<F: num::Float> IndexMut<Dim2> for Point2<F> {
    fn index_mut(&mut self, dim: Dim2) -> &mut Self::Output { &mut self.0[dim] }
}

impl<'a, F: num::Float> Sub<&'a Self> for &'a Point2<F> {
    type Output = Vec2<F>;
    fn sub(self, other: &Self) -> Self::Output {
        Self::Output::new(self[Dim2::X] - other[Dim2::X],
                          self[Dim2::Y] - other[Dim2::Y])
    }
}

impl<F: num::Float> Sub<Self> for Point2<F> {
    type Output = Vec2<F>;
    fn sub(self, other: Self) -> Self::Output { &self - &other }
}

impl<F: num::Float> Sub<Self> for &Point2<F> {
    type Output = Vec2<F>;
    fn sub(self, other: Self) -> Self::Output { #![allow(clippy::op_ref)] self - &other }
}

impl<F: num::Float> Sub<&Self> for Point2<F> {
    type Output = Vec2<F>;
    fn sub(self, other: &Self) -> Self::Output { &self - other }
}

impl<'a, F: num::Float> Add<&'a Vec2<F>> for &'a Point2<F> {
    type Output = Point2<F>;
    fn add(self, vector: &Vec2<F>) -> Self::Output {
        Self::Output::new(self[Dim2::X] + vector[Dim2::X],
                          self[Dim2::Y] + vector[Dim2::Y])
    }
}

impl<F: num::Float> Add<Vec2<F>> for Point2<F> {
    type Output = Self;
    fn add(self, vector: Vec2<F>) -> Self::Output { &self + &vector }
}

impl<F: num::Float> Add<Vec2<F>> for &Point2<F> {
    type Output = Point2<F>;
    fn add(self, vector: Vec2<F>) -> Self::Output { self + &vector }
}

impl<'a, F: num::Float> Sub<&'a Vec2<F>> for &'a Point2<F> {
    type Output = Point2<F>;
    fn sub(self, vector: &Vec2<F>) -> Self::Output {
        Self::Output::new(self[Dim2::X] - vector[Dim2::X],
                          self[Dim2::Y] - vector[Dim2::Y])
    }
}

impl<F: num::Float> Sub<Vec2<F>> for Point2<F> {
    type Output = Self;
    fn sub(self, vector: Vec2<F>) -> Self::Output { &self - &vector }
}

impl<F: num::Float> Sub<Vec2<F>> for &Point2<F> {
    type Output = Point2<F>;
    fn sub(self, vector: Vec2<F>) -> Self::Output { self - &vector }
}

/// A 3D index.
#[derive(Debug, Clone, PartialEq)]
pub struct Idx3<I: num::Integer>(In3D<I>);

impl<I: num::Integer> Idx3<I> {
    /// Creates a new 3D index given the three components.
    pub fn new(i: I, j: I, k: I) -> Self { Idx3(In3D::new(i, j, k)) }

    /// Creates a new 3D index from the given index, which may have a different component type.
    pub fn from<U>(other: &Idx3<U>) -> Self
    where I: num::NumCast + Copy,
          U: num::Integer + num::NumCast + Copy
    {
        Idx3::new(I::from(other[X]).unwrap(), I::from(other[Y]).unwrap(), I::from(other[Z]).unwrap())
    }
}

impl<I: num::Integer> Index<Dim3> for Idx3<I> {
    type Output = I;
    fn index(&self, dim: Dim3) -> &Self::Output { &self.0[dim] }
}

impl<I: num::Integer> IndexMut<Dim3> for Idx3<I> {
    fn index_mut(&mut self, dim: Dim3) -> &mut Self::Output { &mut self.0[dim] }
}

/// A 2D index.
#[derive(Debug, Clone, PartialEq)]
pub struct Idx2<I: num::Integer>(In2D<I>);

impl<I: num::Integer> Idx2<I> {
    /// Creates a new 2D index given the three components.
    pub fn new(i: I, j: I) -> Self { Idx2(In2D::new(i, j)) }

    /// Creates a new 2D index from the given index, which may have a different component type.
    pub fn from<U>(other: &Idx2<U>) -> Self
    where I: num::NumCast + Copy,
          U: num::Integer + num::NumCast + Copy
    {
        Idx2::new(I::from(other[Dim2::X]).unwrap(), I::from(other[Dim2::Y]).unwrap())
    }
}

impl<I: num::Integer> Index<Dim2> for Idx2<I> {
    type Output = I;
    fn index(&self, dim: Dim2) -> &Self::Output { &self.0[dim] }
}

impl<I: num::Integer> IndexMut<Dim2> for Idx2<I> {
    fn index_mut(&mut self, dim: Dim2) -> &mut Self::Output { &mut self.0[dim] }
}

/// 3D spatial coordinate arrays.
#[derive(Debug, Clone, Serialize)]
pub struct Coords3<F: num::Float>(In3D<Array1<F>>);

impl<F: num::Float> Coords3<F> {
    /// Creates a new 3D set of coordinates given the component 1D coordinates.
    pub fn new(x: Array1<F>, y: Array1<F>, z: Array1<F>) -> Self {
        Coords3(In3D::new(x, y, z))
    }

    /// Creates a 3D point from the coordinates at the given index.
    pub fn point(&self, idx: usize) -> Point3<F> {
        Point3::new(self[X][idx], self[Y][idx], self[Z][idx])
    }
}

impl<F: num::Float> Index<Dim3> for Coords3<F> {
    type Output = Array1<F>;
    fn index(&self, dim: Dim3) -> &Self::Output { &self.0[dim] }
}

/// 2D spatial coordinate arrays.
#[derive(Debug, Clone, Serialize)]
pub struct Coords2<F: num::Float>(In2D<Array1<F>>);

impl<F: num::Float> Coords2<F> {
    /// Creates a new 2D set of coordinates given the component 1D coordinates.
    pub fn new(x: Array1<F>, y: Array1<F>) -> Self {
        Coords2(In2D::new(x, y))
    }

    /// Creates a 2D point from the coordinates at the given index.
    pub fn point(&self, idx: usize) -> Point2<F> {
        Point2::new(self[Dim2::X][idx], self[Dim2::Y][idx])
    }
}

impl<F: num::Float> Index<Dim2> for Coords2<F> {
    type Output = Array1<F>;
    fn index(&self, dim: Dim2) -> &Self::Output { &self.0[dim] }
}

/// References to 3D spatial coordinate arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordRefs3<'a, F: num::Float>(In3D<&'a Array1<F>>);

impl<'a, F: num::Float> CoordRefs3<'a, F> {
    /// Creates a new 3D set of coordinate references given references to the component arrays.
    pub fn new(x: &'a Array1<F>, y: &'a Array1<F>, z: &'a Array1<F>) -> Self {
        CoordRefs3(In3D::new(x, y, z))
    }

    /// Clones the coordinate references to produce a set of owned coordinate arrays.
    pub fn into_owned(self) -> Coords3<F> {
        Coords3::new(self[X].clone(), self[Y].clone(), self[Z].clone())
    }

    /// Creates a 3D point from the coordinates at the given index.
    pub fn point(&self, idx: usize) -> Point3<F> {
        Point3::new(self[X][idx], self[Y][idx], self[Z][idx])
    }
}

impl<'a, F: num::Float> Index<Dim3> for CoordRefs3<'a, F> {
    type Output = &'a Array1<F>;
    fn index(&self, dim: Dim3) -> &Self::Output { &self.0[dim] }
}

/// References to 2D spatial coordinate arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordRefs2<'a, F: num::Float>(In2D<&'a Array1<F>>);

impl<'a, F: num::Float> CoordRefs2<'a, F> {
    /// Creates a new 2D set of coordinate references given references to the component arrays.
    pub fn new(x: &'a Array1<F>, y: &'a Array1<F>) -> Self {
        CoordRefs2(In2D::new(x, y))
    }

    /// Clones the coordinate references to produce a set of owned coordinate arrays.
    pub fn into_owned(self) -> Coords2<F> {
        Coords2::new(self[Dim2::X].clone(), self[Dim2::Y].clone())
    }

    /// Creates a 2D point from the coordinates at the given index.
    pub fn point(&self, idx: usize) -> Point2<F> {
        Point2::new(self[Dim2::X][idx], self[Dim2::Y][idx])
    }
}

impl<'a, F: num::Float> Index<Dim2> for CoordRefs2<'a, F> {
    type Output = &'a Array1<F>;
    fn index(&self, dim: Dim2) -> &Self::Output { &self.0[dim] }
}