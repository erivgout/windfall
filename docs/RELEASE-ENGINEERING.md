# Release engineering: first R1 packet

This packet implements a candidate build and evidence pipeline. It does not publish a release or complete phase 7/R1. Existing CI, Cargo/package/Tauri versions, runtime, updater, CSP, credentials, resources and global roadmap/parity accounting are unchanged.

## Candidate contract

`.github/workflows/release-candidate.yml` accepts an explicit full lowercase source commit, a future semantic version already present in that commit, `prerelease` or `stable`, and a distribution mode. The workflow is written in the JSON subset of YAML 1.2 so its syntax and contract can be checked with Node built-ins. `scripts/release/check-workflow.mjs` rejects duplicate fields, unsupported action pins, unsafe shell expression interpolation and publication commands.

The workflow tools are checked out separately at the immutable workflow commit. The requested source is checked out at the requested commit. Preflight validates the workspace version, **every workspace Cargo package**, desktop package version, Tauri version, product name/identifier, clean source, resource contract and published identities. Inputs reach shell commands through quoted environment variables. Nothing changes source versions on the runner. Source commits need to contain the real factory crate and desktop build, and all existing integration/repair gates must pass before the parent authorizes a candidate run.

Only `unsigned-development` is implemented. `signed-release` fails closed, including when operator profile/verifier names are present: the native signing/verifier integration still requires a serialized implementation window. It cannot fall back to development mode. A stable channel input does not make an unsigned candidate a stable published distribution. There are no signing secrets wired into this workflow.

Native build rows:

| Runner             | Native target              | Bundle and exact filename              |
| ------------------ | -------------------------- | -------------------------------------- |
| `windows-2022`     | `x86_64-pc-windows-msvc`   | NSIS: `Windfall_VERSION_x64-setup.exe` |
| `macos-15` (ARM64) | `aarch64-apple-darwin`     | DMG: `Windfall_VERSION_aarch64.dmg`    |
| `ubuntu-22.04`     | `x86_64-unknown-linux-gnu` | Debian: `Windfall_VERSION_amd64.deb`   |

The collector also checks the actual native Node OS/architecture. Intel macOS, Windows ARM64, RPM and AppImage are not admitted yet. The inspected Tauri AppImage bundler can download `linuxdeploy-plugin-appimage` from a mutable `continuous` release and fall back to a different implementation. That requires a reviewed, pinned tool/input contract before admission. Ubuntu 22.04 provides the existing WebKitGTK 4.1 baseline; actual minimum OS/runtime compatibility still requires install tests.

Builds use frozen pnpm installation, `tauri build --ci --no-sign --target ... --bundles ... -- --locked`, the existing Linux prerequisites, and the existing Windows `scripts/msvc-env.sh`. Cargo jobs and Rust test threads are each 1. Generated bindings go to runner temp. The fresh native target is `source/target/release-r1-native`; build targets and previous bundles are **not** restored from cache. Only locked package sources/stores are cached. The factory verifier runs before bundling. No new npm/Rust application dependency is added.

Workflow artifacts are named `unsigned-development-VERSION-PLATFORM-ARCH-RUN-ATTEMPT`, retained for 14 days, uploaded only after verification, and never overwritten. Repository permission is `contents: read`; checkout credentials are not persisted. Read-only API tokens are available only to the two identity checks, not to the native build steps. Uploading a workflow artifact is not uploading a GitHub Release asset. Parent alone handles future publication after reviewing all gates. Dispatch requires the workflow to be available through the repository's Actions/default-branch workflow rules; this worker does not dispatch or push it.

## Immutable published alpha

`v0.1.0-alpha.1` is permanently reserved. Its annotated tag object is `9e7094f1ec1053cbae83d48067fc594fc593855a`; its dereferenced commit is `157f96fd068b46adab046a6880ce27ea4b49aea1`.

