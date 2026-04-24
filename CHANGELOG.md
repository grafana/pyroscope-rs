# Changelog

## [3.0.0](https://github.com/grafana/pyroscope-rs/compare/lib-2.0.2...lib-3.0.0) (2026-04-24)


### ⚠ BREAKING CHANGES

* Memory profiling for Rust using jemalloc crate ([#378](https://github.com/grafana/pyroscope-rs/issues/378))
* replace reports Vec on ReportBatch with ReportData enum ([#447](https://github.com/grafana/pyroscope-rs/issues/447))

### Features

* add basic auth ([#101](https://github.com/grafana/pyroscope-rs/issues/101)) ([fc5c0f1](https://github.com/grafana/pyroscope-rs/commit/fc5c0f1eb7cea33f371f986a2af3a38ef373050a))
* add kindasafe crate for safe memory reading ([#394](https://github.com/grafana/pyroscope-rs/issues/394)) ([950d2d9](https://github.com/grafana/pyroscope-rs/commit/950d2d908a4fda525c4ba0dfc50119cd22835cbe))
* add pysignalprof — signal-based CPU profiler for CPython 3.14 ([#417](https://github.com/grafana/pyroscope-rs/issues/417)) ([0976d99](https://github.com/grafana/pyroscope-rs/commit/0976d999e4a50c14459b8a0b39e72434be6d4bfc))
* add ReportBatch type for multi-profile support ([#437](https://github.com/grafana/pyroscope-rs/issues/437)) ([979a3b2](https://github.com/grafana/pyroscope-rs/commit/979a3b2805b14069d42aa50e874d815f3f865d19))
* bump py-spy to support python 3.12 and 3.13 ([#181](https://github.com/grafana/pyroscope-rs/issues/181)) ([21c562e](https://github.com/grafana/pyroscope-rs/commit/21c562e163bcb5d276ef1f0e687a15ab5877aa29))
* **kindasafe:** add aarch64 support for safe memory reading ([#421](https://github.com/grafana/pyroscope-rs/issues/421)) ([6573284](https://github.com/grafana/pyroscope-rs/commit/6573284e7a81aa58c340a6e13541512cf8ff0080))
* **kindasafe:** add macOS support for x86_64 and aarch64 ([#423](https://github.com/grafana/pyroscope-rs/issues/423)) ([b5b4cb4](https://github.com/grafana/pyroscope-rs/commit/b5b4cb4dd26b63bebccf38e815694e37ac559e0f))
* **kindasafe:** prepare kindasafe crates for crates.io publishing with trusted publishing ([#434](https://github.com/grafana/pyroscope-rs/issues/434)) ([17bd1b9](https://github.com/grafana/pyroscope-rs/commit/17bd1b98d1b242d64a10d8f71ba4a24e4152b418))
* Memory profiling for Rust using jemalloc crate ([#378](https://github.com/grafana/pyroscope-rs/issues/378)) ([3a11f1e](https://github.com/grafana/pyroscope-rs/commit/3a11f1e2e4a3fb2c832f1ef0c98a5942bea6b622))
* rails autoinstrumentation ([1846a60](https://github.com/grafana/pyroscope-rs/commit/1846a601cb2c891221feb9dbb8cbee02414ec3f0))
* rails autoinstrumentation ([c81055f](https://github.com/grafana/pyroscope-rs/commit/c81055fa77ca845baf77b5636a9ca5cd993cf919))
* rails autoinstrumentation ([#62](https://github.com/grafana/pyroscope-rs/issues/62)) ([1846a60](https://github.com/grafana/pyroscope-rs/commit/1846a601cb2c891221feb9dbb8cbee02414ec3f0))
* replace reports Vec on ReportBatch with ReportData enum ([#447](https://github.com/grafana/pyroscope-rs/issues/447)) ([a773660](https://github.com/grafana/pyroscope-rs/commit/a773660952b33accaa441ef08e8acdd42f2c37f7))
* split kindasafe into no_std read crate and kindasafe_init ([#420](https://github.com/grafana/pyroscope-rs/issues/420)) ([5975b08](https://github.com/grafana/pyroscope-rs/commit/5975b084ce2025887d5d9d79626998b50a09cad3))
* upstream gzip compression ([#55](https://github.com/grafana/pyroscope-rs/issues/55)) ([cdf23fd](https://github.com/grafana/pyroscope-rs/commit/cdf23fd049f18b49c1a80a051564dfde942fcf11))
* use prebuilt OpenSSL 3.5.5 instead of vendored openssl-src crate ([#422](https://github.com/grafana/pyroscope-rs/issues/422)) ([7bf43b0](https://github.com/grafana/pyroscope-rs/commit/7bf43b0c6a824c378358c6cd152f29919fe65ce5))
* x-scope-orgid header ([#97](https://github.com/grafana/pyroscope-rs/issues/97)) ([74dce71](https://github.com/grafana/pyroscope-rs/commit/74dce71b41f7ac5b20ec3c2fc8316ffceba795e0))


### Bug Fixes

* **actions:** change label name ([89119a9](https://github.com/grafana/pyroscope-rs/commit/89119a9c79e5ff26f9fa54309fa203acbe4924d3))
* **actions:** fix ruby paths for rbspy lib ([f5d74ce](https://github.com/grafana/pyroscope-rs/commit/f5d74ce2b7e55374294486d754a974bbfdea1aed))
* **actions:** remove github arm64 workflow ([b0ea761](https://github.com/grafana/pyroscope-rs/commit/b0ea761e6fe51a870c433cfad6601367644fd5f8))
* Add musllinux (Alpine Linux) support for Python wheels ([#358](https://github.com/grafana/pyroscope-rs/issues/358)) ([d896893](https://github.com/grafana/pyroscope-rs/commit/d896893da8cfbfbb0d31df205d0c4716f9fdd035))
* append ingest path correctly ([#100](https://github.com/grafana/pyroscope-rs/issues/100)) ([a43f81d](https://github.com/grafana/pyroscope-rs/commit/a43f81de2fd58d5947e34f7eda9a77e6bc58d045))
* build ruby and python in docker ([#159](https://github.com/grafana/pyroscope-rs/issues/159)) ([df9fc0b](https://github.com/grafana/pyroscope-rs/commit/df9fc0b6b656aad54c5b1e97da6843bd2872c910))
* bump pyspy for 3.11 support ([#69](https://github.com/grafana/pyroscope-rs/issues/69)) ([4144b8a](https://github.com/grafana/pyroscope-rs/commit/4144b8aaf855a75fa3ae23acfe4539b33555acb9))
* bump rbspy to fix ruby 3.3.8, 3.4 ([#215](https://github.com/grafana/pyroscope-rs/issues/215)) ([603f822](https://github.com/grafana/pyroscope-rs/commit/603f8221cc18a6dcdbe73d74da0c574ba698f90b))
* change to assert_ne macro ([#160](https://github.com/grafana/pyroscope-rs/issues/160)) ([3e42ad6](https://github.com/grafana/pyroscope-rs/commit/3e42ad62fae34e07a9a6d1185e16867730b2fabd))
* **ci:** add cargo-workspace plugin to release-please config ([#455](https://github.com/grafana/pyroscope-rs/issues/455)) ([6a81280](https://github.com/grafana/pyroscope-rs/commit/6a81280cd8808237fc6f9d536fbc20e1a95e8e93))
* **ci:** add version to pysignalprof dep to fix release-please crash ([#452](https://github.com/grafana/pyroscope-rs/issues/452)) ([fc519f6](https://github.com/grafana/pyroscope-rs/commit/fc519f6c816e1c79f54e492fa870e7ba6d0d122d))
* **ci:** bump Rust toolchain to 1.88.0 for naked_functions stabilization ([#419](https://github.com/grafana/pyroscope-rs/issues/419)) ([37bd11f](https://github.com/grafana/pyroscope-rs/commit/37bd11f3d8788b5747b4a6ee7104b4a1cdb9c16c))
* **ci:** configure release-please to update Cargo.lock ([#454](https://github.com/grafana/pyroscope-rs/issues/454)) ([218900e](https://github.com/grafana/pyroscope-rs/commit/218900e255405b6cc59242a2344c27acb9c60d9a))
* **ci:** make job keys and display names unique across FFI workflows ([#435](https://github.com/grafana/pyroscope-rs/issues/435)) ([2e80296](https://github.com/grafana/pyroscope-rs/commit/2e80296ef93980bf07dd8e47f41a033c838fe8c3))
* **ci:** pin artifact actions version ([665d1a4](https://github.com/grafana/pyroscope-rs/commit/665d1a4ea2a3e98daffb23b4c6807d59a60f63a3))
* **ci:** pin artifact actions version ([ca93365](https://github.com/grafana/pyroscope-rs/commit/ca933659be800078896154a66d2fd782f7fee6d8))
* **ci:** remove invalid cargo-workspace extra-files entries ([#456](https://github.com/grafana/pyroscope-rs/issues/456)) ([4f5cb0b](https://github.com/grafana/pyroscope-rs/commit/4f5cb0b0f8dc2abf7d090c9e8dc208de4f6b8146))
* **ci:** remove kindasafe_init extra-file to fix dependency cycle ([#457](https://github.com/grafana/pyroscope-rs/issues/457)) ([4ca3d0c](https://github.com/grafana/pyroscope-rs/commit/4ca3d0c0c7b44ebbff07d0ab758290e53d049d5c))
* **ci:** remove release-please entirely ([#461](https://github.com/grafana/pyroscope-rs/issues/461)) ([e83c23b](https://github.com/grafana/pyroscope-rs/commit/e83c23b1858ba17a941ace173e815c41597603b7))
* **ci:** swtich arm runners ([#185](https://github.com/grafana/pyroscope-rs/issues/185)) ([d1d2994](https://github.com/grafana/pyroscope-rs/commit/d1d29940fdbf0801042c90bae55f1c34a52265fb))
* **ci:** use valid extra-files type for release-please ([#448](https://github.com/grafana/pyroscope-rs/issues/448)) ([0933fad](https://github.com/grafana/pyroscope-rs/commit/0933fadeba7045325c6bc8afda8927961dba0a54))
* cleanup previous build data for arm runner ([ff17111](https://github.com/grafana/pyroscope-rs/commit/ff171117d097bbed5a19f70cbdb93433be1f5d85))
* **cli:** exec command args ([#130](https://github.com/grafana/pyroscope-rs/issues/130)) ([ad88f50](https://github.com/grafana/pyroscope-rs/commit/ad88f502fa26d578aa712b19ef7966c46f7d33bc))
* **deps:** update rust crate object to 0.38 ([#430](https://github.com/grafana/pyroscope-rs/issues/430)) ([d9514e4](https://github.com/grafana/pyroscope-rs/commit/d9514e490855ff3b99f86ab00643a4397c3ff405))
* **deps:** update rust crate rbspy to 0.42 ([#290](https://github.com/grafana/pyroscope-rs/issues/290)) ([d2605e5](https://github.com/grafana/pyroscope-rs/commit/d2605e518f80d6462cabb37743bae1ef4a3e09c4))
* do not create http clients every 10s ([#239](https://github.com/grafana/pyroscope-rs/issues/239)) ([8ca0102](https://github.com/grafana/pyroscope-rs/commit/8ca01023e827c23cdafb7a0ceeb641bfbb8c3f4b))
* drop drone. fix artifact names in release workflow ([b8e258d](https://github.com/grafana/pyroscope-rs/commit/b8e258d435f8752bf5a44d24003fef9a3aa2c766))
* Fix typo ([#65](https://github.com/grafana/pyroscope-rs/issues/65)) ([70ff908](https://github.com/grafana/pyroscope-rs/commit/70ff908cb84a4b15fefcf52275c1ca1bbb72569a))
* fork actions-upload-release-asset to use node16 to maybe fix release issues  ([#157](https://github.com/grafana/pyroscope-rs/issues/157)): ([1364a6c](https://github.com/grafana/pyroscope-rs/commit/1364a6cbf5b74c027089d05b0e98a4b4bfae2f2c))
* improve python tests speed ([#76](https://github.com/grafana/pyroscope-rs/issues/76)) ([74003b4](https://github.com/grafana/pyroscope-rs/commit/74003b4d80120a8123f347b678232f2c5ff1abf7))
* **kindasafe:** use runtime page size in page boundary tests ([#425](https://github.com/grafana/pyroscope-rs/issues/425)) ([5dd0dac](https://github.com/grafana/pyroscope-rs/commit/5dd0dac6e6c5d6d20cea0cfb1c578ef85de93ca8))
* log error body if response is not successfull ([#237](https://github.com/grafana/pyroscope-rs/issues/237)) ([08e0fb0](https://github.com/grafana/pyroscope-rs/commit/08e0fb0f85526bb688d14554a758960f5df1b2f8))
* migrate to `pprof2` crate ([#183](https://github.com/grafana/pyroscope-rs/issues/183)) ([ab864c6](https://github.com/grafana/pyroscope-rs/commit/ab864c6f17927456be3ba4d9aa2e87690f8b0648))
* namespaces modules to prevent namespace collisions ([630c37e](https://github.com/grafana/pyroscope-rs/commit/630c37e8dfcd04c11de7bb65c29532b25f94eefd))
* **pprof:** use nanos ([#155](https://github.com/grafana/pyroscope-rs/issues/155)) ([4f23122](https://github.com/grafana/pyroscope-rs/commit/4f23122518e9eae024ba4015d57199480fda62bf))
* **pyroscope-cli:** build ([#207](https://github.com/grafana/pyroscope-rs/issues/207)) ([2e03575](https://github.com/grafana/pyroscope-rs/commit/2e03575618b693d43fcb7b65fb2980889295e370))
* python wheel incorrect architecture on mac ([#162](https://github.com/grafana/pyroscope-rs/issues/162)) ([5771cad](https://github.com/grafana/pyroscope-rs/commit/5771cad7e4d9000f99722b0f778e48e324947abf))
* **python:** allow proper shutdown of the agent ([056038c](https://github.com/grafana/pyroscope-rs/commit/056038ceb79b707fb322d9e39f5a9adbe8ad9097))
* **python:** build ([#223](https://github.com/grafana/pyroscope-rs/issues/223)) ([e18351b](https://github.com/grafana/pyroscope-rs/commit/e18351b6a3908d1595de5578aa37705b7691a385))
* **python:** disable native profiling by default ([d2f68af](https://github.com/grafana/pyroscope-rs/commit/d2f68af4c3da176fee0e5839c2dbada4298121a2))
* **python:** ignore native in ffi ([#131](https://github.com/grafana/pyroscope-rs/issues/131)) ([1e7ccf9](https://github.com/grafana/pyroscope-rs/commit/1e7ccf9b84d5366b187ab773c0759ae01d0bc797))
* **python:** oncpu flag frong value ([#123](https://github.com/grafana/pyroscope-rs/issues/123)) ([094e2e7](https://github.com/grafana/pyroscope-rs/commit/094e2e7f4bdc8905f0d781a0bfabc66db2482adc))
* **rbspy:** premature oncpu exit ([#222](https://github.com/grafana/pyroscope-rs/issues/222)) ([2b0f543](https://github.com/grafana/pyroscope-rs/commit/2b0f54305f1a7611c7b66aa13470280cfa2120bb))
* release python macos typo ([a109224](https://github.com/grafana/pyroscope-rs/commit/a1092244ae23e5889339fd496db1b4e235b00d57))
* **release:** cli - use 20.04 runner, fix assets name ([9f57831](https://github.com/grafana/pyroscope-rs/commit/9f578314edc884d371d00515ddc6a0ebb24c6570))
* **release:** fix cli upload url ingh actions workflow ([0d9a491](https://github.com/grafana/pyroscope-rs/commit/0d9a4915d88fb4937eb9fe45afed813cb51e54c4))
* **rellease:** fix cli upload url ingh actions workflow ([f6a696a](https://github.com/grafana/pyroscope-rs/commit/f6a696a6ebd40de135d285944840acb975c81822))
* rename org_id to tenant_id ([#104](https://github.com/grafana/pyroscope-rs/issues/104)) ([64a23e0](https://github.com/grafana/pyroscope-rs/commit/64a23e0d4c2179608c53832975421d9a4f97fdb8))
* replace json with serde_json  ([#180](https://github.com/grafana/pyroscope-rs/issues/180)) ([face91b](https://github.com/grafana/pyroscope-rs/commit/face91b05de77d20660f0f694b87196fd92d6ee2))
* revert 0976d999e4a50c14459b8a0b39e72434be6d4bfc     ([#483](https://github.com/grafana/pyroscope-rs/issues/483)) ([ad3d032](https://github.com/grafana/pyroscope-rs/commit/ad3d03217a14a18868b8ba21f3dbf4ed844d759a))
* **ruby-publish:** fix creds ([ee14883](https://github.com/grafana/pyroscope-rs/commit/ee148833ea1828531c1f5bd46894c1ec48610abb))
* **ruby-publish:** make ruby publish non concurrent ([69c2079](https://github.com/grafana/pyroscope-rs/commit/69c2079680c1bdddddca0fe9a4d453f062b5729b))
* **ruby:** add missing libs for aarch64 linux ([97ce2e0](https://github.com/grafana/pyroscope-rs/commit/97ce2e04eca7b3ee2691e478fe090ad18a8691bf))
* running ruby release on arm selfhosted runnner ([#87](https://github.com/grafana/pyroscope-rs/issues/87)) ([4ec4f73](https://github.com/grafana/pyroscope-rs/commit/4ec4f7376edbe937b404ab5c203b44be6f9b8636))
* sdist not uploaded to GitHub release due to filename normalization ([#373](https://github.com/grafana/pyroscope-rs/issues/373)) ([482293f](https://github.com/grafana/pyroscope-rs/commit/482293fd1440eca0b482a52df047cd4ec254aeb8))
* **session:** remove debug statement ([9700209](https://github.com/grafana/pyroscope-rs/commit/97002095128e3d6fe62e677e195fbd7108f53768))
* **session:** use URL builder to construct ingest url ([786c89b](https://github.com/grafana/pyroscope-rs/commit/786c89bb99839c45778410012a6da60267d395df))
* support adhoc ([#109](https://github.com/grafana/pyroscope-rs/issues/109)) ([25aa401](https://github.com/grafana/pyroscope-rs/commit/25aa401b384e817fb8a29e602f4140f0835ea828))
* Update publish workflow ([#201](https://github.com/grafana/pyroscope-rs/issues/201)) ([b00cd2b](https://github.com/grafana/pyroscope-rs/commit/b00cd2b1f39aab5344818e827be2d50748d01e7c))
* use pprof encoding everywhere ([#122](https://github.com/grafana/pyroscope-rs/issues/122)) ([c6ecc5f](https://github.com/grafana/pyroscope-rs/commit/c6ecc5f72b75c2e9133f660b29247f2e8a8cf358))
* use prebuilt docker images based on manylinux with preinstalled cargo & rust ([#84](https://github.com/grafana/pyroscope-rs/issues/84)) ([b14f758](https://github.com/grafana/pyroscope-rs/commit/b14f758ec2a778395e086c6bfb973636384885a8))


### Miscellaneous Chores

* **actions:** add ARM64 self-hosted runner ([072920f](https://github.com/grafana/pyroscope-rs/commit/072920fc512af5b088b2a00548a2d9ecf9af1a0a))
* **actions:** remove dependabot ([fc25fff](https://github.com/grafana/pyroscope-rs/commit/fc25fff9953f540a3d81a8ce03fdc6e464130c11))
* **actions:** renamed ruby/python ci files ([62a4554](https://github.com/grafana/pyroscope-rs/commit/62a4554089f61df37587601e62de87896ce4b2b3))
* **actions:** restore ARM job ([316e060](https://github.com/grafana/pyroscope-rs/commit/316e06041ec3cc81e1e7ac31f931e902eb4145ed))
* **actions:** temporarily remove ARM64 job ([03a442f](https://github.com/grafana/pyroscope-rs/commit/03a442f5ea7e9ef4e001b51b6abf6d04d3edd9ef))
* add Renovate config for OpenSSL and downgrade to 3.5.4 to test ([#424](https://github.com/grafana/pyroscope-rs/issues/424)) ([c6e3741](https://github.com/grafana/pyroscope-rs/commit/c6e3741f688fc5848f36dd0950065c1cdfb8deff))
* add ThreadId type ([#305](https://github.com/grafana/pyroscope-rs/issues/305)) ([346597c](https://github.com/grafana/pyroscope-rs/commit/346597c42b473ef0934894f9c46ef3bd6c2702db))
* bump py-spy to 0.4.1 ([#251](https://github.com/grafana/pyroscope-rs/issues/251)) ([c0bb396](https://github.com/grafana/pyroscope-rs/commit/c0bb396686d1206b8d0977b8a8e2e81988d78337))
* bump pypa/gh-action-pypi-publish to 1.13.0 ([#256](https://github.com/grafana/pyroscope-rs/issues/256)) ([0ab2a64](https://github.com/grafana/pyroscope-rs/commit/0ab2a6444d9eee8c14e4123154c3abeaef1eeb7e))
* bump python ruby and lib versions ([#105](https://github.com/grafana/pyroscope-rs/issues/105)) ([617abbf](https://github.com/grafana/pyroscope-rs/commit/617abbfee1c078a557975acecbcdcde1ca5d3d85))
* bump rbspy to 0.37 ([#243](https://github.com/grafana/pyroscope-rs/issues/243)) ([39b3b3d](https://github.com/grafana/pyroscope-rs/commit/39b3b3d846f04484a5d9b7a1a5c9127ed574f1d9))
* cargo fmt ([#152](https://github.com/grafana/pyroscope-rs/issues/152)) ([a70f325](https://github.com/grafana/pyroscope-rs/commit/a70f3256bab624b25f365dd4afa0bc959ff69f50))
* centralize all dependency versions in workspace root Cargo.toml ([#390](https://github.com/grafana/pyroscope-rs/issues/390)) ([0be2912](https://github.com/grafana/pyroscope-rs/commit/0be29127056facb39136baa3b58fd6b9f8318e55))
* **ci:** extract python and ruby release jobs into a separate workflow file ([#246](https://github.com/grafana/pyroscope-rs/issues/246)) ([15726e6](https://github.com/grafana/pyroscope-rs/commit/15726e62b2dc8fcc4d9f42a58cebdcb5a1a95713))
* **ci:** update rust build toolchain ([f5bce68](https://github.com/grafana/pyroscope-rs/commit/f5bce683087cfcb87da0bd18059e7554cc418fc6))
* **ci:** update rust build toolchain ([3b786d3](https://github.com/grafana/pyroscope-rs/commit/3b786d3a1a224f75d5df573abaecf43dded3c8a0))
* **cli:** add adhoc ([#115](https://github.com/grafana/pyroscope-rs/issues/115)) ([927dcf2](https://github.com/grafana/pyroscope-rs/commit/927dcf22d1a6b746e041672db223498b3d5f1732))
* **cli:** bump vertion to 0.2.5 ([997a1c8](https://github.com/grafana/pyroscope-rs/commit/997a1c8116bd6551b1d5b25cb25471e1b3949a32))
* **cli:** docker builds ([#114](https://github.com/grafana/pyroscope-rs/issues/114)) ([2347d4c](https://github.com/grafana/pyroscope-rs/commit/2347d4c6be225b5ead2f77cba26f41108e72237a))
* **cli:** update dependencies ([828a7b6](https://github.com/grafana/pyroscope-rs/commit/828a7b6dba4d3c95f1a2b81ed4c77c716f039e24))
* **config:** migrate Renovate config ([#431](https://github.com/grafana/pyroscope-rs/issues/431)) ([ae0b38a](https://github.com/grafana/pyroscope-rs/commit/ae0b38af43fda93b9d5a940c99980ee128b265b5))
* **deps:** bump crossbeam-channel from 0.5.14 to 0.5.15 ([#204](https://github.com/grafana/pyroscope-rs/issues/204)) ([2146dd9](https://github.com/grafana/pyroscope-rs/commit/2146dd92ddc42e08d428bcf006520a2e1cfd9299))
* **deps:** bump openssl from 0.10.71 to 0.10.72 ([#205](https://github.com/grafana/pyroscope-rs/issues/205)) ([7e70610](https://github.com/grafana/pyroscope-rs/commit/7e706108a8176e43b8bf31db96bcb1ad20b47cce))
* **deps:** bump rustls from 0.22.3 to 0.22.4 ([#163](https://github.com/grafana/pyroscope-rs/issues/163)) ([b9c80d8](https://github.com/grafana/pyroscope-rs/commit/b9c80d8c02545cc32e13e97a50b593646684d88a))
* **deps:** bump rustls-webpki from 0.103.12 to 0.103.13 ([#524](https://github.com/grafana/pyroscope-rs/issues/524)) ([935384d](https://github.com/grafana/pyroscope-rs/commit/935384dc4d3479a1a12df2502898a8813b775884))
* **deps:** bump tokio from 1.44.1 to 1.44.2 ([#206](https://github.com/grafana/pyroscope-rs/issues/206)) ([c81fcf9](https://github.com/grafana/pyroscope-rs/commit/c81fcf9208dba46fddd75adfc0fe8f79b813dcd3))
* **deps:** disable py-spy default features to exclude CLI dependencies ([#418](https://github.com/grafana/pyroscope-rs/issues/418)) ([6ca5a16](https://github.com/grafana/pyroscope-rs/commit/6ca5a166d7e684a1096a5e9d549c8022e77d0257))
* **deps:** lock file maintenance ([#353](https://github.com/grafana/pyroscope-rs/issues/353)) ([6d005c9](https://github.com/grafana/pyroscope-rs/commit/6d005c9bd55744cee79f39ae5c880e8391cbc408))
* **deps:** lock file maintenance ([#361](https://github.com/grafana/pyroscope-rs/issues/361)) ([53e8113](https://github.com/grafana/pyroscope-rs/commit/53e8113f08257241c187dae434952d708f9715d0))
* **deps:** lock file maintenance ([#395](https://github.com/grafana/pyroscope-rs/issues/395)) ([6ffbe03](https://github.com/grafana/pyroscope-rs/commit/6ffbe0340dd4b6d49f05de706c1e1638406f84ce))
* **deps:** lock file maintenance ([#416](https://github.com/grafana/pyroscope-rs/issues/416)) ([1d192b0](https://github.com/grafana/pyroscope-rs/commit/1d192b046d41f98a5dd9819a865eaff37ab71bd9))
* **deps:** lock file maintenance ([#467](https://github.com/grafana/pyroscope-rs/issues/467)) ([09411da](https://github.com/grafana/pyroscope-rs/commit/09411daca91120c4863983273863c55b02b4dd89))
* **deps:** lock file maintenance ([#478](https://github.com/grafana/pyroscope-rs/issues/478)) ([447d8ee](https://github.com/grafana/pyroscope-rs/commit/447d8eef3621be5b9b09b0b9eca3968bca081c2a))
* **deps:** lock file maintenance ([#498](https://github.com/grafana/pyroscope-rs/issues/498)) ([4072a04](https://github.com/grafana/pyroscope-rs/commit/4072a04626abd98aa18bc6cda37fb0ccc0803f59))
* **deps:** lock file maintenance ([#502](https://github.com/grafana/pyroscope-rs/issues/502)) ([bef3bc3](https://github.com/grafana/pyroscope-rs/commit/bef3bc3ec08b57b74d5d177204118492cdaa50b2))
* **deps:** lock file maintenance ([#522](https://github.com/grafana/pyroscope-rs/issues/522)) ([4bd95bb](https://github.com/grafana/pyroscope-rs/commit/4bd95bb47f84f3aa6db6aa22173a7d74f774ab45))
* **deps:** pin dependencies ([#257](https://github.com/grafana/pyroscope-rs/issues/257)) ([9c1f929](https://github.com/grafana/pyroscope-rs/commit/9c1f9292c9e897b2dce72dc1c81645f86b0a5a88))
* **deps:** pin dependencies ([#341](https://github.com/grafana/pyroscope-rs/issues/341)) ([e14b212](https://github.com/grafana/pyroscope-rs/commit/e14b212f3801046bc83aab156626f423f5799800))
* **deps:** pin dependencies ([#396](https://github.com/grafana/pyroscope-rs/issues/396)) ([90d21fa](https://github.com/grafana/pyroscope-rs/commit/90d21fa44dda218c70c94034b126f90a1b1a129f))
* **deps:** pin dependencies ([#404](https://github.com/grafana/pyroscope-rs/issues/404)) ([d55cf4c](https://github.com/grafana/pyroscope-rs/commit/d55cf4c4578a6836682f5378cd715e985ffeff70))
* **deps:** pin dependencies ([#427](https://github.com/grafana/pyroscope-rs/issues/427)) ([a46830a](https://github.com/grafana/pyroscope-rs/commit/a46830ac9a75a7634d9cc1ecc514a21b5c0f0603))
* **deps:** pin rust docker tag to e8e2bb5 ([#501](https://github.com/grafana/pyroscope-rs/issues/501)) ([c177d5b](https://github.com/grafana/pyroscope-rs/commit/c177d5b8f28041d3956851d93e61be3aebc6d975))
* **deps:** Revert update dependency openssl/openssl to v3.5.5 ([#428](https://github.com/grafana/pyroscope-rs/issues/428))" ([#433](https://github.com/grafana/pyroscope-rs/issues/433)) ([c81f039](https://github.com/grafana/pyroscope-rs/commit/c81f039b2b334bc00d1ac4ddbb24c5411ff480e6))
* **deps:** update actions/checkout action to v4.3.1 ([#266](https://github.com/grafana/pyroscope-rs/issues/266)) ([32a2311](https://github.com/grafana/pyroscope-rs/commit/32a23111a5f3ebb85b8a7bdba8e50262f0894871))
* **deps:** update actions/checkout action to v6 ([#291](https://github.com/grafana/pyroscope-rs/issues/291)) ([a9ccd12](https://github.com/grafana/pyroscope-rs/commit/a9ccd1291d0fa0b2915e40b68ad53d884a875f83))
* **deps:** update actions/download-artifact action to v8 ([#391](https://github.com/grafana/pyroscope-rs/issues/391)) ([1879c54](https://github.com/grafana/pyroscope-rs/commit/1879c54bdcfbade4f751dba33d42f63f1c86d1e4))
* **deps:** update actions/setup-python action to v6 ([#304](https://github.com/grafana/pyroscope-rs/issues/304)) ([ae34a22](https://github.com/grafana/pyroscope-rs/commit/ae34a2227f730aad3765e427fc06c9d86ae18307))
* **deps:** update actions/upload-artifact action to v7 ([#393](https://github.com/grafana/pyroscope-rs/issues/393)) ([8373748](https://github.com/grafana/pyroscope-rs/commit/837374874b30ab2c235f2425aa558c62401f5a9e))
* **deps:** update dependency openssl/openssl to v3.5.5 ([#428](https://github.com/grafana/pyroscope-rs/issues/428)) ([3ebdee9](https://github.com/grafana/pyroscope-rs/commit/3ebdee9126011b4b23d4225c54979060900e88d8))
* **deps:** update dependency openssl/openssl to v3.5.5 ([#441](https://github.com/grafana/pyroscope-rs/issues/441)) ([886d586](https://github.com/grafana/pyroscope-rs/commit/886d586c2306bc9e9fa8f96e575a2ba9e930dbce))
* **deps:** update dependency ruby ([#267](https://github.com/grafana/pyroscope-rs/issues/267)) ([ad7fc30](https://github.com/grafana/pyroscope-rs/commit/ad7fc30105e7a581cb36d24092a77c03f2671f6f))
* **deps:** update dependency ruby to v4 ([#323](https://github.com/grafana/pyroscope-rs/issues/323)) ([f0070f1](https://github.com/grafana/pyroscope-rs/commit/f0070f1fe0076c93e3b0af8ae1092f1145c3136f))
* **deps:** update dependency ruby to v4.0.3 ([#523](https://github.com/grafana/pyroscope-rs/issues/523)) ([e126aa2](https://github.com/grafana/pyroscope-rs/commit/e126aa24517ba6efea62302e465ea736e4cef54c))
* **deps:** update dependency setuptools to v82 ([#324](https://github.com/grafana/pyroscope-rs/issues/324)) ([bcf6b0c](https://github.com/grafana/pyroscope-rs/commit/bcf6b0c276c17ab5b46c433da1c68810d23d9d9b))
* **deps:** update docker/dockerfile docker tag to v1.21 ([#342](https://github.com/grafana/pyroscope-rs/issues/342)) ([a7faed7](https://github.com/grafana/pyroscope-rs/commit/a7faed7b3e319b0185f5e65629ce098f0ef009dc))
* **deps:** update docker/dockerfile docker tag to v1.22 ([#405](https://github.com/grafana/pyroscope-rs/issues/405)) ([6c9bb7f](https://github.com/grafana/pyroscope-rs/commit/6c9bb7fa40608fa73ee178e671abd623f39ce57d))
* **deps:** update dtolnay/rust-toolchain digest to 3c5f7ea ([#480](https://github.com/grafana/pyroscope-rs/issues/480)) ([c4f2e7d](https://github.com/grafana/pyroscope-rs/commit/c4f2e7dd1b1234f8870ce0f858f92542aef37c2e))
* **deps:** update dtolnay/rust-toolchain digest to 3c5f7ea ([#500](https://github.com/grafana/pyroscope-rs/issues/500)) ([fc21bda](https://github.com/grafana/pyroscope-rs/commit/fc21bda959bdf5d5d5a44f559a2b9de8072d2fba))
* **deps:** update dtolnay/rust-toolchain digest to efa25f7 ([#345](https://github.com/grafana/pyroscope-rs/issues/345)) ([4b028ea](https://github.com/grafana/pyroscope-rs/commit/4b028eae90c5dd9e6f9f04be4db4faff7d446179))
* **deps:** update dtolnay/rust-toolchain digest to f7ccc83 ([#258](https://github.com/grafana/pyroscope-rs/issues/258)) ([5c60285](https://github.com/grafana/pyroscope-rs/commit/5c60285216aac61431dbe4c06f438e4056c4ecc7))
* **deps:** update github artifact actions (major) ([#325](https://github.com/grafana/pyroscope-rs/issues/325)) ([359f5fb](https://github.com/grafana/pyroscope-rs/commit/359f5fbfc2c143af0ed817f936f3fad7af82f815))
* **deps:** update googleapis/release-please-action digest to 5c625bf ([#509](https://github.com/grafana/pyroscope-rs/issues/509)) ([c347e17](https://github.com/grafana/pyroscope-rs/commit/c347e1775487456b996b87ce4672a20d9b9f4541))
* **deps:** update ruby:4.0 docker digest to 1daddc4 ([#376](https://github.com/grafana/pyroscope-rs/issues/376)) ([ff0f837](https://github.com/grafana/pyroscope-rs/commit/ff0f8375174e3a3cf9edb5345e0f5331ca5ff851))
* **deps:** update ruby:4.0 docker digest to 3b8c977 ([#377](https://github.com/grafana/pyroscope-rs/issues/377)) ([3bd6a89](https://github.com/grafana/pyroscope-rs/commit/3bd6a89714c5b74a7c982121a54d08cb8db854d1))
* **deps:** update ruby:4.0 docker digest to 6630261 ([#402](https://github.com/grafana/pyroscope-rs/issues/402)) ([2429e52](https://github.com/grafana/pyroscope-rs/commit/2429e521890be36a0149115aeda7269b718e3722))
* **deps:** update ruby/setup-ruby action to v1.288.0 ([#268](https://github.com/grafana/pyroscope-rs/issues/268)) ([8552797](https://github.com/grafana/pyroscope-rs/commit/855279789dafb42e830381b5e84e7c3b5831c5c4))
* **deps:** update ruby/setup-ruby action to v1.289.0 ([#406](https://github.com/grafana/pyroscope-rs/issues/406)) ([1463f69](https://github.com/grafana/pyroscope-rs/commit/1463f690dd939e905f1ec2e3a11b1b1cb6601dbe))
* **deps:** update rust crate anyhow to v1.0.100 ([#260](https://github.com/grafana/pyroscope-rs/issues/260)) ([796c229](https://github.com/grafana/pyroscope-rs/commit/796c229fb8caca1c422dbbaa1232cfd83c7f6a0a))
* **deps:** update rust crate anyhow to v1.0.101 ([#295](https://github.com/grafana/pyroscope-rs/issues/295)) ([4f1717c](https://github.com/grafana/pyroscope-rs/commit/4f1717ce1382cfab616ece58a6ca0a79af052583))
* **deps:** update rust crate anyhow to v1.0.102 ([#363](https://github.com/grafana/pyroscope-rs/issues/363)) ([1b04975](https://github.com/grafana/pyroscope-rs/commit/1b049750a2abdd9a944e4dade03afc7b446369a0))
* **deps:** update rust crate anyhow to v1.0.102 ([#397](https://github.com/grafana/pyroscope-rs/issues/397)) ([8634a73](https://github.com/grafana/pyroscope-rs/commit/8634a7310acec5ea9aad9e0ab40ed40366809127))
* **deps:** update rust crate cbindgen to v0.29.2 ([#274](https://github.com/grafana/pyroscope-rs/issues/274)) ([8d0e6fb](https://github.com/grafana/pyroscope-rs/commit/8d0e6fb738ef999bde63b565fef08afdb276ad53))
* **deps:** update rust crate env_logger to v0.11.10 ([#490](https://github.com/grafana/pyroscope-rs/issues/490)) ([24fb12d](https://github.com/grafana/pyroscope-rs/commit/24fb12d4c26c9cc167585f3fd93172015864a0e4))
* **deps:** update rust crate libc to v0.2.177 ([#261](https://github.com/grafana/pyroscope-rs/issues/261)) ([01c3b65](https://github.com/grafana/pyroscope-rs/commit/01c3b65a550cad0fd4b942c2729cb30811d7691d))
* **deps:** update rust crate libc to v0.2.180 ([#271](https://github.com/grafana/pyroscope-rs/issues/271)) ([5ca10c1](https://github.com/grafana/pyroscope-rs/commit/5ca10c12df00b39039cf40a3419878249f4760b7))
* **deps:** update rust crate libc to v0.2.183 ([#489](https://github.com/grafana/pyroscope-rs/issues/489)) ([b44bac8](https://github.com/grafana/pyroscope-rs/commit/b44bac8f4274882d793e9e9f92ab0ad9fc4692f0))
* **deps:** update rust crate libc to v0.2.185 ([#514](https://github.com/grafana/pyroscope-rs/issues/514)) ([e72f5c6](https://github.com/grafana/pyroscope-rs/commit/e72f5c6e4944e7b2ac949d6dbd3201dc426704d5))
* **deps:** update rust crate libflate to v2.2.1 ([#275](https://github.com/grafana/pyroscope-rs/issues/275)) ([b883474](https://github.com/grafana/pyroscope-rs/commit/b883474ce1946e12ed2b97d28628aa1570e01d9a))
* **deps:** update rust crate libflate to v2.3.0 ([#516](https://github.com/grafana/pyroscope-rs/issues/516)) ([da2835f](https://github.com/grafana/pyroscope-rs/commit/da2835f536771e089635e9fd8b97a860b09b9479))
* **deps:** update rust crate log to v0.4.29 ([#272](https://github.com/grafana/pyroscope-rs/issues/272)) ([f1b5f87](https://github.com/grafana/pyroscope-rs/commit/f1b5f87d7205f53f2391ab1f5c1927bf487d006f))
* **deps:** update rust crate pprof to v0.1500.3 ([#407](https://github.com/grafana/pyroscope-rs/issues/407)) ([0c6bcbc](https://github.com/grafana/pyroscope-rs/commit/0c6bcbc785d6f17c92e109f38190ef035bacb6cc))
* **deps:** update rust crate prost to v0.14.3 ([#296](https://github.com/grafana/pyroscope-rs/issues/296)) ([5d5a6ef](https://github.com/grafana/pyroscope-rs/commit/5d5a6eff1a893bade7c333c733bdfad4a69e44ba))
* **deps:** update rust crate rbspy to 0.43 ([#471](https://github.com/grafana/pyroscope-rs/issues/471)) ([1e524d3](https://github.com/grafana/pyroscope-rs/commit/1e524d38d84dfedc799515cb70ee434ef91a28e6))
* **deps:** update rust crate rbspy to 0.44 ([#477](https://github.com/grafana/pyroscope-rs/issues/477)) ([c7cfe39](https://github.com/grafana/pyroscope-rs/commit/c7cfe39fecd8a0624df5c5d22b709e73747ddb4d))
* **deps:** update rust crate reqwest to v0.12.24 ([#263](https://github.com/grafana/pyroscope-rs/issues/263)) ([7baa3b6](https://github.com/grafana/pyroscope-rs/commit/7baa3b6961b228aac1ba872244008bcb35007f48))
* **deps:** update rust crate reqwest to v0.13.2 ([#298](https://github.com/grafana/pyroscope-rs/issues/298)) ([bc44a21](https://github.com/grafana/pyroscope-rs/commit/bc44a21899b2c145b702c4ebf4a7ac9ab3d1dcb5))
* **deps:** update rust crate serde_json to v1.0.149 ([#288](https://github.com/grafana/pyroscope-rs/issues/288)) ([86cc6af](https://github.com/grafana/pyroscope-rs/commit/86cc6af9d2ee630d8a3a3449d292220ee4064d50))
* **deps:** update rust crate thiserror to v2.0.17 ([#264](https://github.com/grafana/pyroscope-rs/issues/264)) ([e164e0d](https://github.com/grafana/pyroscope-rs/commit/e164e0d4c58d2f9e5fb50411087964562901160a))
* **deps:** update rust crate thiserror to v2.0.18 ([#299](https://github.com/grafana/pyroscope-rs/issues/299)) ([43f18d5](https://github.com/grafana/pyroscope-rs/commit/43f18d5f08baef9bfb1763f52a95be825359806b))
* **deps:** update rust crate tokio to v1.49.0 ([#279](https://github.com/grafana/pyroscope-rs/issues/279)) ([edfce87](https://github.com/grafana/pyroscope-rs/commit/edfce87b619da500bf7ec9861ef780f7907b038f))
* **deps:** update rust crate url to v2.5.7 ([#265](https://github.com/grafana/pyroscope-rs/issues/265)) ([c13bbd8](https://github.com/grafana/pyroscope-rs/commit/c13bbd8a6cb8e60395d5774a108bc512903d1d80))
* **deps:** update rust crate url to v2.5.8 ([#303](https://github.com/grafana/pyroscope-rs/issues/303)) ([7cb12a5](https://github.com/grafana/pyroscope-rs/commit/7cb12a5f19943da0dfa3d30d64368ed484d57f20))
* **deps:** update rust crate uuid to v1.23.0 ([#481](https://github.com/grafana/pyroscope-rs/issues/481)) ([9c4b53a](https://github.com/grafana/pyroscope-rs/commit/9c4b53aef55b13dc6da99396119e33ec9dc02aa0))
* **deps:** update rust crate uuid to v1.23.1 ([#517](https://github.com/grafana/pyroscope-rs/issues/517)) ([6a89238](https://github.com/grafana/pyroscope-rs/commit/6a89238e7127cb5f3fa0f480ba7447369a21fc12))
* **deps:** update rust-lang/crates-io-auth-action action to v1.0.4 ([#479](https://github.com/grafana/pyroscope-rs/issues/479)) ([f4e08f3](https://github.com/grafana/pyroscope-rs/commit/f4e08f34aa2b14a1e417c005d37f75981940d675))
* **deps:** update rust:trixie docker digest to 652612f ([#510](https://github.com/grafana/pyroscope-rs/issues/510)) ([7f76541](https://github.com/grafana/pyroscope-rs/commit/7f76541afc2a03cd783b0a0627b7a876f18c4918))
* **deps:** update rust:trixie docker digest to e4f09e8 ([#515](https://github.com/grafana/pyroscope-rs/issues/515)) ([f450327](https://github.com/grafana/pyroscope-rs/commit/f450327b1901f43d97d96eee5e0d73ccb64aa768))
* **deps:** update softprops/action-gh-release action to v2.5.0 ([#336](https://github.com/grafana/pyroscope-rs/issues/336)) ([bf80ab4](https://github.com/grafana/pyroscope-rs/commit/bf80ab4ff2bb68d1506d31e10f0293c5ebe91b92))
* **deps:** update softprops/action-gh-release action to v2.5.2 ([#473](https://github.com/grafana/pyroscope-rs/issues/473)) ([1700be3](https://github.com/grafana/pyroscope-rs/commit/1700be3e34e7351a2f5f030187a2fac1ce1082e9))
* **deps:** update softprops/action-gh-release action to v2.5.3 ([#474](https://github.com/grafana/pyroscope-rs/issues/474)) ([12e34c1](https://github.com/grafana/pyroscope-rs/commit/12e34c109bd0e408dc0401430f4da4b5245a09da))
* **deps:** update softprops/action-gh-release action to v2.6.1 ([#475](https://github.com/grafana/pyroscope-rs/issues/475)) ([209c4c0](https://github.com/grafana/pyroscope-rs/commit/209c4c03ce08fe411046316dc333654fb64ab75b))
* disable unused unwind feature in python ([#202](https://github.com/grafana/pyroscope-rs/issues/202)) ([e3cf662](https://github.com/grafana/pyroscope-rs/commit/e3cf6622cdde87a299dcf3a5d38709af0abcd5dc))
* **docs:** add CONTRIBUTING guideline ([84d21df](https://github.com/grafana/pyroscope-rs/commit/84d21dfad116577b7881c89ba35305765ebc00e1))
* **docs:** add PR template ([cea870a](https://github.com/grafana/pyroscope-rs/commit/cea870a11f9699f2c821dec1d0592983b1a5b126))
* generate push api protos ([#309](https://github.com/grafana/pyroscope-rs/issues/309)) ([c6bed73](https://github.com/grafana/pyroscope-rs/commit/c6bed732d244ebaffe75e4e7b4f3f16e68e296e5))
* **main:** release lib 2.0.1 ([#496](https://github.com/grafana/pyroscope-rs/issues/496)) ([f51dcbf](https://github.com/grafana/pyroscope-rs/commit/f51dcbf43ed5f81eef309dd3b924eee45cf082af))
* **main:** release lib 2.0.2 ([#525](https://github.com/grafana/pyroscope-rs/issues/525)) ([b48bf47](https://github.com/grafana/pyroscope-rs/commit/b48bf47641981ffe36da53e8694c09f215a5ae5a))
* mass dependencies and toolchain upgrade ([#194](https://github.com/grafana/pyroscope-rs/issues/194)) ([f26484b](https://github.com/grafana/pyroscope-rs/commit/f26484bed0d7af1b3fcb9b293c109408d54018ec))
* **ops:** add dependabot config ([53ad880](https://github.com/grafana/pyroscope-rs/commit/53ad88057d83686031fe8aa6f67db6f4beaba1d6))
* **ops:** remove commented lines ([0cff44a](https://github.com/grafana/pyroscope-rs/commit/0cff44a206303e43cf952d334e8c156fb2f32ba3))
* prepare lib 2.0.0 ([#462](https://github.com/grafana/pyroscope-rs/issues/462)) ([a349cd1](https://github.com/grafana/pyroscope-rs/commit/a349cd1fb6381d2087dfb73f96560e6b402babd4))
* prepare Rust crate v0.6.0 ([#382](https://github.com/grafana/pyroscope-rs/issues/382)) ([0108336](https://github.com/grafana/pyroscope-rs/commit/01083367ab8a3eaa225a66d0bf6e29c879b51743))
* prepare Rust crate v1.0.0 ([#383](https://github.com/grafana/pyroscope-rs/issues/383)) ([043863d](https://github.com/grafana/pyroscope-rs/commit/043863d8ede6bb6dac62d24a3453dec90d707dd9))
* **release:** bump ruby/python versions ([6abac67](https://github.com/grafana/pyroscope-rs/commit/6abac6723842d811e0c39fa55ce96a420ad8419c))
* **release:** lib-0.5.3 release ([eaff8c0](https://github.com/grafana/pyroscope-rs/commit/eaff8c0d5eb7af2a9e167c618fc107c91f9b1f97))
* **release:** pprofrs-0.2.3 release ([f4b98fb](https://github.com/grafana/pyroscope-rs/commit/f4b98fbc0d15a4752de660459d7948218f0a7ff7))
* **release:** pyspy-0.2.3 release ([a5b1b78](https://github.com/grafana/pyroscope-rs/commit/a5b1b78d0bf2fff59e5c3bbf58f52086969a8d2f))
* **release:** python package 0.7.2 ([fe196d1](https://github.com/grafana/pyroscope-rs/commit/fe196d19111870548024cd0061864eb69a3a2f8d))
* **release:** rbspy-0.2.2 release ([dfccf95](https://github.com/grafana/pyroscope-rs/commit/dfccf95f2fef87f99e1ffe30e5a959f0f91c6356))
* **release:** ruby gem 0.3.1 ([9e566b4](https://github.com/grafana/pyroscope-rs/commit/9e566b4d1717892d1439b60e0fa8b43478d057a2))
* **release:** ruby gem 0.3.2 ([1f921cb](https://github.com/grafana/pyroscope-rs/commit/1f921cbc0f90ee23a8f843069867c7278227e5d7))
* **release:** update mac os version to 11 ([a51d37d](https://github.com/grafana/pyroscope-rs/commit/a51d37d6a2fdca003016908528d6bf81dd692732))
* remove FFI code and kindasafe (moved to separate repos) ([#488](https://github.com/grafana/pyroscope-rs/issues/488)) ([35a4c6a](https://github.com/grafana/pyroscope-rs/commit/35a4c6aa0808f0392e39e752d95a91536068ab74))
* remove obscure thread id hash check ([#302](https://github.com/grafana/pyroscope-rs/issues/302)) ([0aa8f01](https://github.com/grafana/pyroscope-rs/commit/0aa8f01831591a3f1516f42878b2ed35f6ba6c05))
* remove pyroscope_cli ([#238](https://github.com/grafana/pyroscope-rs/issues/238)) ([4f9bf47](https://github.com/grafana/pyroscope-rs/commit/4f9bf47dc83403902dc92291ec44300ef6b4f28b))
* remove std::ops::Add from Stacktrace ([#284](https://github.com/grafana/pyroscope-rs/issues/284)) ([90ce5a5](https://github.com/grafana/pyroscope-rs/commit/90ce5a5d87a1186dc74e63db121d14c22935c65e))
* remove support for collapsed format ([#310](https://github.com/grafana/pyroscope-rs/issues/310)) ([3f4f0c4](https://github.com/grafana/pyroscope-rs/commit/3f4f0c43d407184945a09e19c2cfad54420ed480))
* replace pprof crate with Grafana fork (pprof-pyroscope-fork) ([#386](https://github.com/grafana/pyroscope-rs/issues/386)) ([528bde3](https://github.com/grafana/pyroscope-rs/commit/528bde33d64821e1e458833b8f91172f52f56761))
* Report cleanup functions. ([#311](https://github.com/grafana/pyroscope-rs/issues/311)) ([fd61d6a](https://github.com/grafana/pyroscope-rs/commit/fd61d6a2778835ad68b2b52552d7ecfc905e8f33))
* **rs:** update pprof dependency, use framehop ([#245](https://github.com/grafana/pyroscope-rs/issues/245)) ([b80063a](https://github.com/grafana/pyroscope-rs/commit/b80063acb02765323ebdaf699884be5716d5544b))
* **ruby:** remove libunwind from ruby ([#216](https://github.com/grafana/pyroscope-rs/issues/216)) ([a5280fe](https://github.com/grafana/pyroscope-rs/commit/a5280fe11dc509199511991535b70e1a761ab119))
* **ruby:** ruby gem 0.3.0 release ([e21704f](https://github.com/grafana/pyroscope-rs/commit/e21704f6cb5830a17c6d059396a340d09af2453a))
* simplify crates ([#244](https://github.com/grafana/pyroscope-rs/issues/244)) ([bb1aeb1](https://github.com/grafana/pyroscope-rs/commit/bb1aeb1b2541d66f3c4c6fb433518bf29c3d15c3))
* support for pprof as transport format in ruby ([b6f2a0f](https://github.com/grafana/pyroscope-rs/commit/b6f2a0f8006febae7b306b36cf48dfcf55a3138b))
* Switched to self-hosted GitHub agents ([#232](https://github.com/grafana/pyroscope-rs/issues/232)) ([97e4572](https://github.com/grafana/pyroscope-rs/commit/97e4572ade11491d46384da486f313e66743b206))
* **tests:** update backend tests ([13e7c38](https://github.com/grafana/pyroscope-rs/commit/13e7c38d5a75c7c11496d0299a4aea7c4dcba1c8))
* update pyspy to include linetable panic fix  ([#208](https://github.com/grafana/pyroscope-rs/issues/208)) ([45d41fc](https://github.com/grafana/pyroscope-rs/commit/45d41fc2c3bf8415f73c280821b761afdc0e7232))
* update rbspy to 16.0 ([#82](https://github.com/grafana/pyroscope-rs/issues/82)) ([2aa1f6b](https://github.com/grafana/pyroscope-rs/commit/2aa1f6b9ebc6a222f67e9ae3a49f9eea63bf287d))
* update rbspy to 19.1 ([#156](https://github.com/grafana/pyroscope-rs/issues/156)) ([00b6424](https://github.com/grafana/pyroscope-rs/commit/00b6424b20419a1781b27edba61c8648ed015823))
* update wasm-bindgen ecosystem to fix yanked package warnings ([#385](https://github.com/grafana/pyroscope-rs/issues/385)) ([4dbc476](https://github.com/grafana/pyroscope-rs/commit/4dbc4769c66d81286426b33c40a7a184ddac1f74))
* use rbspy fork crate instead of submodule ([#118](https://github.com/grafana/pyroscope-rs/issues/118)) ([205163a](https://github.com/grafana/pyroscope-rs/commit/205163a1af545ddf6b4aafcb8ce8fcbe1baf06a9))


### Documentation

* fix broken Rust documentation and add doc-test CI ([#484](https://github.com/grafana/pyroscope-rs/issues/484)) ([98ae8a5](https://github.com/grafana/pyroscope-rs/commit/98ae8a595f18946f2edf27544ea6e2bb29724cbf))
* **README:** update README files ([ee3a3db](https://github.com/grafana/pyroscope-rs/commit/ee3a3dbf0c0f774b66954e7b545a5892c7a5ec90))
* remove SECURITY.md ([#270](https://github.com/grafana/pyroscope-rs/issues/270)) ([f3f30a1](https://github.com/grafana/pyroscope-rs/commit/f3f30a1f15dd455e8b7cd65518013cba1fa6694f))


### Code Refactoring

* **cargo:** remove patch section from main Cargo.toml file ([fdef71b](https://github.com/grafana/pyroscope-rs/commit/fdef71bd17a7959c7f5cdc8959823d827a5f8dd3))
* **examples:** change regex example ([4a3c120](https://github.com/grafana/pyroscope-rs/commit/4a3c12055c258558582e330c3315d115ad292c27))
* **examples:** update examples ([0b83280](https://github.com/grafana/pyroscope-rs/commit/0b832805a3008aa99a20c672452440bb5ee8855a))
* **ffikit:** handle unwrap properly ([0baafe7](https://github.com/grafana/pyroscope-rs/commit/0baafe77ac396483edbf277cbda306fc0414a2bf))
* **ffikit:** use lazy_static ([06340ec](https://github.com/grafana/pyroscope-rs/commit/06340ec4ff85250bf12ba49e6b096b9be5520e6f))
* **python:** add pyroscope-io for backward compatbility ([4808dc8](https://github.com/grafana/pyroscope-rs/commit/4808dc819a5ffff39a955d08c71beb3885b17836))
* **python:** change defaults ([79f92ee](https://github.com/grafana/pyroscope-rs/commit/79f92eeebbf7cab3fc4102311aa7853c663153f7))
* **rbspy:** use forked upstream crate ([1aef9aa](https://github.com/grafana/pyroscope-rs/commit/1aef9aa8c19dde15dc0f59575c2b9863775a0f66))
* **ruby:** add fork process support ([64a1f87](https://github.com/grafana/pyroscope-rs/commit/64a1f87565e230af2e6bb0c2788641e03b58882f))
* **ruby:** update ruby spec ([827dedf](https://github.com/grafana/pyroscope-rs/commit/827dedf402425f3e84edaecd542c8243e5a7eeb4))
* **session:** minor refactoring ([2d3b116](https://github.com/grafana/pyroscope-rs/commit/2d3b11690e1088ba82750c986926ca4dad6d6d3e))
* **timer:** remove warning ([6608943](https://github.com/grafana/pyroscope-rs/commit/6608943330f1a6ae33b63e902a61a90bbc9dec72))


### Continuous Integration

* add clippy job to Rust CI to catch warnings ([#384](https://github.com/grafana/pyroscope-rs/issues/384)) ([26fee2e](https://github.com/grafana/pyroscope-rs/commit/26fee2e7a5bb1c03a1e07fc673f6e0d81b522522))
* add musl/Alpine to Rust test matrix ([#495](https://github.com/grafana/pyroscope-rs/issues/495)) ([a4b7ad4](https://github.com/grafana/pyroscope-rs/commit/a4b7ad4c9e530576adbbf278b104d65c970d43b0))
* add release-please and PR title check workflows ([#403](https://github.com/grafana/pyroscope-rs/issues/403)) ([b844fa6](https://github.com/grafana/pyroscope-rs/commit/b844fa6fdfcd8060e1ff5307edc538c1d8dc7e7f))
* add release-please configuration ([#491](https://github.com/grafana/pyroscope-rs/issues/491)) ([fbb2d8e](https://github.com/grafana/pyroscope-rs/commit/fbb2d8edc7cf400343650e77580464d32369d93d))
* add Ruby 3.4.6 to ffi ruby test matrix ([#347](https://github.com/grafana/pyroscope-rs/issues/347)) ([f3b5ed2](https://github.com/grafana/pyroscope-rs/commit/f3b5ed24ca648bc5b0a1e8c32eb8c7512b738ab5))
* add Rust test workflow ([#285](https://github.com/grafana/pyroscope-rs/issues/285)) ([2a9a34c](https://github.com/grafana/pyroscope-rs/commit/2a9a34c81f37105a922a942931eeb3572e7366fb))
* fix PR title check and version script for renovate PRs ([#408](https://github.com/grafana/pyroscope-rs/issues/408)) ([a846ee4](https://github.com/grafana/pyroscope-rs/commit/a846ee476a7050add1e29aa9c6ef16683ae159e9))
* fix release-please illegal path traversal in extra-files ([#411](https://github.com/grafana/pyroscope-rs/issues/411)) ([f8a5fbf](https://github.com/grafana/pyroscope-rs/commit/f8a5fbf53ecaff40abb74bb4d1e7640b542f8e8f))
* improves ruby linux build pipeline by adding libraries to gems ([#43](https://github.com/grafana/pyroscope-rs/issues/43)) ([271f8b3](https://github.com/grafana/pyroscope-rs/commit/271f8b31d2890a0019fbb275f69d083aa0cb10f8))
* pin get-vault-secrets action to commit SHA ([#412](https://github.com/grafana/pyroscope-rs/issues/412)) ([09125d0](https://github.com/grafana/pyroscope-rs/commit/09125d09f1b2773ee47e8cef94ce77ca72b31645))
* replace unmaintained release asset uploader action in Python and Ruby workflows ([#330](https://github.com/grafana/pyroscope-rs/issues/330)) ([60e7eec](https://github.com/grafana/pyroscope-rs/commit/60e7eec8834f8472b4ff9a34a51234b5b2a86c89))
* split publish.yml and replace unmaintained release action ([#380](https://github.com/grafana/pyroscope-rs/issues/380)) ([c8b97ae](https://github.com/grafana/pyroscope-rs/commit/c8b97aec8e86e642374b37d437d469cc487ccf65))
* use crates.io trusted publishing for Rust crate ([#381](https://github.com/grafana/pyroscope-rs/issues/381)) ([9cd838c](https://github.com/grafana/pyroscope-rs/commit/9cd838c9a2cd03230a1a9c8e816bde63314a1a50))
* use github-hosted runners on forks, grafana runners on upstream ([#410](https://github.com/grafana/pyroscope-rs/issues/410)) ([20f22fd](https://github.com/grafana/pyroscope-rs/commit/20f22fd315fdd04433ea8569117d56f999f69bed))
* use large runner for publish-rust-crate workflow ([#388](https://github.com/grafana/pyroscope-rs/issues/388)) ([2b6eafb](https://github.com/grafana/pyroscope-rs/commit/2b6eafbc01d4d7eb8c226cd15edc12466ad94407))
* use main-only push trigger for ruby ffi workflow ([#332](https://github.com/grafana/pyroscope-rs/issues/332)) ([458462f](https://github.com/grafana/pyroscope-rs/commit/458462f436f06d39d6a7fc56ba4fe92ddd6b54be))
* verify release tag version matches Cargo package version ([#389](https://github.com/grafana/pyroscope-rs/issues/389)) ([1f9d4b8](https://github.com/grafana/pyroscope-rs/commit/1f9d4b8979ce28616b77bea3ce1f83ccabcbc931))

## [2.0.2](https://github.com/grafana/pyroscope-rs/compare/lib-2.0.1...lib-2.0.2) (2026-04-24)


### Miscellaneous Chores

* **deps:** bump rustls-webpki from 0.103.12 to 0.103.13 ([#524](https://github.com/grafana/pyroscope-rs/issues/524)) ([935384d](https://github.com/grafana/pyroscope-rs/commit/935384dc4d3479a1a12df2502898a8813b775884))
* **deps:** lock file maintenance ([#522](https://github.com/grafana/pyroscope-rs/issues/522)) ([4bd95bb](https://github.com/grafana/pyroscope-rs/commit/4bd95bb47f84f3aa6db6aa22173a7d74f774ab45))
* **deps:** update dependency ruby to v4.0.3 ([#523](https://github.com/grafana/pyroscope-rs/issues/523)) ([e126aa2](https://github.com/grafana/pyroscope-rs/commit/e126aa24517ba6efea62302e465ea736e4cef54c))
* **deps:** update rust:trixie docker digest to e4f09e8 ([#515](https://github.com/grafana/pyroscope-rs/issues/515)) ([f450327](https://github.com/grafana/pyroscope-rs/commit/f450327b1901f43d97d96eee5e0d73ccb64aa768))

## [2.0.1](https://github.com/grafana/pyroscope-rs/compare/lib-2.0.0...lib-2.0.1) (2026-04-19)


### Bug Fixes

* revert 0976d999e4a50c14459b8a0b39e72434be6d4bfc     ([#483](https://github.com/grafana/pyroscope-rs/issues/483)) ([ad3d032](https://github.com/grafana/pyroscope-rs/commit/ad3d03217a14a18868b8ba21f3dbf4ed844d759a))


### Miscellaneous Chores

* **deps:** lock file maintenance ([#467](https://github.com/grafana/pyroscope-rs/issues/467)) ([09411da](https://github.com/grafana/pyroscope-rs/commit/09411daca91120c4863983273863c55b02b4dd89))
* **deps:** lock file maintenance ([#478](https://github.com/grafana/pyroscope-rs/issues/478)) ([447d8ee](https://github.com/grafana/pyroscope-rs/commit/447d8eef3621be5b9b09b0b9eca3968bca081c2a))
* **deps:** lock file maintenance ([#498](https://github.com/grafana/pyroscope-rs/issues/498)) ([4072a04](https://github.com/grafana/pyroscope-rs/commit/4072a04626abd98aa18bc6cda37fb0ccc0803f59))
* **deps:** lock file maintenance ([#502](https://github.com/grafana/pyroscope-rs/issues/502)) ([bef3bc3](https://github.com/grafana/pyroscope-rs/commit/bef3bc3ec08b57b74d5d177204118492cdaa50b2))
* **deps:** pin dependencies ([#427](https://github.com/grafana/pyroscope-rs/issues/427)) ([a46830a](https://github.com/grafana/pyroscope-rs/commit/a46830ac9a75a7634d9cc1ecc514a21b5c0f0603))
* **deps:** pin rust docker tag to e8e2bb5 ([#501](https://github.com/grafana/pyroscope-rs/issues/501)) ([c177d5b](https://github.com/grafana/pyroscope-rs/commit/c177d5b8f28041d3956851d93e61be3aebc6d975))
* **deps:** update dtolnay/rust-toolchain digest to 3c5f7ea ([#480](https://github.com/grafana/pyroscope-rs/issues/480)) ([c4f2e7d](https://github.com/grafana/pyroscope-rs/commit/c4f2e7dd1b1234f8870ce0f858f92542aef37c2e))
* **deps:** update dtolnay/rust-toolchain digest to 3c5f7ea ([#500](https://github.com/grafana/pyroscope-rs/issues/500)) ([fc21bda](https://github.com/grafana/pyroscope-rs/commit/fc21bda959bdf5d5d5a44f559a2b9de8072d2fba))
* **deps:** update googleapis/release-please-action digest to 5c625bf ([#509](https://github.com/grafana/pyroscope-rs/issues/509)) ([c347e17](https://github.com/grafana/pyroscope-rs/commit/c347e1775487456b996b87ce4672a20d9b9f4541))
* **deps:** update rust crate env_logger to v0.11.10 ([#490](https://github.com/grafana/pyroscope-rs/issues/490)) ([24fb12d](https://github.com/grafana/pyroscope-rs/commit/24fb12d4c26c9cc167585f3fd93172015864a0e4))
* **deps:** update rust crate libc to v0.2.183 ([#489](https://github.com/grafana/pyroscope-rs/issues/489)) ([b44bac8](https://github.com/grafana/pyroscope-rs/commit/b44bac8f4274882d793e9e9f92ab0ad9fc4692f0))
* **deps:** update rust crate libc to v0.2.185 ([#514](https://github.com/grafana/pyroscope-rs/issues/514)) ([e72f5c6](https://github.com/grafana/pyroscope-rs/commit/e72f5c6e4944e7b2ac949d6dbd3201dc426704d5))
* **deps:** update rust crate libflate to v2.3.0 ([#516](https://github.com/grafana/pyroscope-rs/issues/516)) ([da2835f](https://github.com/grafana/pyroscope-rs/commit/da2835f536771e089635e9fd8b97a860b09b9479))
* **deps:** update rust crate rbspy to 0.43 ([#471](https://github.com/grafana/pyroscope-rs/issues/471)) ([1e524d3](https://github.com/grafana/pyroscope-rs/commit/1e524d38d84dfedc799515cb70ee434ef91a28e6))
* **deps:** update rust crate rbspy to 0.44 ([#477](https://github.com/grafana/pyroscope-rs/issues/477)) ([c7cfe39](https://github.com/grafana/pyroscope-rs/commit/c7cfe39fecd8a0624df5c5d22b709e73747ddb4d))
* **deps:** update rust crate uuid to v1.23.0 ([#481](https://github.com/grafana/pyroscope-rs/issues/481)) ([9c4b53a](https://github.com/grafana/pyroscope-rs/commit/9c4b53aef55b13dc6da99396119e33ec9dc02aa0))
* **deps:** update rust crate uuid to v1.23.1 ([#517](https://github.com/grafana/pyroscope-rs/issues/517)) ([6a89238](https://github.com/grafana/pyroscope-rs/commit/6a89238e7127cb5f3fa0f480ba7447369a21fc12))
* **deps:** update rust-lang/crates-io-auth-action action to v1.0.4 ([#479](https://github.com/grafana/pyroscope-rs/issues/479)) ([f4e08f3](https://github.com/grafana/pyroscope-rs/commit/f4e08f34aa2b14a1e417c005d37f75981940d675))
* **deps:** update rust:trixie docker digest to 652612f ([#510](https://github.com/grafana/pyroscope-rs/issues/510)) ([7f76541](https://github.com/grafana/pyroscope-rs/commit/7f76541afc2a03cd783b0a0627b7a876f18c4918))
* **deps:** update softprops/action-gh-release action to v2.5.2 ([#473](https://github.com/grafana/pyroscope-rs/issues/473)) ([1700be3](https://github.com/grafana/pyroscope-rs/commit/1700be3e34e7351a2f5f030187a2fac1ce1082e9))
* **deps:** update softprops/action-gh-release action to v2.5.3 ([#474](https://github.com/grafana/pyroscope-rs/issues/474)) ([12e34c1](https://github.com/grafana/pyroscope-rs/commit/12e34c109bd0e408dc0401430f4da4b5245a09da))
* **deps:** update softprops/action-gh-release action to v2.6.1 ([#475](https://github.com/grafana/pyroscope-rs/issues/475)) ([209c4c0](https://github.com/grafana/pyroscope-rs/commit/209c4c03ce08fe411046316dc333654fb64ab75b))
* remove FFI code and kindasafe (moved to separate repos) ([#488](https://github.com/grafana/pyroscope-rs/issues/488)) ([35a4c6a](https://github.com/grafana/pyroscope-rs/commit/35a4c6aa0808f0392e39e752d95a91536068ab74))


### Documentation

* fix broken Rust documentation and add doc-test CI ([#484](https://github.com/grafana/pyroscope-rs/issues/484)) ([98ae8a5](https://github.com/grafana/pyroscope-rs/commit/98ae8a595f18946f2edf27544ea6e2bb29724cbf))


### Continuous Integration

* add musl/Alpine to Rust test matrix ([#495](https://github.com/grafana/pyroscope-rs/issues/495)) ([a4b7ad4](https://github.com/grafana/pyroscope-rs/commit/a4b7ad4c9e530576adbbf278b104d65c970d43b0))
* add release-please configuration ([#491](https://github.com/grafana/pyroscope-rs/issues/491)) ([fbb2d8e](https://github.com/grafana/pyroscope-rs/commit/fbb2d8edc7cf400343650e77580464d32369d93d))

## v2.0.0

## Breaking Changes
- `Backend::report()` now returns `ReportBatch` instead of `Vec<Report>` ([#437](https://github.com/grafana/pyroscope-rs/pull/437), [#447](https://github.com/grafana/pyroscope-rs/pull/447))
- New `ReportData` enum replaces the plain `Vec<Report>` in `ReportBatch`, supporting both structured reports (`ReportData::Reports`) and pre-encoded pprof bytes (`ReportData::RawPprof`) ([#447](https://github.com/grafana/pyroscope-rs/pull/447))
- Minimum supported Rust version bumped from 1.64 to 1.66

## New Features
- **Jemalloc memory profiling backend** — new `backend-jemalloc` feature flag enables heap profiling via `jemalloc_pprof`. Use `pyroscope::backend::jemalloc::jemalloc_backend()` to get started ([#378](https://github.com/grafana/pyroscope-rs/pull/378))
- **`ReportBatch` type** — backends now return a `ReportBatch` with a `profile_type` field (e.g. `"process_cpu"`, `"memory"`), enabling multi-profile support ([#437](https://github.com/grafana/pyroscope-rs/pull/437))

## Dependencies
- Updated `pprof` (pprof-pyroscope-fork) to v0.1500.3 ([#407](https://github.com/grafana/pyroscope-rs/pull/407))
- Updated `object` crate to 0.38 ([#430](https://github.com/grafana/pyroscope-rs/pull/430))
- Disabled py-spy default features to exclude CLI dependencies ([#418](https://github.com/grafana/pyroscope-rs/pull/418))
- Added `jemalloc_pprof` 0.8 and `tokio` 1 as workspace dependencies

## v1.0.0
## Breaking Changes
- Removed `auth_token` from Python and Ruby FFI bindings and related code
- Removed `detect_subprocesses` from Python and Ruby configs
- Config constructor now requires app/spy identity and sample rate inputs
- Removed support for collapsed format
- Removed global tags from ruleset

## New Features
- Integrated pprof-rs backend into main crate behind optional `backend-pprof-rs` feature
- Switched to push API (from `/ingest` to `/push`)
- Generated push API protos
- Added `ThreadId` type
- Added `rustls-no-provider` TLS feature

## Bug Fixes / Improvements
- Unified signal logic
- Report cleanup functions
- Optimized ruleset
- Removed obscure thread id hash check
- Ruby: inline thread_id crate; remove detect_subprocess
- Dependency updates (reqwest 0.13, prost 0.14, thiserror 2.0, serde_json 1.0.115, uuid 1.20, libflate 2.1)

## v0.5.4
## New Features
- Add report transfromation function which allows changing the report before
  sending it to the Pyroscope Server.
- Add Gzip support.

## Bug Fixes
- Use URL Builder. ([786c89b](https://github.com/pyroscope-io/pyroscope-rs/commit/786c89bb99839c45778410012a6da60267d395df))

## v0.5.3
## New Features
- Add BackendConfig to make reporting of pid, thread_id and thread_name
  optional. 
- Backends can add a suffix to the "application_name"

## Bug Fixes
- **main**: fixed an obsecure bug when counting stacktraces ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/bdecaa13aeae3ce7d4c3d97f88bdd104ec35e7c5))

## v0.5.2
## New features
- Authentication Token support

## API Changes
- use rust-tls instead of openssl

## v0.5.1
## API Changes
- Backend shutdown bug
- Docs update

## v0.5.0
## API Changes
- New API for building, starting and stopping the profiling agent.
- Backend supports reporting multiple threads.
- Tagging within local thread-scope

## v0.4.0
## API Changes
- Backend now support passing a configuration struct.
- TimerSignal enum
- pprof-rs backend is split into a different package. It has to be imported manually.

## What's Changed
* fix: avoid binding two unrelated vars to the same type by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/18
* avoid almost all unwraps by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/14
* use more features of `thiserror` by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/11
* introduce LOG_TAGs, avoid repetitive prefixes by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/10
* allow configurable accumulation_cycle by @drahnr in https://github.com/pyroscope-io/pyroscope-rs/pull/21
* Add CI Targets by @omarabid in https://github.com/pyroscope-io/pyroscope-rs/pull/22
* 0.4.0 release by @omarabid in https://github.com/pyroscope-io/pyroscope-rs/pull/23

## New Contributors
* @drahnr made their first contribution in https://github.com/pyroscope-io/pyroscope-rs/pull/18

**Full Changelog**: https://github.com/pyroscope-io/pyroscope-rs/compare/0.3.1...lib-0.4.0

## v0.3.1
Minor release with bug fixes.

## Bug Fixes
- **session**: avoid breaking SessionManager if request fails ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/1254bcc9a3b0d76b7adbeb82ba21f33b875c0b39))
- **typo**: variable name typo ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/0d8caffbe7855bec8158333dba2923cd07286a5f))
- **pprof**: fix #12 ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/936d3d99a2cc82812bc8251fd2fbf8152a87d4cb))
- **copyright**: fix #13 ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/b8eaf13137810df932e1b70e33b3ad3e25b65ece))

## Code Refactoring
- **option**: replace unwrap for various Options ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/3fd4e794d74523855c66f65c0b7fc8ff9dfe4604))
- **time**: add get_time_range ([Abid Omar](https://github.com/pyroscope-io/pyroscope-rs/commit/a6d4dbcef494b2bfe8016a817201499937cf3528))

## v0.3.0
First stable release

## v0.0.2-alpha
Second beta release

## v0.0.1-alpha
Initial beta release
