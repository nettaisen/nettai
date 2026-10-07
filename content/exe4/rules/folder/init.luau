--!strict
-- EXE4's folder rules, as its folder editor enforces them as a chip goes in (0x08031458 on, by the chip's class,
-- 0x080190D0's +0x08): a tool's question (`folder_check`: a match's checks, netplay's offer, the editor, live play's
-- random folder), never the simulation's, as EXE5's and EXE6's (their rules/folder). The folder is
-- `battle.checked_folder()`; each rule it breaks is a `battle.folder_problem(rule, text)`. The limits are the navi's
-- stats as the round set them up. Red Sun US's addresses.
--
-- - `size`: a folder is 30 chips.
-- - `chip`: what the chip pack lists: a library chip (the record's flags 0x40) of a folder's classes, with a code. EXE4's
--   dark chips have no library flag: the custom screen offers them in battle, and no folder holds one.
-- - `code`: each chip in one of its codes.
-- - `copies`: of one chip, in any codes: four of a Standard chip (0x080314F0: the folder's count of it against 4), one of
--   a Mega or of a Giga chip (0x0803153C, 0x080315BC).
-- - `mega`, `giga`: at most the navi's Mega and Giga levels (the editor's counts, +0x18 and +0x19 of its block, against
--   the limits, +0x1C and +0x1D: 0x08031542, 0x080315C2; their counts by class, 0x08031484).
-- - `regular`: an entry of the folder, its MB at most the navi's Regular memory.
-- - `tags`: EXE4's folders have no tag chips (they are EXE6's).

local SIZE = 30

-- How many copies of a chip a folder holds, by its class.
local COPIES: { [string]: number } = { standard = 4, mega = 1, giga = 1 }

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local function class(c: Chip): string
    return c.class or ""
end

-- What the chip pack lists: a chip a folder can hold.
local function in_pack(c: Chip): boolean
    return COPIES[class(c)] ~= nil and has(c.flags, "library") and c.codes ~= nil and #c.codes > 0
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
    local mega, giga = 0, 0
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
        if c.class == "mega" then
            mega += 1
        elseif c.class == "giga" then
            giga += 1
        end
    end
    for _, c in order do
        local n, allowed = copies[c], COPIES[class(c)] or 0
        if n > allowed then
            problem("copies", string.format("%d copies of %s (a %s chip), past %d", n, name(c), class(c), allowed))
        end
    end
    if mega > stats.mega_level then
        problem("mega", string.format("%d Mega chips, past the navi's %d", mega, stats.mega_level))
    end
    if giga > stats.giga_level then
        problem("giga", string.format("%d Giga chips, past the navi's %d", giga, stats.giga_level))
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
        problem("tags", "tag chips, but EXE4's folders have none")
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
-- `problem(text, field)`: its Regular chip is an entry with a chip, and it keeps the rules.
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
    local regular: number? = nil
    if setup.regular_chip ~= nil then
        regular = place[setup.regular_chip]
        if regular == nil then
            problem("folder: the Regular chip isn't a chip of the folder", "folder")
        end
    end
    local f: CheckedFolder = { side = side, chips = chips, regular = regular, tags = nil, complete = true }
    rules_broken(battle.navi(side), f, function(_, text)
        problem("folder: " .. text, "folder")
    end)
end

return {
    check = check,
    validate = validate,
}
