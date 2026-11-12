# Runbook: client reach against real clients (#38)

`check --client-reach` reports what each client era will miss: PNG art with
no GIF copy (#57 writes the copies), gmaps and level formats (#55), and
script era (#58). This runbook checks those reports against real clients.
Tiles and weapon images are not reported yet.

**Blocked first on a decision.** #58 places the GS1/GS2 cut at the first
4.x profile in seihai-core (4.04r2), because seihai-core has no field that
says which script engines a version runs. Whether that should be a real
profile field, and where the cut is, is the owner's call; see the triage
note. Until then, step 3 checks the 4.x assumption itself.

## Steps (loopback, the lab world from 47-real-fork.md)

1. Make a world with: a level NPC using `.png` art with no `.gif`; the same
   after `check --make-gif`; a GS1-only NPC; a GS2-only NPC; a gmap.
2. `seihai-servertools check --client-reach WORLD` and keep the output.
3. Log in with one client from each era you have (for example 1.41, 2.x,
   3.x, 4.x, 5.x, 6.x) and record what each draws and runs.

| Item | Client | Report says | Client did | Match |
|---|---|---|---|---|
| PNG, no GIF | 1.41 | missing | | |
| PNG + made GIF | 1.41 | ok | | |
| GS2-only NPC | 3.x | won't run | | |
| GS2-only NPC | 4.x | runs | | |
| gmap | 1.41 | single levels | | |

The 3.x and 4.x GS2 rows are the ones that test the cut. If a 4.x client
below some build does not run GS2, record the build: it is the evidence for
the profile field.
