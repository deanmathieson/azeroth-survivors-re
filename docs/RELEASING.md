# Releasing Azeroth Survivors

1. Make sure the intended friends build is on `progression`, including any co-op wire changes.
   Bump the co-op `PROTOCOL` whenever that wire format changes; mixed protocol builds must not
   join each other.
2. Push the intended commit to `progression` and wait for the Windows release-cache warmer. Its
   first run takes about 20 minutes; later tag releases should take about 5–8 minutes. Tag builds
   can restore only the default branch's cache: if the default branch is not `progression`, set
   **Settings → Branches → default** to `progression` before relying on this cache.
3. Create and push a `vX.Y.Z` tag on `progression` (for example, `git tag v0.3.1` followed by
   `git push origin v0.3.1`).
4. The Windows release workflow builds the non-dev co-op executable, packages it without WoW
   data, and publishes both a versioned archive and the stable `AzerothSurvivors.zip` download.
5. Friends download once from
   <https://github.com/deanmathieson/azeroth-survivors-re/releases/latest/download/AzerothSurvivors.zip>.
   On their next launcher start it checks for the release and updates while preserving their
   data path, last host, names, and `benilla-config` saves.
