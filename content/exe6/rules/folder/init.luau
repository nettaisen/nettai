--!strict
-- EXE6's folder rules, as its folder editor enforces them (`sub_8135080`, with `sub_8135500`) and a link battle checks
-- them again before it starts (`sub_8120D10`, counting with `sub_8120AE8`): a tool's questions (`folder_check`,
-- `folder_view`, `validate`: a match's checks, netplay's offer, the build screens), never the simulation's. The rules
-- every game of the series shares are @exelib/folder_rules; these are EXE6's own.
--
-- - The chips a folder can hold are those a save can collect, which is what its chip pack can list (`sub_811FE7C`):
--   the Standard, Mega and Giga chips its library lists (the record's flags 0x40), each with a code; a version's own
--   chips (the chip's `version`); and DblBeast, which the Japanese game gave out and no library lists
--   (`collectible`). That leaves out the four arm chips, SyncTrgr and the dark chips (Standard and Mega chips with
--   codes whose records have no library flag: no library lists them, and Tango's saves' packs hold none), the BatCans
--   and BeastOut (after the pack's last chip, 0x13A), and the Program Advances.
-- - A save of one version holds none of the other's own chips (the chip's `version`: five of each version's Giga
--   chips, and Gregar and Falzar themselves, which the Japanese game gave out). Each version's ROM gives the library
--   flag to its own five Giga chips and not to the other's (the four ROMs' chip records; the content keeps one record
--   of each chip, the Falzar ROM's), each version's traders and rewards give its own, and a Giga chip isn't traded.
-- - Copies of one chip, in any codes, by its MB: up to 19 MB five, 20-29 four, 30-39 three, 40-49 two, 50 and up one
--   (`sub_8135500`).
-- - The Regular chip's MB is at most the navi's Regular memory (+0x09: the editor drops a Regular chip past it,
--   `sub_81352A0`, `sub_813CEA0`).
-- - The tag chips are two other entries, their MB together at most 60 (`sub_81349E8`: a chip can be tagged while the
--   tag chips' MB leave room for it).
--
-- The editor also holds a folder to three chips with the dark flag: no EXE6 chip has the flag (its dark chips are
-- marked otherwise, and no folder holds them), so no folder can break that rule and it isn't stated here.

local folder_rules = require("@exelib/folder_rules")

local FOLDER_CLASSES: { [string]: boolean } = { standard = true, mega = true, giga = true }

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local RULES: FolderRules = {
    size = 30,
    in_pack = function(c: Chip): boolean
        return FOLDER_CLASSES[c.class or ""] == true
            and c.codes ~= nil
            and #c.codes > 0
            and (has(c.flags, "library") or (c :: any).version ~= nil or (c :: any).collectible == true)
    end,
    owned = function(c: Chip, setup: any): boolean
        local version: string? = (c :: any).version
        return version == nil or version == setup.version
    end,
    copies = function(c: Chip): number
        local mb = c.mb or 0
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
    end,
    tag_memory = 60,
}

return {
    check = function(side: number): { SetupProblem }?
        return folder_rules.check(RULES, side, rules.setup_of(side))
    end,
    view = function(side: number): FolderView?
        return folder_rules.checked_view(RULES, side, rules.setup_of(side))
    end,
    validate = function(side: number, problem: (p: SetupProblem) -> ())
        folder_rules.validate(RULES, side, rules.setup_of(side), problem)
    end,
}
