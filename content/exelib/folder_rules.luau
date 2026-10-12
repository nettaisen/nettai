--!strict
-- The folder rules EXE4, EXE5 and EXE6 share, as each game's folder editor enforces them and a link battle checks them
-- again before it starts: a tool's questions (a match's checks, netplay's offer, the build screens), never the
-- simulation's. Each game's rules/folder states what is its own (`FolderRules`, types.d.luau: which chips a folder can
-- hold, which a save of the side's version can, how many copies, its dark chips, its tag chips) and cites its
-- routines. The limits are the navi's stats as the round set them up (the NaviCust's and the patch cards' work: the
-- reload's, which the editor and the link battle read).
--
-- A rule broken is a problem a tool words itself (`SetupProblem`, core.d.luau): its kind and numbers, with a sentence
-- for a log.
--
-- - `folder_size`: a folder is its size in chips (of a whole folder).
-- - `chip_not_allowed`: a chip no folder holds.
-- - `chip_of_another_version`: a chip the side's save can't hold (another version's).
-- - `chip_code`: each chip in one of its codes (a battle counts any other as the invalid chip).
-- - `copies`: of one chip, in any codes.
-- - `mega`, `giga`: at most the navi's Mega and Giga levels (NaviStats +0x0B, +0x0C). A dark chip counts as dark alone.
-- - `dark`: at most the game's dark chips.
-- - `regular_memory`: the Regular chip's MB at most the navi's Regular memory (+0x09).
-- - `tag_memory`: the two tag chips' MB together at most the game's tag memory; `tag_regular`: neither is the Regular
--   chip; `no_tags`: a game without tag chips.
-- - `regular_entry`, `tag_entries`: the Regular and tag chips are entries with a chip, the tag chips two of them.

local folder_rules = {}

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local function name(c: Chip): string
    return c.id :: string
end

local function mb(c: Chip): number
    return c.mb or 0
end

local function is_dark(c: Chip): boolean
    return has(c.flags, "dark")
end

-- What a folder holds against the limits: its Mega, Giga and dark chips (a dark chip is counted as dark alone), and
-- its copies of each chip.
type Counts = { mega: number, giga: number, dark: number, copies: { [Chip]: number }, order: { Chip } }

local function count(chips: { { chip: Chip, code: string } }): Counts
    local counts: Counts = { mega = 0, giga = 0, dark = 0, copies = {}, order = {} }
    for _, e in chips do
        local c = e.chip
        if counts.copies[c] == nil then
            table.insert(counts.order, c)
        end
        counts.copies[c] = (counts.copies[c] or 0) + 1
        if is_dark(c) then
            counts.dark += 1
        elseif c.class == "mega" then
            counts.mega += 1
        elseif c.class == "giga" then
            counts.giga += 1
        end
    end
    return counts
end

-- The rules folder `f` breaks (a folder as `battle.checked_folder()` gives one: its chips, its Regular and tag chips
-- by their place among them, from 1), by `game`'s rules, the limits `stats`' and the save `setup`'s: each a problem,
-- its `entry` a place among the folder's chips.
function folder_rules.problems(game: FolderRules, stats: Navi, setup: any, f: CheckedFolder): { SetupProblem }
    local out: { SetupProblem } = {}
    local function say(p: SetupProblem)
        table.insert(out, p)
    end
    local chips = f.chips
    if f.complete and #chips ~= game.size then
        say({
            kind = "folder_size",
            used = #chips,
            limit = game.size,
            text = string.format("the folder has %d chips; a folder is %d", #chips, game.size),
        })
    end
    for i, e in chips do
        local c = e.chip
        if not game.in_pack(c) then
            say({
                kind = "chip_not_allowed",
                entry = i,
                chip = c,
                text = string.format("entry %d: %s is no chip a folder can hold", i - 1, name(c)),
            })
        elseif game.owned ~= nil and not game.owned(c, setup) then
            say({
                kind = "chip_of_another_version",
                entry = i,
                chip = c,
                text = string.format("entry %d: %s is a chip of another version's saves", i - 1, name(c)),
            })
        end
        if not has(c.codes, e.code) then
            say({
                kind = "chip_code",
                entry = i,
                chip = c,
                code = e.code,
                text = string.format("entry %d: %s doesn't come in code %s", i - 1, name(c), e.code),
            })
        end
    end
    local counts = count(chips)
    for _, c in counts.order do
        local n, allowed = counts.copies[c], game.copies(c)
        if n > allowed and game.in_pack(c) then
            say({
                kind = "copies",
                chip = c,
                used = n,
                limit = allowed,
                text = string.format("%d copies of %s, past %d", n, name(c), allowed),
            })
        end
    end
    if counts.mega > stats.mega_level then
        say({
            kind = "mega",
            used = counts.mega,
            limit = stats.mega_level,
            text = string.format("%d Mega chips, past the navi's %d", counts.mega, stats.mega_level),
        })
    end
    if counts.giga > stats.giga_level then
        say({
            kind = "giga",
            used = counts.giga,
            limit = stats.giga_level,
            text = string.format("%d Giga chips, past the navi's %d", counts.giga, stats.giga_level),
        })
    end
    if game.dark ~= nil and counts.dark > game.dark then
        say({
            kind = "dark",
            used = counts.dark,
            limit = game.dark,
            text = string.format("%d dark chips, past %d", counts.dark, game.dark),
        })
    end
    if f.regular ~= nil then
        local e = chips[f.regular]
        if e == nil then
            say({
                kind = "regular_entry",
                text = string.format("the Regular chip is entry %d, past the folder", f.regular - 1),
            })
        elseif mb(e.chip) > stats.regular_memory then
            say({
                kind = "regular_memory",
                entry = f.regular,
                chip = e.chip,
                used = mb(e.chip),
                limit = stats.regular_memory,
                text = string.format(
                    "the Regular chip %s is %d MB, past the navi's %d",
                    name(e.chip),
                    mb(e.chip),
                    stats.regular_memory
                ),
            })
        end
    end
    if f.tags ~= nil then
        local a, b = f.tags[1], f.tags[2]
        local x, y = chips[a], chips[b]
        local tag_memory = game.tag_memory
        if tag_memory == nil then
            say({ kind = "no_tags", text = "tag chips, but the game's folders have none" })
        elseif a == b then
            say({ kind = "tag_entries", text = string.format("the tag chips are both entry %d", a - 1) })
        elseif x == nil or y == nil then
            say({
                kind = "tag_entries",
                text = string.format("the tag chips are entries %d and %d, past the folder", a - 1, b - 1),
            })
        else
            local together = mb(x.chip) + mb(y.chip)
            if together > tag_memory then
                say({
                    kind = "tag_memory",
                    used = together,
                    limit = tag_memory,
                    text = string.format(
                        "the tag chips %s and %s are %d MB, past %d",
                        name(x.chip),
                        name(y.chip),
                        together,
                        tag_memory
                    ),
                })
            end
            if f.regular == a or f.regular == b then
                say({ kind = "tag_regular", text = "the Regular chip is a tag chip" })
            end
        end
    end
    return out
end

-- What `game`'s rules allow of folder `f` (the chips so far), the limits `stats`' and the save `setup`'s: the folder's
-- size and its chips, its Mega, Giga and dark chips against their limits, the Regular and tag memory, and each chip
-- offered (`f.offered`) that can't go into it now with why (the first rule one more copy would break: no folder holds
-- it, another version's, the folder is whole, its copies, its class's limit), `used` what the folder holds now.
function folder_rules.view(game: FolderRules, stats: Navi, setup: any, f: CheckedFolder): FolderView
    local counts = count(f.chips)
    local full = #f.chips >= game.size
    -- Why chip `c` can't go into the folder now; nil: it can.
    local function refusal(c: Chip): SetupProblem?
        local held = counts.copies[c] or 0
        local dark_limit = game.dark
        if not game.in_pack(c) then
            return { kind = "chip_not_allowed", chip = c, text = string.format("%s is no chip a folder can hold", name(c)) }
        elseif game.owned ~= nil and not game.owned(c, setup) then
            return {
                kind = "chip_of_another_version",
                chip = c,
                text = string.format("%s is a chip of another version's saves", name(c)),
            }
        elseif full then
            return {
                kind = "folder_size",
                chip = c,
                used = #f.chips,
                limit = game.size,
                text = string.format("the folder has its %d chips", game.size),
            }
        elseif held >= game.copies(c) then
            return {
                kind = "copies",
                chip = c,
                used = held,
                limit = game.copies(c),
                text = string.format("%d copies of %s already, of %d", held, name(c), game.copies(c)),
            }
        elseif is_dark(c) then
            if dark_limit ~= nil and counts.dark >= dark_limit then
                return {
                    kind = "dark",
                    chip = c,
                    used = counts.dark,
                    limit = dark_limit,
                    text = string.format("%d dark chips already, of %d", counts.dark, dark_limit),
                }
            end
        elseif c.class == "mega" and counts.mega >= stats.mega_level then
            return {
                kind = "mega",
                chip = c,
                used = counts.mega,
                limit = stats.mega_level,
                text = string.format("%d Mega chips already, of the navi's %d", counts.mega, stats.mega_level),
            }
        elseif c.class == "giga" and counts.giga >= stats.giga_level then
            return {
                kind = "giga",
                chip = c,
                used = counts.giga,
                limit = stats.giga_level,
                text = string.format("%d Giga chips already, of the navi's %d", counts.giga, stats.giga_level),
            }
        end
        return nil
    end
    local refused: { SetupProblem } = {}
    local offered: { Chip } = f.offered or {}
    for _, c in offered do
        local problem = refusal(c)
        if problem ~= nil then
            table.insert(refused, problem)
        end
    end
    local dark: FolderCount? = nil
    if game.dark ~= nil then
        dark = { used = counts.dark, limit = game.dark }
    end
    return {
        size = game.size,
        chips = #f.chips,
        mega = { used = counts.mega, limit = stats.mega_level },
        giga = { used = counts.giga, limit = stats.giga_level },
        dark = dark,
        regular_memory = stats.regular_memory,
        tag_memory = game.tag_memory,
        refused = refused,
    }
end

-- A game's `folder_check`: the folder a tool asks of (`battle.checked_folder()`), each rule it breaks; `setup` the
-- side's setup, which its game's module hands over.
function folder_rules.check(game: FolderRules, side: number, setup: any): { SetupProblem }?
    local f = battle.checked_folder()
    if f == nil or f.side ~= side then
        return nil
    end
    return folder_rules.problems(game, battle.navi(side), setup, f)
end

-- A game's `folder_view`: what its rules allow of the folder a tool asks of.
function folder_rules.checked_view(game: FolderRules, side: number, setup: any): FolderView?
    local f = battle.checked_folder()
    if f == nil or f.side ~= side then
        return nil
    end
    return folder_rules.view(game, battle.navi(side), setup, f)
end

-- A game's `validate`'s part: the side's own folder (its `setup`'s `folder`, `regular_chip` and, where the game has
-- them, `tag_chips`), each problem said with `problem`, of the setup's `folder` and its entry there: its Regular and
-- tag chips are entries with a chip, and it keeps the rules.
function folder_rules.validate(game: FolderRules, side: number, setup: any, problem: (p: SetupProblem) -> ())
    -- The entries with a chip, each one's place among them (from 1) by its entry (from 0), and back.
    local chips: { { chip: Chip, code: string } } = {}
    local place: { [number]: number } = {}
    local entry: { [number]: number } = {}
    for i, e in setup.folder do
        local chip: Chip?, code: string? = e.chip, e.code
        if chip ~= nil and code ~= nil then
            table.insert(chips, { chip = chip, code = code })
            place[i - 1] = #chips
            entry[#chips] = i
        end
    end
    local regular: number? = nil
    if setup.regular_chip ~= nil then
        regular = place[setup.regular_chip]
        if regular == nil then
            problem({ kind = "regular_entry", field = "folder", text = "folder: the Regular chip isn't a chip of the folder" })
        end
    end
    local tags: { number }? = nil
    local t: { number } = if game.tag_memory ~= nil then setup.tag_chips else {}
    if #t == 2 then
        if place[t[1]] == nil or place[t[2]] == nil then
            problem({ kind = "tag_entries", field = "folder", text = "folder: the tag chips aren't chips of the folder" })
        else
            tags = { place[t[1]], place[t[2]] }
        end
    end
    local f: CheckedFolder = { side = side, chips = chips, regular = regular, tags = tags, complete = true }
    for _, p in folder_rules.problems(game, battle.navi(side), setup, f) do
        p.field = "folder"
        p.text = "folder: " .. p.text
        if p.entry ~= nil then
            p.entry = entry[p.entry]
        end
        problem(p)
    end
end

return folder_rules
