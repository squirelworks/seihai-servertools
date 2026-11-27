Notes for the next seihai-servertools release. One line per user-visible change, added by the PR that makes it; the release PR moves them into `vX.Y.Z.md` (VERSIONING.md).

- `seihai-edit3d` no longer lists valid `portal` and `regions` lines (seihai-gserver 0.2.0) as problems, and two terrain saves in the same second keep separate backups (`-2`, `-3` ...) instead of the second overwriting the first. New `docs/3D-TOOLS.md`: a guide to the standalone 3D tools.
