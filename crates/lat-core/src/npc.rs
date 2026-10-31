//! NPCs for the NPC editor. **Planned** (`docs/NEXT.md`, LAT track).
//!
//! Two kinds of NPC:
//!
//! - **Level NPCs**: the `NPC ... NPCEND` blocks in a level. Edited through
//!   the level document, with the whole script (the Seihai editor's pane
//!   shows 20 lines). Syntax check with `seihai-core`'s GS1 parser.
//! - **Database NPCs, classes and weapons** on the NPC-server: these need an
//!   NC login (RC type 3). That is `seihai-rc` stage 5; the editor uses its
//!   library, pinned by tag, once it exists.
