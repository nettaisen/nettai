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
  for development, tests and tools. **An app release ships no content**: the app downloads the published bundles,
  the first start included (§5; this replaced shipping the newest bundles with each release).
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
  hash of the content as the engine defines it, so only the engine computes it, and two bundles that differ only where
  play doesn't look (another language's text) share it. It is the bundle's alone: the assets' handles and the
  sprites' timing are content (content-model-v2.md §6.3), so no asset pack, nothing from a ROM, goes into it.

Within a minor line every engine computes the same content hash for the same bundle: anything netplay compares is a
minor-version change (docs/design/rollback.md).

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

**Publishing runs locally.** It once had to: the content hash needed the game's asset pack, which comes from the ROMs,
and no ROM goes to CI. Since 2026-10-27 a bundle's hash needs nothing but the bundle (above), so publishing could move
to CI; it hasn't yet. `nettai-content publish <game> [--dry-run]`, run in the engine checkout of the line with `gh`
signed in, does six things in order:

1. It refuses unless the game's and its support packs' submodules are clean (nothing untracked or ignored either: a
   bundle holds the folder), at the commits the engine's HEAD pins, and on their repos' main branches (fetched
   first), and unless the game's lock names its support packs' commits.
2. It type-checks the game and its support packs and checks the lock: `nettai-content-check --pack exe6 --pack
   exelib`, run with cargo (its Luau can't be linked beside the engine's).
3. It builds the bundle from the submodules, the support packs at the locked commits. The line is the engine's major
   and minor version (the workspace's; the program and the checkout must share it). The revision is one past the
   count of the line's releases, which must be r1 to rN.
4. It computes the content hash: it unpacks the bundle as the app's store does and loads its game (the content
   alone).
