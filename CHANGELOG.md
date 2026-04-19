# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-04-19


### Features
- **(hippo)** Show 😴 Rest day tooltip on empty days ([75b4bf3](https://github.com/chagui/hippoclaudus/commit/75b4bf3e4863debd7151ef2d741a3c3ef3f899d8))


### Bug Fixes
- **(analytics)** Drop daily buckets outside the window ([5bff54d](https://github.com/chagui/hippoclaudus/commit/5bff54dca3f8045071b854192bec300ad81ce043))
- **(hippo)** Align daily-chart rule and tooltip with plot area ([a98bcad](https://github.com/chagui/hippoclaudus/commit/a98bcad0655ab2b75f85a6c72f197caf598a88ca))

## [0.3.0] - 2026-04-19


### Features
- **(analytics)** Stack daily bars by model, show days in durations, reorder sections ([959f9a5](https://github.com/chagui/hippoclaudus/commit/959f9a53c3eeedfd67f127968941c3b6a3afdd03))
- **(hippo)** Prefix weekday abbreviation in daily-chart tooltip ([5e04d0d](https://github.com/chagui/hippoclaudus/commit/5e04d0d388bbcf4e659de13450dd5f1d846a3607))
- **(hippo)** Show date tooltip on daily chart hover ([73e3bea](https://github.com/chagui/hippoclaudus/commit/73e3beaeb89b3b088e74fdc650788383d369f30f))
- **(hippo)** Add Session Analytics window ([e84cc98](https://github.com/chagui/hippoclaudus/commit/e84cc9801fff8904074c1ab3cbd1ac6c726ae679))
- **(analytics)** Add hpc analytics subcommand ([9ebcb36](https://github.com/chagui/hippoclaudus/commit/9ebcb36fb655da70c616892c76c33c3c684b6095))
- **(analytics)** Add pricing and percentile modules ([6af9b2b](https://github.com/chagui/hippoclaudus/commit/6af9b2b9ac27d097f4f0557ffe4ff63a26537439))
- **(hippo)** Tint expanded Inactive Sessions block for visual grouping ([285be57](https://github.com/chagui/hippoclaudus/commit/285be57d7d92a9e6cb75a4f4358e45f0384f072e))
- **(release)** Generate changelog and GitHub Release on tag push ([73dd7dd](https://github.com/chagui/hippoclaudus/commit/73dd7dd4697d63c1ad3443ee2a7492d068e834e1))
- **(hippo)** Add Inactive Sessions section to menu bar ([d529e4c](https://github.com/chagui/hippoclaudus/commit/d529e4cf38433896ea350e57a4b0250d8edc410b))
- **(config)** Make active session window configurable ([3ffcbe6](https://github.com/chagui/hippoclaudus/commit/3ffcbe6be516cca73f8f70a7569f903fbd56cf7f))


### Bug Fixes
- **(hippo)** Hide segmented picker labels in Analytics window ([5664fb5](https://github.com/chagui/hippoclaudus/commit/5664fb503074eaf845445d7da509a88987e969b1))


### Refactor
- **(cost)** Move pricing to Rust as single source of truth ([e4b3fb6](https://github.com/chagui/hippoclaudus/commit/e4b3fb65c3d80ee433bd5223d12f145beae6a124))
- **(hippo)** Move Inactive Sessions inside Tools, below Tags ([d7d8c19](https://github.com/chagui/hippoclaudus/commit/d7d8c19607432d90f6c011358a5925b14c09f335))

## [0.2.0] - 2026-04-17


### Features
- **(config)** Add vault_name to remove hard-coded Obsidian vault URLs ([bce2d59](https://github.com/chagui/hippoclaudus/commit/bce2d598aa2f34b6acae26fc3c342a4a43e57ae4))
- Add save-note and save-knowledge Claude Code skills ([e32bc58](https://github.com/chagui/hippoclaudus/commit/e32bc5830d0773bf98c613fe45572e8dca53bd5d))
- **(hippo)** Add About window and embed Info.plist ([834ce18](https://github.com/chagui/hippoclaudus/commit/834ce18155f7688e65402da9eb6e6cb293d5e5ab))
- Add worktree detection and flatten UI layout ([43ddbf3](https://github.com/chagui/hippoclaudus/commit/43ddbf37d149845459dad4d66b88a9b3a3f0b5c7))


### Bug Fixes
- **(vault)** Log when vault path is missing instead of silently returning 0 ([f665b8f](https://github.com/chagui/hippoclaudus/commit/f665b8fb89a2b2968c3beaeaf11d71c12cae301d))
- **(ci)** Exclude untestable files from coverage gate ([9e44a69](https://github.com/chagui/hippoclaudus/commit/9e44a6906a309dc51a6ce9b419e1c249dc736a07))
- Resolve CI failures and add local CI script ([ddc6848](https://github.com/chagui/hippoclaudus/commit/ddc6848e9a5f7a37d16683c296cb2c80c1194e78))
- **(swift)** Cancellable sync, error visibility, and parallel tag refresh ([16d091d](https://github.com/chagui/hippoclaudus/commit/16d091dd18a89d782a2e87e21209f7cacc7fa6ce))


### Refactor
- **(hippo)** Read app version from Info.plist instead of hard-coding ([a538b7f](https://github.com/chagui/hippoclaudus/commit/a538b7fcf9ed65f75f723bb3906e23fc85e05d13))
- Rename project from ClaudePulse to Hippoclaudus ([abfad33](https://github.com/chagui/hippoclaudus/commit/abfad33aaf559bf12e5ad608fc2943ec80f480e5))


