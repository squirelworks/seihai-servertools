Notes for the next seihai-servertools release. One line per user-visible change, added by the PR that makes it; the release PR moves them into `vX.Y.Z.md` (VERSIONING.md).

- `seihai-suite-stress probe` has two new steps. `want FILE` sends a file request. `expect SECONDS TEXT` fails the run unless a matching server message arrives in time, so a steps file can be a smoke test (seihai-gserver-v2#87).