| Existing asset                         |   Bytes | SHA256                                                             |
| -------------------------------------- | ------: | ------------------------------------------------------------------ |
| `Windfall_0.1.0-alpha.1_x64-setup.exe` | 9070359 | `51b1ab3a6d1fa2ad0f6bfb78c9bacf9ecc1bb73a4415dd1c6dfc5db7286a81a3` |
| `SHA256SUMS.txt`                       |     103 | `520617b06f4fb04b1bbd75b51cc23fa23143a7b0df3902b0ecdceb4ad75a78d6` |

The tools reject that version, its source commit, and any semantic version at or below it. Remote preflight checks the tag object/dereference, release flags and asset sizes/digests when GitHub supplies digests. It rejects existing version tags and existing published **or draft** releases, and repeats the check after building. Missing API evidence or pagination failure is an error. These read-only checks do not download assets and are not a fresh byte verification of the historical installer. A race after the check remains a publication-time gate; the candidate pipeline cannot reserve a release identity by mutating GitHub.

The present source version remains `0.1.0-alpha.1`, so it intentionally cannot pass candidate preflight. The parent must approve and commit a fresh matching version in all reserved metadata/lock entries before running the pipeline. A fixture using an authored future version is not evidence that the current app is released.

## Manifest, checksums and source evidence

Each platform produces a separate candidate directory containing:

- `manifest.json`: schema version 1, expected source/version/channel, platform/architecture/triple, workflow commit/run/runner image and actual Git/Node/pnpm/Rust/Cargo/Tauri tool versions, exact artifact paths/sizes/SHA256, signing/notarization states and unresolved publication gates.
- `SHA256SUMS.txt`: sorted checksums for every inventoried payload **and the manifest**; the checksum file does not hash itself.
- `installers/`: only the expected native bundle for that row.
- `source/Windfall-COMMIT.tar`: `git archive` of the exact expected commit with a fixed prefix and tar umask. It includes checked-in build/install sources and lockfiles. It does not substitute a branch archive URL for corresponding source.
- `source/inventory.json`: complete tracked source inventory, Git blobs/modes/tree, exact checkout byte hashes, and source capture digest.
- `resources.json`: exact source resource mapping, every factory file's size/hash and packaged destination, generator reference and provenance limitations.
- `dependencies.json`: a machine-readable **Windfall lock-backed SBOM**, all Cargo/npm lock entries (including build and optional platform packages), pinned registry checksums/integrities, actual installed declared licenses, provenance/notice gaps and the resolved Cargo graph. It is not labelled SPDX or CycloneDX conformance.
- `notices/inventory.json` and `notices/`: source notices and available dependency license/notice texts. Workspace packages without a separate license file use the repository license text. This is evidence collection, not a license-compatibility decision.

The generator checks the supplied capture against current source before and after collection. Source reads are checked against their immutable Git blob, including files hidden with `assume-unchanged`; Git-confirmed LF-to-CRLF checkout conversion is allowed and the actual checkout bytes are still recorded. Source symlinks/submodules and archive `export-ignore`/`export-subst` attributes are rejected; Git's global archive attributes are disabled. The verifier independently checks the externally supplied commit/version/channel, recreates the source archive, repeats workspace/resource/lock/notices checks, hashes real artifacts, and rejects omissions, tampering, duplicate logical artifacts, case collisions, path traversal, absolute paths, symlink/junction escapes, malformed/duplicate JSON fields, unknown states and contradictory signing claims. The manifest cannot remove publication blockers or claim compliance.

Paths are portable forward-slash relative names. Sorting uses explicit codepoint order, not locale order. Windows device names, alternate streams and trailing dots/spaces are rejected. Candidate files are nonempty regular files. Inventory JSON and checksums have fixed ordering/newlines and contain no generated wall-clock timestamp. Repeated inventories with identical inputs/provenance produce identical bytes; per-OS checkout line endings, different run identities, compiler/tool/runner versions, installer timestamps, signatures/notarization receipts and bundler downloads can change outputs. This packet does **not** promise byte-reproducible signed installers. JSON object order is canonical in produced evidence; deliberately reordered metadata is rejected rather than silently normalized.

