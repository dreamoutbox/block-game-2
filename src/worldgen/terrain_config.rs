/// Configuration options for procedural terrain generation.
#[derive(Debug, Clone)]
pub struct TerrainConfig {
    /// Random seed determining terrain topology.
    pub seed: u32,
    /// Base sea-level or baseline elevation in block units.
    pub base_height: f64,
    /// Horizontal frequency scale for sampling noise.
    pub scale: f64,
    /// Vertical amplitude multiplier for elevation variation.
    pub amplitude: f64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            base_height: 16.0,
            scale: 0.02,
            amplitude: 14.0,
        }
    }
}
