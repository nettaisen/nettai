# Working on content

Each game's content is Luau in its own repository, checked out here as a git submodule:

| Folder | Repository | What it is |
| --- | --- | --- |
| `content/exe4` | [nettaisen/exe4](https://github.com/nettaisen/exe4) | Battle Network 4 |
| `content/exe5` | [nettaisen/exe5](https://github.com/nettaisen/exe5) | Battle Network 5 |
| `content/exe6` | [nettaisen/exe6](https://github.com/nettaisen/exe6) | Battle Network 6 |
| `content/exelib` | [nettaisen/exelib](https://github.com/nettaisen/exelib) | Support code the games share |
| `content/nettai` | (this repository) | The engine's API declarations (`core.d.luau`) |

Clone with `git clone --recursive`, or run `git submodule update --init` in an existing clone or a new worktree.
The content model is described in [design/content-model-v2.md](design/content-model-v2.md), and how content is
released in [design/content-deployment.md](design/content-deployment.md).

## Checking content

```sh
cargo run -p nettai-content-check                      # every pack: types, lints, requires, depends.lock
cargo run --release -p nettai-content -- check exe6    # a game's assets, extracted from the ROMs, against its content
```

`nettai-content-check` type-checks each pack against its declarations (the engine's API in
`content/nettai/core.d.luau`, then its support packs', then its own), lints it, and checks that its manifests and
requires name only modules that exist, that a pack requires only itself and the support packs it uses, and that each
game's `depends.lock` names its support packs' commits. It needs no ROMs. `nettai-content check` reads the ROMs from
`$NETTAI_ROMS` (see the README).

## Custom assets

A game can have backgrounds, sprites and songs of its own, beside the ones extracted from its ROMs: pictures as
indexed PNGs in the GBA's colors, songs as MIDI files (with WAV recordings for instruments). One given the name of
a ROM asset replaces it wherever that name is used (a reskin). A game's backgrounds and music are presentation, so
they are stated in two files of its folder, not in its modules, and changing them leaves the content's hash (what
two players compare to play together) as it was: `backgrounds.toml` (the background its link battle stages are
drawn on, those its link battles pick one of at random, and each background's music) and `music.toml` (the battle
themes a player may pick, and songs of its own). A background is 16 colors, each side a multiple of 8 pixels:

```toml
# content/exe6/backgrounds.toml
[backgrounds.my-stage]
file = "backgrounds/my_stage.png"   # its path in the game's folder
scroll = [16, 8]
music = "virus-battle"              # one of music.toml's themes
```

`speed_up = 120` has the scroll start still and reach its speed in 120 frames, and `frames = [8, 8, 8]` with
`loop = true` animates it: the PNG is that many pictures stacked top to bottom, each shown for its frames (each
picture may use its own 16 colors). To offer it in the background pool, name it in the game's `locales/en.toml`
under `[backgrounds]`. `nettai-content check` then loads it with the game's assets.

A sprite is a sheet of equal cells, one per frame in the order its animations list them, color 0 clear:

```luau
-- content/exe6/objects/drone/drone_sprite.luau
return asset.sprite("drone", { { 4, 4, 4, 4, loop = true }, { 3, 3 } }, {
    file = "./drone.png",            -- 6 cells: the first animation's 4, then the second's 2
    frame = { 32, 32 },              -- each cell's size (left out: one row of cells)
    origin = { 16, 30 },             -- where the object stands in a cell (left out: the bottom middle)
    shadow = "./drone_shadow.png",   -- optional: drawn on the ground under it
})
```

Whatever shows it requires that module, as with the ROMs' sprites.

A song is a MIDI file played on the instruments of one of the game's songs, read the way the GBA's mid2agb reads
one (a program change picks that song's instrument of that number), stated in the game's `music.toml`:

```toml
# content/exe6/music.toml
themes = ["virus-battle", "boss-battle", "my-theme"]   # what a player may pick as a battle's music

[songs.my-theme]
file = "music/my_theme.mid"
voices = "virus-battle"                               # the ROMs' song whose instruments it plays on
```

A background plays it with `music = "my-theme"` in its `backgrounds.toml` entry (a background's music is one of the
themes). Markers named `[` and `]` in the MIDI file make it loop. To use recordings of your own, put them in place
of programs:

```toml
[songs.my-theme.instruments.41]
file = "music/strings.wav"
key = 69
loop = 1024
envelope = { release = 200 }
```

where `key` is the note the recording sounds at (60 is middle C) and `loop` the sample it loops back to. The details are in [design/asset-formats.md](design/asset-formats.md) §7 and §8.

## Editing in Zed

Install the [Luau extension](https://github.com/4teapo/zed-luau), open the repository root, and generate the
editor's type definitions once:

```sh
python3 .zed/generate-luau-definitions.py
```

The project settings load the generated `.zed/luau-globals.d.luau` (gitignored), use the new type solver, and select
the standard Luau platform. The engine's and the packs' declarations reference each other, so they must be loaded
together; passing each `.d.luau` to the server separately leaves unresolved types. The sources use
`declare extern type`, which current luau-lsp and the content checker both understand. The launch arguments load
only the combined file; `types.definitionFiles` also lists the source declarations so the server treats them as
definitions when they are opened.

After editing a source `.d.luau`, run the Zed task **Luau: regenerate editor definitions** (or the command above),
then **editor: restart language server** from the command palette with a Luau file open. `content/.luaurc` supplies
the pack require aliases.

The editor loads every pack's declarations into one shared environment, so `nettai-content-check` stays the check
for each pack's permitted dependencies. The editor does follow imports, though, and can report cross-module type
errors that the checker (which types `require` as `any`) does not. A module with a same-named folder belongs in that
folder's `init.luau`: luau-lsp resolves the folder before a sibling `.luau` file.

To check every module with import resolution from the command line, regenerate the definitions and run luau-lsp
(on your PATH, or the full path to Zed's downloaded binary):

```sh
luau-lsp analyze --platform=standard --flag:LuauSolverV2=true \
  --definitions=.zed/luau-globals.d.luau --ignore='**/*.d.luau' content
```

## Publishing

The app downloads each game's content from the game's site (`https://nettai.n1gp.net/exe6/` and so on), not from
this repository. To publish a game's content for this engine's version line, once the game's and exelib's
repositories have the commits on main and this checkout's submodules point at them:

```sh
cargo run --release -p nettai-content -- publish exe6 --dry-run   # check, bundle and hash it; show the release
cargo run --release -p nettai-content -- publish exe6             # make the release
```

This needs `gh` signed in with write access to the game's repository. No ROM is read: the content hash covers the
content alone. The release (`0.16-r1`, `0.16-r2`, ...) holds the bundle, and the repository's Pages workflow then
puts it on the site. Publishing refuses a dirty or unpushed submodule, content that doesn't type-check, and a bundle
identical to the line's newest. The full procedure is in
[design/content-deployment.md](design/content-deployment.md) §4.

### Changing exelib

Each game's repository locks the exelib commit it goes with (`depends.lock`), and `nettai-content-check` refuses a
game whose lock doesn't match `content/exelib`. To change exelib:

1. Push the change to exelib's main and move `content/exelib` here to it.
2. Check the games against it (`cargo run -p nettai-content-check`).
3. Update each game's lock:

   ```sh
   cargo run -p nettai-content -- lock
   ```

4. Commit and push each game's `depends.lock` in its own repository, then move the game submodules here along with
   exelib's.
