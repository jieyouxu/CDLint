# Change Log

<!-- next-header -->
## [Unreleased] - ReleaseDate

- Generate and load config at `$CWD`.

## [0.1.9] - 2024-02-14

- Fix cycle string for `cyclic-enemy-descriptor-references` lint.

## [0.1.8] - 2024-02-14

- Implement `cyclic-enemy-descriptor-references` lint, which detects if
  `Base` attributes in custom Enemy Descriptor definition forms cycles with
  each other.

## [0.1.7] - 2024-02-11

- Bump suggestion max edit distance to 7
- Implement `ambiguous-enemy-pool-add-remove` lint, which detects if you add
  and remove the same Enemy Descriptor from an enemy pool

## [0.1.6] - 2024-02-11

- Add missing `ED_Spider_Stalker` as a vanilla Enemy Descriptor

## [0.1.5] - 2024-02-11

- Lint defunct `UseSpawnRarityModifiers` in Enemy Descriptors
- Fix `unused-enemy-descriptors` lint to account for custom Enemy Descriptors 
  with generated dummy "Base" members

## [0.1.4] - 2024-02-11

- Fix handling of single numbers and strings

## [0.1.3] - 2024-02-11

- Implement `min > max` lint
- Fix handling of single or array of items
- Fix handling of missing mandatory attributes

## [0.1.2] - 2024-02-11

- Setup CHANGELOG.md automation
- Improve CI via cargo-release

## [0.1.1] - 2024-02-11

- Initial test release

<!-- next-url -->
[Unreleased]: https://github.com/jieyouxu/CDLint/compare/v0.1.9...HEAD
[0.1.9]: https://github.com/jieyouxu/CDLint/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/jieyouxu/CDLint/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/jieyouxu/CDLint/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/jieyouxu/CDLint/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/jieyouxu/CDLint/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/jieyouxu/CDLint/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/jieyouxu/CDLint/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/jieyouxu/CDLint/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/jieyouxu/CDLint/compare/v0.1.0...v0.1.1
