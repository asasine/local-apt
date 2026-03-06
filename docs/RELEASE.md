# Release

1. Add changelog entries during development:

   ```bash
   dch -M "Description of change"
   ```

1. Merge changes into `main`
1. Run the **Release** workflow from the Actions tab, providing the new version number

   The workflow will finalize the changelog, update `Cargo.toml`, commit, tag, and push.
   CI runs on the tag, and on success, CD builds the `.deb` and creates a GitHub Release.

1. Verify the release at `https://github.com/asasine/local-apt/releases`

   Artifact provenance can be verified with:

   ```bash
   gh attestation verify ./local-apt_<VERSION>_amd64.deb --owner asasine
   ```
