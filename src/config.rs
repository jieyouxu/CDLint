use confique::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
pub struct Config {
    /// Add your custom enemy descriptors e.g. `ED_EnemyName` to this list, so that lints such as
    /// `undefined-enemy-descriptors` can augment its "defined" enemy descriptors library.
    #[config(default = [])]
    pub extra_enemy_descriptors: Vec<String>,
}
