# Content deployment

The games' content (their rules and data in Luau, docs/design/content-model-v2.md) is released apart from the
engine: a content update reaches players without an app release, and a replay plays on the content it was made
with.

## 1. Decisions (the user, 2026-10-26)

- **One repo per pack**, with the pack's history: `nettaisen/nettai-exe4`, `nettai-exe5`, `nettai-exe6` and
  `nettai-exelib`. The engine's API declarations (`content/nettai`) and `content/.luaurc` stay in the engine repo:
  they are the engine's.
- **The engine repo has them as git submodules** (`content/exe4`, `content/exe5`, `content/exe6`, `content/exelib`)
  for development, tests and tools. **An app release downloads the published bundles** and ships those.
- **Published on GitHub Pages**, each content repo its own site: static files, which the desktop app and the browser
  build both fetch (Pages serves them to any origin).
- **Content names its assets.** The names the extractor writes a pack's assets under (`compat/assets.toml`, the
  tables a chip's icon or a navi's emblem is found by) travel in the content, not compiled into the app: content that
  names a new asset has the app extract it from the Legacy Collection, as an app update does now.
- **Replays follow the engine's minor line.** A replay plays on its own content and an engine of its line (0.11.x),
  as now (docs/design/rollback.md; the minor version marks every change to the simulation, the state, the digest or
  the netplay protocol). Within a line, an engine runs every bundle published for the line: a patch release changes
  nothing content sees.

## 2. Repos

Each content repo's root is its pack's folder: `manifest.toml`, `init.luau`, the folders, `locales/`, `compat/`. Its
history is the pack's, kept through the folder's earlier names (`content/bn6` → `exe6`, `content/bn5` → `exe5`,
`content/common` → `exelib`; `git filter-repo`), so log and blame work there.

In the engine repo `content/` holds the four submodules, `content/nettai` and `content/.luaurc`; every path the
engine, its tests and the verification workspace use stays as it was. A change to the engine and its content is a
commit in the content repo, then the engine's commit that moves the submodule. A new worktree has no submodules
checked out: `git submodule update --init` first.

## 3. Bundles

A **bundle** is what a game plays: the game pack and the support packs it depends on, at fixed commits.

- **Its files:** `bundle.toml`, then each pack's folder under its id (`exe6/`, `exelib/`), with every file the pack's
  repo tracks.
- **`bundle.toml`:** the game; the engine line it is for (`line = "0.12"`); its revision (the game's publish count:
  `r1`, `r2`, ..., what a player sees); and each pack's commit.
- **Its form:** a deterministic tar (paths sorted, mode 0644, no owner, time 0), gzipped.
- **Its id:** the SHA-256 of the tar, in hex. It names the bundle everywhere: the store, replays, the lobby.

**Support packs.** A support pack isn't published on its own. A game's repo locks the commits of the support packs it
depends on (`depends.lock`: `exelib = "<commit>"`), and publishing the game takes them from their repos at those
commits. The engine repo's check refuses a game whose lock differs from its exelib submodule's commit, so what the
engine tests is what gets published.

## 4. Publishing

Each game's repo serves on Pages, from its `gh-pages` branch, to which publishing only adds:

- `index.json`: the pack and, by engine line, its bundles in publish order (id, revision, commits, date, size).
- `bundles/<id>.tar.gz`: each bundle.

**The workflow.** `publish.yml`, run by hand on main with the engine line, does four things in order:

1. It builds the bundle with the support packs at their locked commits.
2. It checks the bundle with the content SDK of that line. Each engine release attaches the SDK: the content checker
   and `content/nettai`'s declarations.
3. It refuses a bundle whose id is already the line's newest.
4. It commits the bundle and the new index to `gh-pages`.

A new engine line gets each game's content published for it, the same commit or a newer one.

## 5. The app

**The store.** Bundles are kept unpacked in the data folder's `content/<id>/`. The app also ships the newest bundle
of each game for its line, so it plays offline from the first start.

**Updates.** At start the app reads each game's `index.json`, unless Settings turned content updates off. It
downloads the newest bundle of its line on a thread, checks its id, and unpacks it. New matches use it once it is
down.

**Replays.** The replay's head names the game's bundle id beside the content hash. To play one, the app needs:

- an engine of the replay's line (as now);
- the bundle from the store, or downloaded by its id from the game's site.

If the bundle can't be had, the replays screen says which one is missing.

**Netplay.** The lobby's compatibility fields carry the bundle id. Peers whose ids differ both play the newer
revision: the peer without it downloads it. If it can't be had, the lobby refuses with that reason.

**Asset packs.** Packs are kept by the asset fingerprint the bundle's names give (`packs/<game>-<fingerprint>/`). A
bundle whose fingerprint has no pack has it extracted from the Legacy Collection when the game first loads. The
extractor reads the names from the bundle (`compat/assets.toml` and the tables it names), not from tables compiled
into it.

**Development.** A build run from the repository plays the submodules' content as now. Its bundle id is the same
function's, over those folders, so a replay made there names an id no site has. `$NETTAI_CONTENT` still names
another content directory.

**The browser.** The store is in the origin private file system, and bundles come from the sites. The page ships the
newest bundles as the desktop app does, in place of today's `content.tar`.

## 6. Steps

1. The four repos with their history, pushed.
2. The engine repo on submodules; CI, worktrees and the verification workspace's scripts initialize them.
3. The bundle (`nettai-content bundle`), its id, `depends.lock` and the engine's lock check; the content SDK in the
   engine's release; `publish.yml` and Pages in each game's repo.
4. The app: the store, content updates, the replay head's bundle id, the lobby's, packs by fingerprint, the
   extractor's names from the bundle, the browser's store.
5. `release.yml` ships the downloaded bundles; the first bundles are published for the line that step 4 starts (a
   new minor version: the replay head and the lobby change).
