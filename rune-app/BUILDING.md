# Chiikawizator desktop builds

The Windows and macOS apps are native Rust executables. Python, Node, a browser and a local web server are not needed to run an installed app. Build tooling uses Rust **1.99.1** and Node **22**; Windows packaging also uses NSIS 3. macOS bundles require macOS 13 or later and are built separately for Apple Silicon and Intel.

## Framework checkout

Check out `https://github.com/anomalyco/rune` at revision `ea6ddf8f2c3cd476a3797caad7d77cbe04641d9c` into `rune-app/vendor/rune`. An existing local checkout can be linked there instead. The app pins all four registry patches to this checkout, and its workspace excludes Rune's own workspace. Do not commit the local checkout, junction or `target` directory.

## Personal build credential

Set `CHIIKAWA_EMBEDDED_MCP_KEY` in the build environment or use the existing workspace `.env` entry `ELYUM_MCP_KEY`. Scripts read this value without printing it or evaluating the `.env` file as shell code. GitHub Actions reads the repository secret `CHIIKAWA_EMBEDDED_MCP_KEY`.

The resulting executable contains the personal credential. CI never uploads an unencrypted executable, installer or Cargo build cache. Share decrypted installers only with intended users of the personal account.

## Windows

From the repository directory:

```powershell
./rune-app/scripts/build-windows.ps1
```

This runs Rust tests in the release profile, creates the release executable, verifies the offline interface, and builds a per-user English installer at `rune-app/dist/Chiikawizator_4.1.0_windows-x64-setup.exe`. Using the same profile avoids compiling the graphics dependencies twice. No administrator rights are needed to install the app. The installed app still offers Russian, English and Japanese. Installer pages load the bundled Chiikawa font privately; it is not registered system-wide.

The app, Start menu entry and desktop shortcut are named **Chiikawizator**. The installer retains the existing 4.0 registry identity and installation directory when upgrading, replaces the old executable and removes only the known old shortcuts. Drawings and settings remain in the existing application data directory. New installations default to `Programs/Chiikawizator`.

For packaging an already verified release, pass `-SkipBuild -SkipSmoke`. Use `-NsisPath PATH_TO_MAKENSIS` if NSIS is not detected automatically.

## macOS

```sh
bash rune-app/scripts/build-macos.sh
```

The script builds and tests in the release profile on the host architecture, creates `Chiikawizator.app`, adds its icon and licenses, signs it ad hoc, runs offline verification and creates a verified DMG under `rune-app/dist`. The disk contains the app and an Applications shortcut. Ad-hoc signing verifies bundle integrity; it is not Apple Developer ID signing or notarization.

`--skip-build` packages an existing release binary. `--skip-tests` and `--skip-smoke` are available for an already verified local build. CI uses the complete verification sequence.

## Verification and private artifacts

```sh
node --test rune-app/scripts/artifact-crypto.test.mjs rune-app/scripts/stage-source.test.mjs rune-app/scripts/sanitize-verification.test.mjs
node rune-app/scripts/verify-package.mjs EXECUTABLE OUTPUT_DIRECTORY
```

Verification sets `CHIIKAWIZATOR_OFFLINE=1`, runs `--self-test`, renders `home-ru`, `home-en` and `home-ja`, checks the corresponding JSON trees, and presents ten frames in an actual native window. It does not submit or buy images. Diagnostics are checked for credential content before upload.

If verification fails, CI uploads the reports separately only after their text has been sanitized. Unexpected binary files and symlinks prevent that diagnostic upload. Raw application executables and installers are never included in a diagnostic archive.

The `Chiikawizator desktop apps` GitHub Actions workflow produces encrypted `.enc` installers and screenshots/check reports. Restore a downloaded installer locally with the same build key in the environment:

```sh
node rune-app/scripts/artifact-crypto.mjs decrypt DOWNLOADED.dmg.enc Chiikawizator.dmg
```

The format uses AES-256-GCM with a fresh salt and IV. Failed authentication deletes the partial output. Existing destination files are never overwritten.

## Source staging

```sh
node rune-app/scripts/stage-source.mjs
```

This stages app sources and the Rune workflow under `build/rune-ci-stage`, scans selected content for credentials, writes a root README linking to the app guide, and records `rune-source-manifest.json`. It excludes the framework checkout, local configuration, compiled files and build outputs. It writes the exact scanned bytes and rejects nested junctions or symlinks in the destination. An explicit staging checkout must also be inside this workspace's `build` directory. Restage after the final source changes, then review the Git diff before committing. The script does not delete unrelated files, commit, push or start a workflow.
