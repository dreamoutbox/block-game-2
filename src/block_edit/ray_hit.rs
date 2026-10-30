use bevy::prelude::IVec3;

/// Result of a voxel raycast: the solid block hit and the face the ray entered through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RayHit {
    /// World coordinate of the solid block that was hit.
    pub block: IVec3,
    /// Outward unit normal of the entered face; `block + normal` is the placement cell.
    pub normal: IVec3,
}
