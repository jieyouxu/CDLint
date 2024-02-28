// Data
mod vanilla_enemy_descriptors;

pub(crate) use vanilla_enemy_descriptors::VANILLA_ENEMY_DESCRIPTORS;

// Late lints
mod ambiguous_enemy_pool_add_remove;
mod cyclic_enemy_descriptor_references;
mod empty_cd_name;
mod min_larger_than_max;
mod undefined_enemy_descriptors;
mod unused_custom_enemy_descriptors;

pub(crate) use ambiguous_enemy_pool_add_remove::*;
pub(crate) use cyclic_enemy_descriptor_references::*;
pub(crate) use empty_cd_name::*;
pub(crate) use min_larger_than_max::*;
pub(crate) use undefined_enemy_descriptors::*;
pub(crate) use unused_custom_enemy_descriptors::*;
