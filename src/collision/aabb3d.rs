use bevy::prelude::{IVec3, Vec3};

/// A 3D axis-aligned bounding box defined by minimum and maximum extents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb3d {
    /// Minimum corner of the bounding box.
    pub min: Vec3,
    /// Maximum corner of the bounding box.
    pub max: Vec3,
}

impl Aabb3d {
    /// Creates a new AABB from a center point and half extents.
    ///
    /// # Arguments
    /// * `center` - World-space center of the box.
    /// * `half_extents` - Positive distances from center to edges along each axis.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::collision::Aabb3d;
    /// use bevy::prelude::Vec3;
    ///
    /// let aabb = Aabb3d::from_center_half_extents(Vec3::new(1.0, 2.0, 3.0), Vec3::splat(0.5));
    /// assert_eq!(aabb.min, Vec3::new(0.5, 1.5, 2.5));
    /// assert_eq!(aabb.max, Vec3::new(1.5, 2.5, 3.5));
    /// ```
    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents.abs(),
            max: center + half_extents.abs(),
        }
    }

    /// Checks if this AABB intersects another AABB.
    ///
    /// # Arguments
    /// * `other` - The other AABB to test against.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::collision::Aabb3d;
    /// use bevy::prelude::Vec3;
    ///
    /// let a = Aabb3d::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.5));
    /// let b = Aabb3d::from_center_half_extents(Vec3::new(0.8, 0.0, 0.0), Vec3::splat(0.5));
    /// assert!(a.intersects(&b));
    /// ```
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }

    /// Returns the integer voxel coordinates covered by this AABB.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::collision::Aabb3d;
    /// use bevy::prelude::{IVec3, Vec3};
    ///
    /// let aabb = Aabb3d::from_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(0.4));
    /// let (min_voxel, max_voxel) = aabb.voxel_bounds();
    /// assert_eq!(min_voxel, IVec3::new(0, 0, 0));
    /// assert_eq!(max_voxel, IVec3::new(0, 0, 0));
    /// ```
    pub fn voxel_bounds(&self) -> (IVec3, IVec3) {
        let min_voxel = self.min.floor().as_ivec3();
        let max_voxel = (self.max - Vec3::splat(1e-5)).floor().as_ivec3();
        (min_voxel, max_voxel)
    }

    /// Checks whether this AABB intersects the unit-cube block at `voxel_pos`.
    ///
    /// # Arguments
    /// * `voxel_pos` - World integer coordinate of the unit block `[pos, pos + 1)`.
    ///
    /// # Example
    /// ```rust
    /// use block_game_2::collision::Aabb3d;
    /// use bevy::prelude::{IVec3, Vec3};
    ///
    /// let aabb = Aabb3d::from_center_half_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(0.4));
    /// assert!(aabb.intersects_block(IVec3::new(0, 0, 0)));
    /// assert!(!aabb.intersects_block(IVec3::new(1, 0, 0)));
    /// ```
    pub fn intersects_block(&self, voxel_pos: IVec3) -> bool {
        let block_min = voxel_pos.as_vec3();
        let block_max = block_min + Vec3::ONE;
        self.min.x < block_max.x
            && self.max.x > block_min.x
            && self.min.y < block_max.y
            && self.max.y > block_min.y
            && self.min.z < block_max.z
            && self.max.z > block_min.z
    }
}
