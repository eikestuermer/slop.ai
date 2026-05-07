# integration-tauri

Closes `S-TAURI-001` once signing + notarization are wired with real certs.

## Job body (current — unsigned smoke)

The current job body in [.github/workflows/ci.yml](../../../../.github/workflows/ci.yml) runs `pnpm tauri build` on macOS / Linux / Windows. This proves the bundler works; it does not produce signed/notarized installers.

## Phase B work

1. **macOS notarization**: provision an Apple Developer ID certificate; export as base64 into `APPLE_CERT_BASE64`; populate `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` repo secrets. The release workflow at [.github/workflows/release.yml](../../../../.github/workflows/release.yml) already references these.
2. **Windows code signing**: Azure Trusted Signing — populate `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TS_*`. Already wired.
3. **Linux**: `.deb` / `.rpm` / `.AppImage` need the GPG signing key (`GPG_SIGNING_KEY`) wired so apt/dnf repos accept the artifacts.
4. **Smoke step**: install the signed installer in a fresh container, run the binary with `--help` or `--version`, assert exit code 0.

## Promotion criteria

- All three OSes produce signed/notarized installers in CI.
- A smoke installer-and-launch step passes.
- Then promote to gating, mark `S-TAURI-001` green.
