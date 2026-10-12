--!strict
-- EXE5's folder rules, as its folder editor enforces them and a link battle checks them again before it starts: a
-- tool's questions (`folder_check`, `folder_view`, `validate`: a match's checks, netplay's offer, the build screens),
-- never the simulation's. The rules every game of the series shares are @exelib/folder_rules; these are EXE5's own.
-- Team ProtoMan US's addresses; Team Colonel's routines are the same code, and so is its table of copies.
--
-- - A folder is 30 chips (the editor's list of a folder is its 30 entries, 0x081293A4).
-- - The chips a folder can hold are those a save can collect, which is what its chip pack can list (0x0812956C, which
--   the editor opens with, 0x08131184): the Standard, Mega and Giga chips its library lists (the record's flags
--   0x40), its dark chips (the library's own tab of them), and those a chip's `collectible` says: the six Giga chips
--   Team Colonel's library lists (each version's ROM gives the library flag to its own six and not to the other's;
--   the content keeps one record of each chip, Team ProtoMan's ROM's), and LeadRaid and ChaosLrd, which the Japanese
--   game gave out and no library lists; each with a code. That leaves out the soul modes' chips (CannMode to FinalGun:
--   their records have no library flag, and Tango's saves' packs hold none, though the pack's own list would keep one,
--   the record's extra flag 0x20), the BatCans (after the pack's last chip, Death Phoenix) and the Program Advances.
-- - A save of one team holds none of the other's Giga chips: its setup's `giga_chips` are the Giga chips it can have
--   (its version's six and ChaosLrd, which a tool's version presets name; a side that states none has every one).
-- - Copies of one chip, in any codes: four of a Standard chip, one of a Mega or of a Giga chip, one of a dark chip
--   (the editor's check as a chip goes in, 0x081325B4: its table 0x081326B8 by class, a dark chip's last). EXE6
--   counts them by MB instead.
-- - The Mega and Giga chips are at most the navi's Mega and Giga levels (the editor checks them as a chip goes in,
--   counting with 0x081326C0, a dark chip aside; the link battle counts with 0x0812A038, 0x0812F1FC). No EXE5 dark
--   chip is a Mega or Giga chip.
-- - At most three dark chips (the record's flags 0x20; counted with 0x08132704).
-- - The Regular chip's MB is at most the navi's Regular memory (+0x09: Select in the editor, 0x0813200C). (The editor
--   opening a folder whose Regular chip is past it sets the memory to 0xFF instead, 0x081327E8.)
-- - EXE5's folders have no tag chips (they are EXE6's).
--
-- The editor doesn't look at a chip's light or dark (`megaman`, the record's +0x15): rules/light_dark does, as a chip
-- is used.

local folder_rules = require("@exelib/folder_rules")

-- How many copies of a chip a folder holds, by its class (a dark chip one).
local COPIES: { [string]: number } = { standard = 4, mega = 1, giga = 1 }

local function has(list: any, x: any): boolean
    return list ~= nil and table.find(list, x) ~= nil
end

local RULES: FolderRules = {
    size = 30,
    in_pack = function(c: Chip): boolean
        return COPIES[c.class or ""] ~= nil
            and c.codes ~= nil
            and #c.codes > 0
            and (has(c.flags, "library") or has(c.flags, "dark") or (c :: any).collectible == true)
    end,
    owned = function(c: Chip, setup: any): boolean
        if c.class ~= "giga" then
            return true
        end
        for _, held in setup.giga_chips do
            if held == c then
                return true
            end
        end
        return false
    end,
    copies = function(c: Chip): number
        if has(c.flags, "dark") then
            return 1
        end
        return COPIES[c.class or ""] or 0
    end,
    dark = 3,
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