5. It refuses a bundle that is the line's newest again: the same id, or the same content hash from the same commits
   (the newest release's `bundle.json`). The same content from other commits is a new revision.
6. It creates the release (`gh release create`), showing the command first. `--dry-run` does all but run it, and
   leaves the release's files in `target/publish/<game>/<tag>/` for the command it shows.

A release stays: replays made with its content find it by its content hash. A mistake is mended by the next
revision. A new engine line gets each game's content
published for it, from the same commits or newer ones.

## 5. The app

**The catalog.** The app knows one address: the catalog, `https://nettai.n1gp.net/catalog.json`
(`bundle::catalog_url`, `bundle::Catalog`), a map of each game's pack id to the full address of its site's
`index.json` (and, as `ai`, nettai-ai's index). Every other address is data: the catalog gives each game's index, and
each index entry the full address of its bundle (`Published::url`). The app's games are the catalog's: a bundle of
the app's line holds its game's extraction recipe (extraction-recipes.md), and the app names no game.

**The store.** Bundles are kept unpacked in the data folder's `content/<id>/` (`nettai_content::store`), each with its
record beside it, `content/<id>.toml`: its game, line and revision, the content hash its index gives it
(`published`), and the one computed here (`hash`), recorded the first time the game is loaded from it. A folder
without its record, a record without its folder, or a folder being unpacked (`<id>.part`) is what a storing cut short
left, and goes as the app starts. Beside the bundles the store keeps the catalog and the indexes as last read
(`catalog.json`, `<game>.index.json`), so a start without a connection plays what the store has. A game plays the
newest revision of the app's line in the store.

**Downloads.** The app ships no content. At start, on the window's loop: the catalog is read, then each game's index,
and the newest bundle of the line is downloaded where the store hasn't it, the game shown first; a download is checked
against its id as it is unpacked. A game the store has no content of yet is downloaded whatever Settings say, the
first start included: without it there is nothing to play. Settings' "Check for updates" off stops only the look for
newer revisions of the games that have some. While a game's content downloads, the loading screen's bar is the
download's ("DOWNLOADING BN6", where "EXTRACTING BN6" shows); a game whose download fails is a dialog ("BN6's content
couldn't be downloaded. Check your connection, then start nettai again."), or the loading screen's note when no game
has content. A game whose index has nothing for the line isn't listed, and where no game has content for that reason
the note says none is out yet ("There's no game content for this version of nettai yet. Check back later."). A game
that is loaded when its newer content arrives plays it from the next time the main menu shows with nothing over
it.

**What a bundle plays.** The first time a game is loaded from a bundle, the content hash computed is checked against
the index's. A bundle that plays another is taken out of the store, the reason logged, and the game
loads its next newest bundle (or, with none, can't be played, said as a load failure).

**Replays.** A replay names its content hash, as before. The Replays screen plays one on the game's own content where
the hashes match; else on the store's bundle that plays its hash, or on the bundle the game's index lists for it,
downloaded while the replay is selected (the index is read again, once a visit, where it lists no such hash). The
game is loaded on that content beside its own (its pack extracted where its fingerprint has none, its parts read on a
thread, its runtime), and the replay's still, its play-out, the viewer, its thumbnails and its export use it. A replay
whose content the store hasn't and no index lists shows "The content this replay was made with isn't available, so
it can't be played." in its row.

**Netplay.** The lobby compares content hashes, as before; but a lobby that waits on content (`Lobby::wait_on_content`:
the app's, but development's) doesn't refuse other content at once. While the hashes differ it takes nothing of the
other's lobby, and tells its host the other's hash (`Lobby::their_content`). Both play the newer revision: each side
finds the other's revision in its store or the game's index (read again where it isn't listed). The side with the
older one downloads the newer, loads the game again on it behind the bar, and gives the lobby the new content and the
build made again of it (`Lobby::replace_content`); the side with the newer one waits. Content no index lists, the same
revision of other content, or development's content (it has no revision) is refused for its content
(`Lobby::refuse_content`: `Reason::DifferentContent` on both sides).

The background pool (every game's backgrounds, which a round may be drawn on: `Compat::backgrounds`, its identity)
isn't in the hash, and comes from every game's content, not only the one played; peers must agree on it all the same.
A waiting lobby doesn't refuse another pool at once either. While the pools differ (the hashes the same) it takes
nothing of the other's lobby, and tells its host the other's pool (`Lobby::their_backgrounds`). Both sides then take
every game's newest revision of the line, whatever Settings say (`contents::fetch_every_newest`: the catalog and each
game's index read again, a bundle downloaded where the store hasn't it), as a game with no content, a replay's
content and the played game's are; this covers a side whose "Check for updates" is off, or which started offline.
Where that changed the played game's content or the pool, the game is loaded again on it behind the bar (the pool is
read as a game loads) and the lobby given the new content and the build made again of it (`Lobby::replace_content`).
Then the host tells the lobby what it couldn't have (`Lobby::updated_backgrounds`), and the lobby compares again. Both
sides do this, so both come to the same pool without telling each other which game differs: the side that already had
everything changes nothing and waits for the other. While the other's pool stays the one a side took the newest
content against, that side waits for it (the other may still be downloading), refusing nothing; a pool of theirs that
changes and still differs is taken against again (the sites may have published since). Where a side couldn't have
something (the catalog, or a game's bundle) and the pools still differ, its lobby refuses for that, the reason
naming what couldn't be had (`Reason::DifferentContent` on both sides). Two sides that each have every game's newest
by their own reading of the sites and still differ (one read an index a cache hadn't caught up on) wait until a
player leaves. Replays need none of this: each round's background is recorded by name.

Nothing on the wire changes: each peer says its own content and pool as always, and a peer that doesn't wait refuses
as before.

**Asset packs.** Packs are kept by their content's extraction recipe's fingerprint (`packs/<game>-<fingerprint>/`,
`nettai_extract::Recipe::fingerprint`): the engine's line, the pack formats, and every file the recipe reads (its
`extract/` and its support packs', and its game's compat tables: the names the assets are written under). Content whose fingerprint has no pack has it extracted from the Legacy Collection when the game first loads,
and so does other content a replay plays on. The recipe is read from the content (`<game>/extract`,
`<game>/compat`), not compiled into the app: the compat crates' built-in tables are their `builtin` feature's, for
tools and tests, and the app builds without the content submodules.

**Development.** A build run from the repository plays every game from the submodules' content directory, as before,
and downloads nothing for it; `$NETTAI_CONTENT` names another. Other content (a replay's) is found in the store and
the sites all the same. `$NETTAI_CATALOG` names another catalog than the organization's (absent in normal use): a
site of indexes and bundles served from elsewhere, as the tests serve one on the machine.

**The browser.** The page ships no content either: its store is the data folder's, which the page keeps in the origin
private file system, and the bundles come from the sites (asked of the Pages domain directly, which serves them to any
origin).

None of this changes the replay format or the lobby's messages: an app with the store is a patch release of its line.

## 6. Steps

1. The four repos with their history, pushed.
2. The engine repo on submodules; CI, worktrees and the verification workspace's scripts initialize them.
3. The bundle format and id (`nettai_content::bundle`), `nettai-content publish`, and Pages on each game's repo.
4. The app: the catalog, the store and its downloads, replays and the lobby finding bundles by content hash, packs by
   fingerprint, the extractor's names from the content, the browser's store.
5. The packages (`release.yml`, packaging/) and the browser's page (crates/nettai-web/build.sh) ship no content: the
   app downloads each game's, the first start included, and the release's build jobs check out no submodules. Each
   game's content is published for the line.
