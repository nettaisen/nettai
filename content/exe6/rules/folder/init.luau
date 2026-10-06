--!strict
-- EXE6's folder rules, as its folder editor enforces them (`sub_8135080`, with
-- `sub_8135500`) and a link battle checks them again before it starts
-- (`sub_8120D10`, counting with `sub_8120AE8`): a tool's question
-- (`folder_check`: a match's checks, netplay's offer, the editor, live play's
-- random folder), never the simulation's. The folder is
-- `battle.checked_folder()`; each rule it breaks is a
-- `battle.folder_problem(rule, text)`. The limits are the navi's stats as the
-- round set them up (the NaviCust's and the patch cards' work: the reload's,
-- which the editor and the link battle read).
--
-- - `size`: a folder is 30 chips (of a whole folder).
-- - `chip`: what the chip pack lists (`sub_811FE7C`): Standard, Mega and Giga
--   chips with a code, but the dark chips (extra flag 0x20) and those past
--   the pack's last (BatCan1-4 and BeastOut).
-- - `code`: each chip in one of its codes (`sub_8006EE8`; a battle counts any
--   other as the invalid chip).
-- - `copies`: of one chip, in any codes, by its MB: up to 19 MB five, 20-29
--   four, 30-39 three, 40-49 two, 50 and up one (`sub_8135500`).
-- - `mega`, `giga`: at most the navi's Mega and Giga levels (NaviStats+0x0B,
--   +0x0C).
-- - `dark`: at most three chips with the dark flag (no EXE6 chip has it).
-- - `regular`: an entry of the folder, its MB at most the navi's Regular
--   memory (+0x09: the editor drops a Regular chip past it, `sub_81352A0`,
--   `sub_813CEA0`).
-- - `tags`: two other entries, their MB together at most 60 (`sub_81349E8`: a
--   chip can be tagged while the tag chips' MB leave room for it).

local batcan_chips = require("../chips/batcan")
local beastout = require("../chips/beastout").beastout

local SIZE = 30
local DARK_CHIPS = 3
local TAG_MB = 60
-- The chip pack's last chip is 0x13A: these come after it.
local PAST_THE_PACK: { Chip } = { batcan_chips.batcan1, batcan_chips.batcan2, batcan_chips.batcan3, batcan_chips.batcan4, beastout }

local function copies_allowed(mb: number): number
    if mb <= 19 then
        return 5
    elseif mb <= 29 then
        return 4
    elseif mb <= 39 then
        return 3
    elseif mb <= 49 then
        return 2
    end
    return 1
end

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

-- What the chip pack lists: a chip a folder can hold.
local function in_pack(c: Chip): boolean
    local class = c.class
    return (class == "standard" or class == "mega" or class == "giga")
        and c.codes ~= nil
        and #c.codes > 0
        and not has(c.extra_flags, 32)
        and table.find(PAST_THE_PACK, c) == nil
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
        if has(c.flags, "dark") then
            dark += 1
        end
        if c.class == "mega" then
            mega += 1
        elseif c.class == "giga" then
            giga += 1
        end
    end
    for _, c in order do
        local n, allowed = copies[c], copies_allowed(mb(c))
        if n > allowed then
            problem("copies", string.format("%d copies of %s (%d MB), past %d", n, name(c), mb(c), allowed))
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
        local a, b = f.tags[1], f.tags[2]
        local x, y = chips[a], chips[b]
        if a == b then
            problem("tags", string.format("the tag chips are both entry %d", a - 1))
        elseif x == nil or y == nil then
            problem("tags", string.format("the tag chips are entries %d and %d, past the folder", a - 1, b - 1))
        else
            local together = mb(x.chip) + mb(y.chip)
            if together > TAG_MB then
                problem("tags", string.format("the tag chips %s and %s are %d MB, past %d", name(x.chip), name(y.chip), together, TAG_MB))
            end
        end
        if f.regular == a or f.regular == b then
            problem("tags", "the Regular chip is a tag chip")
        end
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

-- `validate`'s part: the side's own folder (its setup's `folder`, `regular_chip`, `tag_chips`), each problem said with
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
    local t = setup.tag_chips
    if #t == 2 then
        if place[t[1]] == nil or place[t[2]] == nil then
            outside = true
        else
            tags = { place[t[1]], place[t[2]] }
        end
    end
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