Outputs are staged in an owned sibling directory. The final directory is reserved with an exclusive mkdir; each file is installed by an atomic no-clobber hard link, with the manifest linked last as the completion marker. Existing final directories/files and a competing destination are preserved. A failed final installation can leave an incomplete directory without a completion marker; rerun into a new destination after inspecting it. Staging cleanup checks its resolved parent, prefix and ownership token and never deletes the whole source/workspace. The destination filesystem must support local hard links. Capture output uses the same exclusive temporary-file/link approach.

Hashes establish integrity, not publisher authenticity. Treat workflow provenance and the original candidate manifest as trusted review inputs; there is no signed attestation yet. In particular, native executable identity, embedded resources, loaded libraries, signer chains and installability are not established merely by recognizing a filename. They are explicit blockers. Security state `signature: not-verified` is intentional: `--no-sign` disables requested distribution signing, but the collector does not pretend to verify every binary or classify compiler/platform ad-hoc signatures. macOS notarization is `not-requested`; other rows use `not-applicable`.

## License, resource and future integration gates

Cargo evidence comes from `cargo metadata --locked` and Cargo.lock. npm evidence comes from installed `pnpm licenses list --json`, package declarations and the v9 pnpm lockfile. The lock reader admits pinned registry identities/checksums only and fails on unsupported source forms/lock shapes. Installed dependency paths must stay in the source/declared Cargo registry/npm stores. Optional packages absent on the current platform remain `not-installed-on-this-platform`; missing declarations or notice texts remain `missing-license-or-notice`. Malformed or inconsistent metadata fails generation. Missing provenance appears as an explicit manifest blocker. All declared licenses still require review; no license expression is rewritten into a supposed GPL-equivalent license.

The exact-commit source tar is the project's corresponding-source starting point. Complete distribution obligations still require review of dependency corresponding source, vendored/native/bundler tools and platform libraries, any build scripts and special license/notice requirements. The Cargo/npm inventory is not a complete binary system-library SBOM. The candidate always has `publication.eligible: false`. A successful candidate gate must never be described as GPL compliance or production approval.

The accepted source resource seam is exactly `../../../content/factory/` to `factory/`. Its 33 current WAVs plus README/license must match the actual Rust factory manifest, and the workflow's Rust factory verifier checks generated bytes. The CC0 dedication belongs to those sounds; the generator remains GPL-3.0-or-later. Source resource evidence is not an extracted-installer inspection. Platform files, platform config overrides, extra resource maps, additional desktop binary targets and `bundle.externalBin` require an accepted contract before this collector admits them.

N4 must first supply an accepted helper binary name/target inventory, parent-owned runtime discovery choice, ABI/auth/lifecycle contract and verified packaged native fixtures. Only then can a serialized window add the agreed `bundle.externalBin` or resource map and corresponding verifier. M1 must first supply an accepted model/resource manifest with exact content digests, origin, license text, redistribution/corresponding-source obligations and runtime discovery policy. No helper, weights or inference claim is added by this packet. Parent serializes both seams; worker foundation code does not establish working inference or shipping resources.

## Operator signing, notarization and updater requirements

The `WINDFALL_SIGNING_PROFILE` and `WINDFALL_SIGNING_VERIFIER` names identify future reviewed operator configuration; they are **not** Tauri environment variables and do not unlock the current signed gate. Do not put secrets in dispatch inputs, source config, logs or a candidate directory.

Windows requires an operator-owned certificate/provider and approved timestamp service. Tauri's inspected Windows config has `certificateThumbprint`, `digestAlgorithm`, `timestampUrl`, `tsp`, or an approved `signCommand` using `%1` as its binary placeholder. The certificate must be provisioned to the native runner/provider without exposing its key. A later implementation must verify actual app/helper/installer Authenticode chains, signer identity, digest and timestamp before declaring verified signing. No certificate thumbprint, timestamp URL or provider credentials are invented here.

