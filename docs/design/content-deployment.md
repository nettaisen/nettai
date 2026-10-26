# Content deployment

The games' content (their rules and data in Luau, docs/design/content-model-v2.md) is released apart from the
engine: a content update reaches players without an app release, and a replay plays on the content it was made
with.

## 1. Decisions (the user, 2026-10-26)

- **One repo per pack**, named as the pack, with the pack's history: `nettaisen/exe4`, `exe5`, `exe6` and `exelib`.
  Their sites are at `https://nettai.n1gp.net/exe6/` and so on: the organization's Pages domain, which its github.io
  addresses redirect to (without the cross-origin header a browser needs, so the app asks the domain directly). The
  engine's API declarations (`content/nettai`) and `content/.luaurc` stay in the engine repo: they are the engine's.
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
- **`bundle.toml`:** the game; the engine line it is for (`line = "0.11"`); its revision (the game's publish count
  within the line: `r1`, `r2`, ..., what a player sees); and each pack's commit.
- **Its form:** a deterministic tar (paths sorted, mode 0644, no owner, time 0), gzipped.

**Two identities.**

- **The bundle id** is the SHA-256 of the tar, in hex. It names the files: the store keeps a bundle by it, a site
  serves it under it, and a download is checked against it.
- **The content hash** (`Content::hash`, what replays and the lobby already compare) says what a bundle plays. It is a
  hash of the content as the engine defines it with the game's asset pack, so only the engine computes it, and two
  bundles that differ only where play doesn't look (another language's text) share it.

Within a minor line every engine computes the same content hash for the same bundle and asset pack: anything netplay
compares is a minor-version change (docs/design/rollback.md).

**Support packs.** A support pack isn't published on its own. The engine checkout a game is published from pins it
(its submodule's commit), and `bundle.toml` records that commit.

## 4. Publishing

Each game's repo serves on Pages, from its `gh-pages` branch, to which publishing only adds:

- `index.json`: by engine line, the game's bundles in publish order. Each entry gives the id, the content hash, the
  revision, the packs' commits, the date and the size.
- `bundles/<id>.tar.gz`: each bundle.

**Publishing runs locally**, not in CI: computing the content hash needs the game's asset pack, which comes from the
ROMs, and no ROM goes to CI. `nettai-content publish <game>`, run in the engine checkout of the line, does six things
in order:

1. It refuses unless the game's and its support packs' submodules are clean and their commits are on their repos'
   main branches.
2. It type-checks the content.
3. It builds the bundle from the submodules.
4. It computes the content hash with the local asset pack.
5. It refuses a bundle whose content hash and id are already the line's newest.
6. It adds the bundle and the new index entry to the game repo's `gh-pages` branch, then commits and pushes.

A new engine line gets each game's content published for it, from the same commits or newer ones.

## 5. The app

**The store.** Bundles are kept unpacked in the data folder's `content/<id>/`, with a note of each one's content hash.
The app also ships the newest bundle of each game for its line, so it plays offline from the first start.

**Updates.** At start the app reads each game's `index.json`, unless Settings turned content updates off. It
downloads the newest bundle of its line on a thread, checks its id, unpacks it, and checks that the content hash it
computes is the index's. New matches use it once it is down.

**Replays.** A replay names its content hash, as now. To play one, the app needs:

- an engine of the replay's line (as now);
- a bundle with that content hash: from the store, or found in the game's index and downloaded by its id.

If there's none, the replays screen says the content isn't available.

**Netplay.** The lobby compares content hashes, as now. Peers whose hashes differ both play the newer revision: the
peer without it finds it in the index by the other's content hash and downloads it. If it can't be had, the lobby
refuses with that reason.

**Asset packs.** Packs are kept by the asset fingerprint the bundle's names give (`packs/<game>-<fingerprint>/`). A
bundle whose fingerprint has no pack has it extracted from the Legacy Collection when the game first loads. The
extractor reads the names from the bundle (`compat/assets.toml` and the tables it names), not from tables compiled
into it.

**Development.** A build run from the repository plays the submodules' content as now. `$NETTAI_CONTENT` still names
another content directory.

**The browser.** The store is in the origin private file system, and bundles come from the sites. The page ships the
newest bundles as the desktop app does, in place of today's `content.tar`.

None of this changes the replay format or the lobby's messages: an app with the store is a patch release of its line.

## 6. Steps

1. The four repos with their history, pushed.
2. The engine repo on submodules; CI, worktrees and the verification workspace's scripts initialize them.
3. The bundle format and id (`nettai_content::bundle`), `nettai-content publish`, and Pages on each game's repo.
4. The app: the store, content updates, replays and the lobby finding bundles by content hash, packs by fingerprint,
   the extractor's names from the bundle, the browser's store.
5. `release.yml` ships the newest published bundles instead of `content/`; each game's content is published for the
   line.
