# Local file safety

`LocalFolder` checks relative paths and resolves existing ancestors against
the canonical world root before listing, reading or writing. An existing
symlink or Windows directory junction must resolve inside that root. This
also applies when the requested file or its immediate parent is missing.
Links that stay inside the world continue to work.

These are path checks, not handle-relative filesystem isolation. They do
not prevent an adversary from replacing links between validation and file
access. Use a world folder whose directory structure is controlled by its
owner; no adversarial link-race protection is claimed.

World-file writes and config `backup::write_atomic` use exclusively created,
uniquely named temporary files beside the destination. A successful save
replaces the complete destination; failed writes or replacements clean up
only their own temporary file. Existing files with the former deterministic
temporary name remain untouched. Config dated-backup and undo behavior is
unchanged. Atomic replacement does not imply power-loss durability or
compare-and-swap conflict detection.

Synthetic tests cover outside links and missing descendants, allowed inside
links, preservation of unrelated temporary names, replacement failure and
cleanup, and simultaneous world saves returning one complete version.
Windows uses directory junctions; Unix uses symlinks. No original-client or
adversarial link-race experiment is claimed.
