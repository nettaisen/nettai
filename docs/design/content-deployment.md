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
  repo tracks but its workflows (`.github/`, which build the site, §4).
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

**Support packs.** A support pack isn't published on its own: a bundle holds it at the commit its game locks.

- **`depends.lock`**, at the root of each game's repo, names the commit of each support pack the game goes with, the
  one it was checked against: one line per support pack it depends on (and theirs), with the full commit
  (`exelib = "a73907b5ec9bfdb63116ae0ed025ad9550403a92"`). It is no content: nothing loads it, and it goes in the
  game's bundle with the pack's other files. `bundle.toml` records the same commits.
- **The check.** `nettai-content-check` refuses a game whose lock isn't the commit its support packs' submodules are
  at, naming both commits and the fix; where a support pack is no git checkout of its own (an unpacked bundle's
  folder) it says so and leaves the lock unchecked. Publishing refuses such a game too, so a game is never bundled
  with an exelib it wasn't checked against.

**Changing exelib.**

1. Commit the change in exelib's repo, and push it to main.
2. Move the engine's `content/exelib` submodule to it.
3. Check each game against it (`cargo run -p nettai-content-check`, the tests). The check names every game's lock
   as stale until step 4; the rest of its report is the games' with the new exelib.
4. `nettai-content lock` (or `lock exe6` for one game) rewrites each game's `depends.lock` to `content/exelib`'s
   commit. It only writes the files: commit each in its game's repo, and push it to main.
5. Move the engine's game submodules to those commits, in one engine commit with exelib's.

## 4. Publishing

A game's bundles are **releases of its repo**, and its site is built from them. No bundle enters git history: every
engine checkout's submodule clone fetches all of a repo's branches, so a branch of bundles would reach every
developer's disk.

- **A release** is tagged `<line>-r<revision>` (`0.11-r1`) on the game pack's commit the bundle holds, titled
  `exe6 r1 for 0.11`, with two files: `<id>.tar.gz`, the bundle, and `bundle.json`, its index entry
  (`bundle::Published`: the id, the content hash, the revision, the packs' commits, the date and the size).
- **The site** (`https://nettai.n1gp.net/exe6/`, §1) is built from all the repo's releases by its Pages workflow,
  `.github/workflows/pages.yml` (the same in each game's repo), each time a release is published:
  - `index.json` (`bundle::Index`): the game, and its bundles by engine line, each line's in revision order, from
    the releases' `bundle.json`;
  - `bundles/<id>.tar.gz`: each bundle, checked against its id (the SHA-256 of its tar) and its entry's size. A
    release that fails a check fails the build, and the site stays as it was.

  Pages' source is GitHub Actions. The site deploys from main alone (the `github-pages` environment's rule), and a
  release's run is on its tag, so that run only starts the workflow again on main (`gh workflow run`), which builds
  and deploys. A release's run is the workflow its tagged commit holds, so a release of a commit without
  `pages.yml` starts nothing; `gh workflow run pages.yml -R nettaisen/<game>` builds the site by hand.

**Publishing runs locally**, not in CI: computing the content hash needs the game's asset pack, which comes from the
ROMs, and no ROM goes to CI. `nettai-content publish <game> [--packs DIR] [--dry-run]`, run in the engine checkout
of the line with `gh` signed in, does six things in order:

1. It refuses unless the game's and its support packs' submodules are clean (nothing untracked or ignored either: a
   bundle holds the folder), at the commits the engine's HEAD pins, and on their repos' main branches (fetched
   first), and unless the game's lock names its support packs' commits.
2. It type-checks the game and its support packs and checks the lock: `nettai-content-check --pack exe6 --pack
   exelib`, run with cargo (its Luau can't be linked beside the engine's).
3. It builds the bundle from the submodules, the support packs at the locked commits. The line is the engine's major
   and minor version (the workspace's; the program and the checkout must share it). The revision is one past the
   count of the line's releases, which must be r1 to rN.
4. It computes the content hash: it unpacks the bundle as the app's store does and loads it with the game's asset
   pack (`--packs DIR`; the checkout's `data/<game>` by default).
5. It refuses a bundle that is the line's newest again: the same id, or the same content hash from the same commits
   (the newest release's `bundle.json`). The same content from other commits is a new revision.
6. It creates the release (`gh release create`), showing the command first. `--dry-run` does all but run it, and
   leaves the release's files in `target/publish/<game>/<tag>/` for the command it shows.

A release stays: replays made with its content find it by its content hash. A mistake is mended by the next
revision. A new engine line gets each game's content
published for it, from the same commits or newer ones.

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
