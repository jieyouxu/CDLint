// Data
mod vanilla_enemy_descriptors;

pub(crate) use vanilla_enemy_descriptors::VANILLA_ENEMY_DESCRIPTORS;

// Late lints
mod empty_cd_name;
mod min_larger_than_max;
mod undefined_enemy_descriptors;

pub(crate) use empty_cd_name::*;
pub(crate) use min_larger_than_max::*;
pub(crate) use undefined_enemy_descriptors::*;
