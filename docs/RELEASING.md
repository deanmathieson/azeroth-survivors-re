# Releasing Azeroth Survivors

1. Make sure the intended friends build is on `progression`, including any co-op wire changes.
   Bump the co-op `PROTOCOL` whenever that wire format changes; mixed protocol builds must not
   join each other.
2. Create and push a `vX.Y.Z` tag on `progression` (for example, `git tag v0.3.1` followed by
   `git push origin v0.3.1`).
3. The Windows release workflow builds the non-dev co-op executable, packages it without WoW
   data, and publishes both a versioned archive and the stable `AzerothSurvivors.zip` download.
   It is normally live in about 25 minutes.
4. Friends download once from
   <https://github.com/deanmathieson/azeroth-survivors-re/releases/latest/download/AzerothSurvivors.zip>.
   On their next launcher start it checks for the release and updates while preserving their
   data path, last host, names, and `benilla-config` saves.
