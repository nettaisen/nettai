--!strict
-- EXE5's folder rules, as its folder editor enforces them and a link battle checks them again before it starts: a
-- tool's question (`folder_check`: a match's checks, netplay's offer, the editor, live play's random folder), never
-- the simulation's, as EXE6's (its rules/folder/init). The folder is `battle.checked_folder()`; each rule it breaks
-- is a `battle.folder_problem(rule, text)`. The limits are the navi's stats as the round set them up. Team ProtoMan
-- US's addresses; Team Colonel's routines are the same code, and so is its table of copies.
--
-- - `size`: a folder is 30 chips (the editor's list of a folder is its 30 entries, 0x081293A4; of a whole folder).
-- - `chip`: what the chip pack lists (0x0812956C, which the editor opens with, 0x08131184): the chips up to the
--   pack's last (Death Phoenix), each with a code. That is every Standard, Mega and Giga chip but the BatCans, which
--   come after it; none of the Program Advances or the hidden chips. (Unlike EXE6's, EXE5's pack keeps the chips its
--   record's extra flag 0x20 marks, the soul modes'.)
-- - `code`: each chip in one of its codes. The editor won't open a folder with another, or with an entry of no
--   chip (0x081293A4, and for the other folders 0x081294A4).
-- - `copies`: of one chip, in any codes: four of a Standard chip, one of a Mega or of a Giga chip, one of a dark
--   chip (the editor's check as a chip goes in, 0x081325B4: its table 0x081326B8 by class, a dark chip's last).
--   EXE6 counts them by MB instead.
-- - `mega`, `giga`: at most the navi's Mega and Giga levels (NaviStats+0x0B, +0x0C). The editor checks them as a
--   chip goes in (counting with 0x081326C0), a dark chip aside; the link battle, before it starts, counts every
--   chip of the class (0x0812F1FC, counting with 0x0812A038), which this does. (No EXE5 dark chip is a Mega or Giga
--   chip.)
-- - `dark`: at most three chips with the dark flag (the record's flags 0x20; counted with 0x08132704).
-- - `regular`: an entry of the folder, its MB at most the navi's Regular memory (+0x09: Select in the editor,
--   0x0813200C). (The editor opening a folder whose Regular chip is past it sets the memory to 0xFF instead,
--   0x081327E8.)
-- - `tags`: EXE5's folders have no tag chips (they are EXE6's).
--
-- The editor doesn't look at a chip's light or dark (`megaman`, the record's +0x15): rules/light_dark does,
-- as a chip is used. A chip of another game (a mixed folder) is held to these rules by its record: its class, its
-- codes, its MB, its dark flag.

local batcan1 = require("../chips/batcan").batcan1
local batcan2 = require("../chips/batcan").batcan2
local batcan3 = require("../chips/batcan").batcan3
local batcan4 = require("../chips/batcan").batcan4

local SIZE = 30
local DARK_CHIPS = 3
-- The chip pack's last chip is 0x13A: these come after it.
local PAST_THE_PACK: { Chip } = { batcan1, batcan2, batcan3, batcan4 }

-- How many copies of a chip a folder holds, by its class (a dark chip one).
local COPIES: { [string]: number } = { standard = 4, mega = 1, giga = 1 }
local DARK_COPIES = 1

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local function is_dark(c: Chip): boolean
    return has(c.flags, "dark")
end

local function class(c: Chip): string
    return c.class or ""
end

-- What the chip pack lists: a chip a folder can hold.
local function in_pack(c: Chip): boolean
    return COPIES[class(c)] ~= nil and c.codes ~= nil and #c.codes > 0 and table.find(PAST_THE_PACK, c) == nil
end

local function copies_allowed(c: Chip): number
    if is_dark(c) then
        return DARK_COPIES
    end
    return COPIES[class(c)] or 0
end

local function name(c: Chip): string
    return c.id :: string
end

local function mb(c: Chip): number
    return c.mb or 0
end

-- The rules folder `f` breaks (a folder as `battle.checked_folder()` gives one: its chips, its Regular and tag chips
-- by their place among them, from 1), each said with `problem(rule, text)`; its limits are `stats'`.
local function rules_broken(stats: Navi, f: CheckedFolder, problem: (rule: string, text: string) -> ())
    local chips = f.chips
    if f.complete and #chips ~= SIZE then
        problem("size", string.format("the folder has %d chips; a folder is %d", #chips, SIZE))
    end
    local copies: { [Chip]: number } = {}
    local order: { Chip } = {}
    local mega, giga, dark = 0, 0, 0
    for i, e in chips do
        local c = e.chip
        if not in_pack(c) then
            problem("chip", string.format("entry %d: %s is no chip a folder can hold", i - 1, name(c)))
        end
        if not has(c.codes, e.code) then
            problem("code", string.format("entry %d: %s doesn't come in code %s", i - 1, name(c), e.code))
        end
        if copies[c] == nil then
            table.insert(order, c)
        end
        copies[c] = (copies[c] or 0) + 1
        if is_dark(c) then
            dark += 1
        end
        if c.class == "mega" then
            mega += 1
        elseif c.class == "giga" then
            giga += 1
        end
    end
    for _, c in order do
        local n, allowed = copies[c], copies_allowed(c)
        if n > allowed then
            local what = if is_dark(c) then "a dark chip" else string.format("a %s chip", class(c))
            problem("copies", string.format("%d copies of %s (%s), past %d", n, name(c), what, allowed))
        end
    end
    if mega > stats.mega_level then
        problem("mega", string.format("%d Mega chips, past the navi's %d", mega, stats.mega_level))
    end
    if giga > stats.giga_level then
        problem("giga", string.format("%d Giga chips, past the navi's %d", giga, stats.giga_level))
    end
    if dark > DARK_CHIPS then
        problem("dark", string.format("%d dark chips, past %d", dark, DARK_CHIPS))
    end
    if f.regular ~= nil then
        local e = chips[f.regular]
        if e == nil then
            problem("regular", string.format("the Regular chip is entry %d, past the folder", f.regular - 1))
        elseif mb(e.chip) > stats.regular_memory then
            problem(
                "regular",
                string.format("the Regular chip %s is %d MB, past the navi's %d", name(e.chip), mb(e.chip), stats.regular_memory)
            )
        end
    end
    if f.tags ~= nil then
        problem("tags", "tag chips, but EXE5's folders have none")
    end
end

-- `folder_check`: the folder a tool asks of (`battle.checked_folder()`), each rule it breaks a
-- `battle.folder_problem(rule, text)`.
local function check(side: number)
    local f = battle.checked_folder()
    if f == nil or f.side ~= side then
        return
    end
    rules_broken(battle.navi(side), f, battle.folder_problem)
end

-- `validate`'s part: the side's own folder (its setup's `folder`, `regular_chip`), each problem said with
-- `problem(text, field)`: its Regular and tag chips are entries with a chip, and it keeps the rules.
local function validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local setup = rules.setup_of(side)
    -- The entries with a chip, and each one's place among them (from 1) by its entry (from 0).
    local chips: { { chip: Chip, code: string } } = {}
    local place: { [number]: number } = {}
    for i, e in setup.folder do
        local chip: Chip?, code: string? = e.chip, e.code
        if chip ~= nil and code ~= nil then
            table.insert(chips, { chip = chip, code = code })
            place[i - 1] = #chips
        end
    end
    local outside = false
    local regular: number? = nil
    if setup.regular_chip ~= nil then
        regular = place[setup.regular_chip]
        outside = regular == nil
    end
    local tags: { number }? = nil
    if outside then
        problem("folder: the Regular or tag chips aren't chips of the folder", "folder")
    end
    local f: CheckedFolder = { side = side, chips = chips, regular = regular, tags = tags, complete = true }
    rules_broken(battle.navi(side), f, function(_, text)
        problem("folder: " .. text, "folder")
    end)
end

return {
    check = check,
    validate = validate,
}
