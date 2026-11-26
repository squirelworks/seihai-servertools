//! A 2D Graal playerworld as 3D worlds that seihai-gserver
//! serves to the 4.20r6, 4.20r8 and 5.x 3D clients.
//!
//! A 3D client draws none of a level's tiles, signs or NPCs
//! (Graal_analysis `torque/3d-win10-sky-prefs-scripting-512-2026-11-26.md`,
//! T3D-138), so everything has to become 3D content the client does draw:
//! `.3ds` GameObjects with colour materials, a Sky and a Sun.
//!
//! The steps:
//! - [`meaning`] works out what each tile shows (tree, house, door, cliff,
//!   path, water, fence, rock, ...) from the names of the object-set stamps
//!   that use it, with the tiledefs types as the fallback.
//! - [`scene`] extrapolates 3D from that: ground heights from the cliffs a
//!   3/4 view draws, trees with trunks and crowns, houses with walls, a
//!   gabled roof and a door, shrubs, rocks, fences, sunk water.
//! - [`convert`] lays out the worlds (each gmap one world with its levels
//!   side by side, every other level a world of its own) and writes one
//!   `.3ds` a level ([`shape`]); the `PW3DWalk` weapon ([`walker`]) walks
//!   the player over the heights, stops it at walls, follows links between
//!   worlds and shows signs.
//!
//! The output is a seihai-gserver world folder: `config/graal3dworld.txt`
//! (the start level), `config/graal3dworlds/<name>.txt` (each other world,
//! named after its level or gmap so a TCP warp to that level takes a 3D
//! client along), `files/` (shapes and sky), `weapons/pw3dwalk.gs2` and the
//! 2D levels unchanged in `levels/`, plus `PW3D-REPORT.md`.
//! Everything generated is our own; the playerworld's levels are copied, so
//! the folder stays local.

pub mod convert;
pub mod meaning;
pub mod scene;
pub mod shape;
pub mod walker;

use std::path::Path;

/// Write `files` under `out`; without `force`, refuse if any exists.
pub fn write_files(out: &Path, files: &[convert::File], force: bool) -> Result<(), String> {
    if !force {
        if let Some((name, _)) = files.iter().find(|(n, _)| out.join(n).exists()) {
            return Err(format!(
                "{} already exists (use --force to replace)",
                out.join(name).display()
            ));
        }
    }
    for (name, bytes) in files {
        let p = out.join(name);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::write(&p, bytes).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(())
}