macOS outside-App-Store distribution requires an operator-owned Developer ID Application identity, hardened runtime/approved entitlements and notarization. Official Tauri inputs include `APPLE_SIGNING_IDENTITY`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`; notarization can use `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`, or the approved Apple-ID path `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`. Import keys only in an isolated runner keychain with reviewed cleanup. Ad-hoc identity `-` is not authenticated distribution signing. A later verifier must run actual codesign/Gatekeeper checks plus stapled-ticket validation for the shipped app/DMG, with receipts bound to its hashes. The current workflow neither imports certificates nor requests notarization.

Linux requires an agreed signing/distribution policy and verifier for the exact Debian package/manifest plus repository trust. No Linux signing key/account is configured here.

Auto-update is a separate runtime/config window. It needs an operator-approved HTTPS update host, public updater key and secret references `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, matched channels and immutable artifact URLs. Tauri updater signatures are distinct from OS code signing. Neither update URL nor key is invented, and the updater plugin/runtime is not installed by this packet. Offline/corrupt/downgrade handling, rollback and project-format safety must be tested before an updater is enabled.

## Local commands and handoff

Use Node 26.4.0 (executed locally) or the pinned workflow toolchain. These commands write no release metadata and use no signing material:

```powershell
node --test --test-concurrency=1 scripts/release/release.test.mjs
node scripts/release/check-workflow.mjs
```

For an approved clean future-version checkout, first capture outside the checkout, build native bundles with the exact workflow flags, then collect and verify. The CLI requires all identity fields on every command; it never derives the expected source identity from the manifest under review:

```text
node scripts/release/cli.mjs preflight --source SOURCE --commit COMMIT --version VERSION --channel prerelease --mode unsigned-development --github
node scripts/release/cli.mjs capture --source SOURCE --commit COMMIT --version VERSION --channel prerelease --mode unsigned-development --output CAPTURE.json
node scripts/release/cli.mjs candidate --source SOURCE --commit COMMIT --version VERSION --channel prerelease --mode unsigned-development --capture CAPTURE.json --bundle-root BUNDLE_DIR --platform windows --architecture x64 --output NEW_CANDIDATE_DIR
node scripts/release/cli.mjs verify --source SOURCE --commit COMMIT --version VERSION --channel prerelease --mode unsigned-development --candidate NEW_CANDIDATE_DIR
```

`--github` uses an already configured read-only `GH_TOKEN` and is mandatory in the workflow identity steps. It is optional for offline local fixtures/collection. Local Windows compilation must use Git Bash after sourcing `scripts/msvc-env.sh`, an isolated `target/release-r1-native`, jobs 1 and a task-specific binding-output directory. Run one Cargo build at a time. The fixture suite invokes Cargo metadata only; it does not compile Rust, generate bindings/WASM or install tools. Authored fixture bundles are real byte fixtures, not installable app claims.

Requested next parent integration window, after this foundation is accepted:

1. Approve the future version and serialize reserved Cargo/package/Tauri/lock changes. Preserve the existing identifier/product identity and immutable alpha. No particular version is selected by this worker.
2. Optionally add a narrow existing-CI job/steps using `node --test --test-concurrency=1 scripts/release/release.test.mjs` and `node scripts/release/check-workflow.mjs` with Node 26.4.0; existing CI is untouched here.
3. Serialize `apps/desktop/src-tauri/tauri.conf.json` (or a reviewed dedicated release override) for `bundle.windows.allowDowngrades: false`, approved Windows signing fields, macOS hardened-runtime/identity/entitlements and native verification plumbing. Do not enable a signed-release mode until the verifier binds verified signer/notarization receipts to actual artifact hashes.
4. Admit N4/M1 resources only after their accepted contracts above, then serialize updater dependencies/capabilities/runtime/config after the operator supplies approved key/host references and recovery tests. Request only that bounded window; do not reopen timeline/DSP/native runtime ownership.

