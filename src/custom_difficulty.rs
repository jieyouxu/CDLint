use std::collections::BTreeMap;

use chumsky::span::SimpleSpan;
use serde::Deserialize;

use crate::Spanned;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ArrayOrSingleItem<T> {
    Array(Vec<T>),
    SingleItem(T),
}

impl<T: Default> Default for ArrayOrSingleItem<T> {
    fn default() -> Self {
        Self::SingleItem(Default::default())
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Range<T> {
    pub min: T,
    pub max: T,
}

impl<T: Default> Default for Range<T> {
    fn default() -> Self {
        Self {
            min: Default::default(),
            max: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WeightedRange<T> {
    pub weight: f32,
    pub range: Range<T>,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnemyPool {
    #[serde(default)]
    pub clear: bool,
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

impl Default for EnemyPool {
    fn default() -> Self {
        Self {
            clear: Default::default(),
            add: Default::default(),
            remove: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EscortMule {
    /// The damage taken from players.
    pub friendly_fire_modifier: f32,
    /// The damage taken from neutral damage sources.
    pub neutral_damage_modifier: f32,
    /// The damage taken from big hits.
    pub big_hit_damage_modifier: f32,
    /// The damage threshold for a hit to be considered a "big hit" and get affected by the
    /// `BigHitDamageModifier`.
    pub big_hit_damage_reduction_threshold: f32,
}

impl Default for EscortMule {
    fn default() -> Self {
        EscortMule {
            friendly_fire_modifier: 0.1,
            neutral_damage_modifier: 0.1,
            big_hit_damage_modifier: 0.5,
            big_hit_damage_reduction_threshold: 0.0,
        }
    }
}

#[derive(Debug, PartialEq, Default, Deserialize)]
pub struct PawnStats(pub BTreeMap<String, f32>);

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EnemyDescriptor {
    /// The EnemyDescriptor to copy values from. Required if defining a new EnemyDescriptor.
    #[serde(default)]
    pub base: String,
    /// The maximum distance enemies can spawn from the center of spawn point in centimeters.
    #[serde(default)]
    pub spawn_spread: f32,
    #[serde(default)]
    pub ideal_spawn_size: usize,
    /// Whether this descriptor can spawn in constant pressure waves (preset in point extraction and
    /// during the repair phase on refinery).
    #[serde(default)]
    pub can_be_used_for_constant_pressure: bool,
    /// Whether this descriptor can spawn in encounters.
    #[serde(default)]
    pub can_be_used_in_encounters: bool,
    /// The difficulty cost to spawn each individual enemy. The exact interaction with
    /// SpawnAmountModifier is currently unknown.
    #[serde(default)]
    pub difficulty_rating: f32,
    #[serde(default)]
    pub min_spawn_count: usize,
    #[serde(default)]
    pub max_spawn_count: usize,
    #[serde(default)]
    pub rarity: usize,
    #[serde(default)]
    pub spawn_amount_modifier: usize,
    /// Whether the enemy should be turned into an elite.
    #[serde(default)]
    pub elite: bool,
    /// How large the enemy is.
    #[serde(default)]
    pub scale: f32,
    /// How fast the enemy moves relative to everything else.
    #[serde(default)]
    pub time_dilation: f32,
    #[serde(default)]
    pub pawn_stats: PawnStats,
}

impl<T: Default> Default for Spanned<T> {
    fn default() -> Self {
        Self {
            span: SimpleSpan::new(usize::MAX, usize::MAX),
            val: Default::default(),
        }
    }
}

//FIXME: make newtypes for fields that need a default value.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CustomDifficulty {
    /// The difficulty name
    #[serde(default)]
    pub name: Spanned<String>,
    #[serde(default)]
    pub description: Spanned<String>,
    /// The maximum number of critters (maggots, lootbugs, silica harvesters, etc.) allowed to exist
    /// at once.
    #[serde(default)]
    pub max_active_critters: Spanned<ArrayOrSingleItem<usize>>,
    /// The maximum number of swarmers allowed to exist at once.
    #[serde(default)]
    pub max_active_swarmers: Spanned<ArrayOrSingleItem<usize>>,
    /// The maximum number of enemies allowed to exist at once.
    #[serde(default)]
    pub max_active_enemies: Spanned<ArrayOrSingleItem<usize>>,
    /// The amount of nitra required to call a resupply pod. This can be lowered to relieve ammo
    /// pressure on more demanding difficulties.
    #[serde(default)]
    pub resupply_cost: Spanned<ArrayOrSingleItem<f32>>,
    /// The amount of nitra initially in the team depository. This can be used to counter bad RNG at
    /// the start of missions that would otherwise making getting the first resupply quite
    /// difficult.
    #[serde(default)]
    pub starting_nitra: Spanned<ArrayOrSingleItem<usize>>,
    /// The damage resistance for ExtraLarge enemies for the corresponding player count.
    #[serde(default)]
    pub extra_large_enemy_damage_resistance: Spanned<ArrayOrSingleItem<f32>>,
    /// The damage resistance for ExtraLargeB enemies for the corresponding player count.
    #[serde(default)]
    pub extra_large_enemy_damage_resistance_b: Spanned<ArrayOrSingleItem<f32>>,
    /// The damage resistance for ExtraLargeC enemies for the corresponding player count.
    #[serde(default)]
    pub extra_large_enemy_damage_resistance_c: Spanned<ArrayOrSingleItem<f32>>,
    /// The damage resistance for ExtraLargeD enemies for the corresponding player count.
    #[serde(default)]
    pub extra_large_enemy_damage_resistance_d: Spanned<ArrayOrSingleItem<f32>>,
    /// The damage resistance for enemies for the corresponding player count.
    #[serde(default)]
    pub enemy_damage_resistance: Spanned<ArrayOrSingleItem<f32>>,
    /// The damage resistance for small enemies for the corresponding player count.
    #[serde(default)]
    pub small_enemy_damage_resistance: Spanned<ArrayOrSingleItem<f32>>,
    /// The amount of damage done to players by enemies for the corresponding player count.
    #[serde(default)]
    pub enemy_damage_modifier: Spanned<ArrayOrSingleItem<f32>>,
    /// The number of enemies spawned for nearly all wave and encounter types for the corresponding
    /// player count.
    #[serde(default)]
    pub enemy_count_modifier: Spanned<ArrayOrSingleItem<f32>>,
    /// An array of weighted bins used to calculate the difficulty of encounter enemies when spawned
    /// (enemies spawned inside rooms when approached by the player the first time).
    #[serde(default)]
    pub encounter_difficulty: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate the difficulty of stationary enemies (spitball
    /// infectors, brood nexuses, leeches, and breeders).
    #[serde(default)]
    pub stationary_difficulty: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate time (in seconds) between timed announced waves
    /// (present in mining, point extraction, and refinery mission types).
    #[serde(default)]
    pub enemy_wave_interval: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate time (in seconds) between timed unannounced
    /// waves (present in mining, refinery, egg, elimination, salvage, escort?, and industrial
    /// sabotage mission types).
    #[serde(default)]
    pub enemy_normal_wave_interval: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate difficulty of normal waves.
    #[serde(default)]
    pub enemy_normal_wave_difficulty: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate diversity (number of unique enemy types) spawned
    /// in a wave.
    #[serde(default)]
    pub enemy_diversity: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate diversity (number of unique enemy types) spawned
    /// in a room.
    #[serde(default)]
    pub stationary_enemy_diversity: Spanned<Vec<WeightedRange<usize>>>,
    /// An array of weighted bins used to calculate percentage of grunts and mactera to be promoted
    /// to veteran variants.
    #[serde(default)]
    pub veteran_normal: Spanned<Vec<WeightedRange<f32>>>,
    /// An array of weighted bins used to calculate percentage of praetorians to be promoted to
    /// oppressors.
    #[serde(default)]
    pub veteran_large: Spanned<Vec<WeightedRange<f32>>>,
    /// The number of disruptive enemies to fill the enemy pool with at the start of the mission.
    /// Has no effect if changed mid mission.
    #[serde(default)]
    pub disruptive_enemy_pool_count: Spanned<Range<usize>>,
    /// The size of the enemy pool. Enemies will be selected and added to the enemy pool until it is
    /// full in the following order: common enemies, disruptive enemies, then special enemies.
    #[serde(default)]
    pub min_pool_size: Spanned<Range<usize>>,
    /// The maximum number of elite enemies allowed to exist at once.
    #[serde(default)]
    pub max_active_elites: Spanned<usize>,
    /// The amount of damage environmental sources of damage do to players. Does not seem to have
    /// any effect.
    #[serde(default)]
    pub environmental_damage_modifier: Spanned<f32>,
    /// How quickly the constant pressure waves on point extraction scale as time goes on. Does not
    /// seem to do anything when increased higher than 1.
    #[serde(default)]
    pub point_extraction_scalar: Spanned<f32>,
    /// The hazard bonus reward for the difficulty.
    #[serde(default)]
    pub hazard_bonus: Spanned<f32>,
    /// The amount of damage done to other players.
    #[serde(default)]
    pub friendly_fire_modifier: Spanned<f32>,
    /// How long before the first wave starts in units of wave interval.
    #[serde(default)]
    pub wave_start_delay_scale: Spanned<f32>,
    /// The movement speed of most enemies.
    #[serde(default)]
    pub speed_modifier: Spanned<f32>,
    /// The cooldown between enemy attacks.
    #[serde(default)]
    pub attack_cooldown_modifier: Spanned<f32>,
    /// The speed of non-arcing enemy projectiles.
    #[serde(default)]
    pub projectile_speed_modifier: Spanned<f32>,
    /// The percentage of full health to regenerate to over time.
    #[serde(default)]
    pub health_regeneration_max: Spanned<f32>,
    /// The percentage of health to get back upon being revived by another player (does not apply to
    /// a revive from bosco).
    #[serde(default)]
    pub revive_health_ratio: Spanned<f32>,
    /// The cooldown in seconds between spawning elite enemies.
    #[serde(default)]
    pub elite_cooldown: Spanned<usize>,
    /// A map of `EnemyDescriptor` names and definitions. It will override fields on existing
    /// `EnemyDescriptor`s or create a new `EnemyDescriptor` if one does not already exist. This can
    /// be used to define new `EnemyDescriptor`s that can be added to pools or modify (or completely
    /// replace) existing `EnemyDescriptor`s.
    #[serde(default)]
    pub enemy_descriptors: Spanned<BTreeMap<String, EnemyDescriptor>>,
    /// The enemy pool which is what the game pulls `EnemyDescriptor`s from when attempting to spawn
    /// enemies. This pool is built by pulling enemies from the `CommonEnemies`,
    /// `DisruptiveEnemies`, and `SpecialEnemies` pools upon mission start. It is recommended to not
    /// modify this pool directly and instead modify the pools this pulls from.
    #[serde(default)]
    pub enemy_pool: Spanned<EnemyPool>,
    /// The common enemy pool which is added to the enemy pool before anything else.
    #[serde(default)]
    pub common_enemies: Spanned<EnemyPool>,
    /// The disruptive enemy pool which is added to the enemy pool after common enemies. The
    /// quantity depends on what value is rolled from `DisruptiveEnemyPoolCount`.
    #[serde(default)]
    pub disruptive_enemies: Spanned<EnemyPool>,
    /// The special enemy pool which is added to the enemy pool after disruptive enemies.
    #[serde(default)]
    pub special_enemies: Spanned<EnemyPool>,
    /// The stationary enemy pool.
    #[serde(default)]
    pub stationary_enemies: Spanned<EnemyPool>,
    /// An array of season events that can spawn. Can be used to disable events such as the
    /// Prospector which can be quite disruptive if encountered on high enemy count missions.
    #[serde(default)]
    pub seasonal_events: Spanned<Vec<String>>,
    /// The escort mule damage resistance properties.
    #[serde(default)]
    pub escort_mule: Spanned<EscortMule>,
}
