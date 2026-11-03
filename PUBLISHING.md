# Trusted Publishing Guide (npm & crates.io via GitHub Actions OIDC)

`msfPDF` supports modern **Tokenless Trusted Publishing** backed by OpenID Connect (OIDC) and Sigstore cryptographic provenance (SLSA Build Level 2), eliminating the need for long-lived static API tokens stored in GitHub Secrets.

---

## 1. Configuring npm Trusted Publishing (`msfpdf`)

npm supports GitHub Actions OIDC directly. Once configured, GitHub Actions mints a short-lived OIDC exchange token (`id-token: write`) automatically during release workflows.

### First-Time Setup on npmjs.com:
1. Log in to [npmjs.com](https://www.npmjs.com/) and navigate to your package settings for **`msfpdf`** (or create the package).
2. Go to **Settings** -> **Publishing Access** -> **Trusted Publishers**.
3. Click **Add Trusted Publisher** -> Select **GitHub Actions**.
4. Configure the fields:
   - **GitHub Organization or User:** `mfelisberto90`
   - **Repository:** `msfPDF`
   - **Workflow filename:** `publish.yml`
   - **Environment:** *(leave blank or specify `release`)*
5. Save. The workflow will now automatically publish with `--provenance` attestation.
6. *(Optional Fallback)*: If you prefer traditional tokens, save your granular npm automation token as `NPM_TOKEN` under GitHub Repository Secrets.

---

## 2. Configuring crates.io Trusted Publishing (`msfpdf-core` & `msfpdf-cli`)

crates.io supports GitHub Actions OIDC publishing:

### Setup on crates.io:
1. Log in to [crates.io](https://crates.io/) and navigate to **Account Settings** -> **Trusted Publishing** (or individual crate settings for `msfpdf-core` and `msfpdf-cli`).
2. Add a new publisher selecting GitHub:
   - **Repository:** `mfelisberto90/msfPDF`
   - **Workflow:** `publish.yml`
3. *(Optional Fallback)*: Alternatively, add your crates.io publishing token as `CARGO_REGISTRY_TOKEN` under GitHub Repository Secrets.

---

## 3. Triggering a Release

To trigger automated publishing:
1. Bump version numbers in `Cargo.toml`, `crates/msfpdf-core/Cargo.toml`, `crates/msfpdf-cli/Cargo.toml`, and `package.json`.
2. Commit and draft a GitHub Release (e.g. `v0.2.1` or `v0.3.0`).
3. Click **Publish Release**.
4. GitHub Actions will run `.github/workflows/publish.yml`, verify all tests, generate Sigstore provenance attestation, and publish both npm and crates.io packages simultaneously.
