# Release

1. Make a new changelog version:

   ```bash
   # append version to end
   dch -Mv
   ```

1. Mark that release as finalized:

   ```bash
   dch -Mr
   ```

1. Merge changes into `main`
1. Tag the merge commit

   ```bash
   git tag "v$(dpkg-parsechangelog -S Version)"
   ```
