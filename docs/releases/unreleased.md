Notes for the next seihai-servertools release. One line per user-visible change, added by the PR that makes it; the release PR moves them into `vX.Y.Z.md` (VERSIONING.md).

- `seihai-suite-stress probe` has two new steps. `want FILE` sends a file request. `expect SECONDS TEXT` fails the run unless a matching server message arrives in time, so a steps file can be a smoke test (seihai-gserver-v2#87).
- Built on seihai-core and seihai-draw v0.7.2, with seihai-rc at a `main` commit on the same core, so the lock file holds one core (it held v0.6.1 and, through seihai-rc, v0.4.0). The LAT NPC-server database page now tokenizes NPC and class scripts in the server's text profile, which seihai-rc reads from the server's `Server:` line.
