use confique::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
pub struct Config {
    /// Add your custom enemy descriptors e.g. `ED_EnemyName` to this list, so that lints such as
    /// `undefined-enemy-descriptors` can augment its "defined" enemy descriptors library.
    #[config(default = [])]
    pub extra_enemy_descriptors: Vec<String>,

    /// Would you like `cyclic_enemy_descriptor_references` lint to generate a graphviz graph of
    /// the "based-on" relationships between Enemy Descriptors? Note that if this option is
    /// enabled, the graphviz `dot` command line must be installed:
    /// <https://graphviz.org/download/>.
    #[config(default = false)]
    pub generate_cyclic_reference_graph: bool,
}