Executed locally on Windows: all 38 Node fixtures passed (no skips), including real temporary artifact bytes, generator/verifier, CLI success/failure exits, source/archive/metadata tampering, CRLF, junction escapes, no-clobber contention and missing/contradictory provenance. The unchanged Tauri config and an in-memory future-version copy passed the installed 2.12.1 schema with existing Ajv; non-Unicode regex mode was required for the schema's escaped filename pattern and uint8/uint32 formats were explicitly registered. Existing Prettier's YAML parser accepted the workflow, all 14 shell command blocks passed Git Bash syntax checking, and the workflow contract check passed. The actual lock readers parsed 538 Cargo and 589 npm entries. CLI preflight rejected current alpha with exit 1 as required. Formatting and whitespace checks passed. No native app compilation or environment tool installation was needed for these checks.

Still unexecuted: GitHub dispatch; real new-version Windows/macOS/Linux bundle runs; authenticated signing/notarization; extracted package/native dependency inventories; fresh-user install/uninstall/upgrade with project/settings preservation; and update-host/runtime recovery tests. Full R1 also retains crash consent/recovery, tested CSP, localization/tutorials, native multi-window polish and macOS/Linux renderer/accessibility verification. This packet closes none of those phase/parity rows.

## Verified upstream contracts and pins

Inspected local Tauri CLI/schema version 2.12.1 and upstream Tauri CLI tag commit `30da1fd6e17de6107ecc850c95dfb16b5729f2dd` for native filename/build contracts. Current action commits below were resolved through read-only upstream GitHub refs on 2026-10-08; all four upstream repository licenses report MIT, and their runtime is Node 24. Their committed lockfiles/distribution bundles pin their transitives; there is no floating action or third-party Rust-toolchain/pnpm-setup action added here. Registry metadata/integrities were checked for the 174 unique actual runtime transitives across these action locks, resolving npm aliases and legacy SHA1 integrities correctly. 173 have declared licenses (MIT, ISC, Apache-2.0, BSD, 0BSD, BlueOak-1.0.0 or the exact combined expressions); `buffers@0.1.1` in upload-artifact has no license declaration/text in its integrity-checked tarball and its referenced upstream repository is unavailable. That missing provenance is an explicit `build-tool-license-provenance` publication blocker, not a fabricated MIT/GPL equivalence or a closed tool-license review. The supported candidate-only workflow uses the official Actions service; these tool dependencies are not added to or linked into Windfall's runtime.

Existing pnpm 11.1.2 was verified against registry metadata: MIT, Node >=22.13, no separately declared runtime dependencies; distribution integrity `sha512-QVocwll0cx51RVwUaDcb50xapft2IbUNQFbSIkUWCfEUEvI/1gLmFp8eBgRmZB95hZfhvpYaEGiINqZ7FlaUmQ==`. Its bundled dependency/native tool obligations remain subject to the distribution review above. Rust 1.99.0 and Node 26.4.0 match the installed local tools; official Node distribution checksums exist for that exact version. Native NSIS/DMG bundler download/source closures and runner-provided tools still require the same build-tool review before production distribution; this packet introduces no custom remote tool or `curl|sh` installer.

| Action                           | Verified commit                            |
| -------------------------------- | ------------------------------------------ |
| `actions/checkout` v7.0.1        | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| `actions/setup-node` v7.1.0      | `949feb2413d6458794dcd2491c4babbbce0c15c1` |
| `actions/cache` v6.1.0           | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` |
| `actions/upload-artifact` v7.0.2 | `cf430e030ddbb5b0abf93d22962f4752f3646cd9` |

Primary references: [Tauri distribution/build contract](https://v2.tauri.app/distribute/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/), [macOS signing/notarization](https://v2.tauri.app/distribute/sign/macos/), [Debian baseline](https://v2.tauri.app/distribute/debian/), [updater](https://v2.tauri.app/plugin/updater/), [GitHub workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax), [runner architectures for private repositories](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [secure use of Actions](https://docs.github.com/en/actions/reference/security/secure-use), [artifact upload contract at the pinned action](https://github.com/actions/upload-artifact/tree/cf430e030ddbb5b0abf93d22962f4752f3646cd9), [Node 26.4.0 distribution checksums](https://nodejs.org/dist/v26.4.0/SHASUMS256.txt). These are configuration references, not evidence that Windfall passed external release gates.
