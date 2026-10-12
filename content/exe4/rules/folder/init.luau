--!strict
-- EXE4's folder rules, as its folder editor enforces them as a chip goes in (0x08031458 on, by the chip's class,
-- 0x080190D0's +0x08): a tool's questions (`folder_check`, `folder_view`, `validate`: a match's checks, netplay's
-- offer, the build screens), never the simulation's. The rules every game of the series shares are
-- @exelib/folder_rules; these are EXE4's own. Red Sun US's addresses.
--
-- - A folder is 30 chips.
-- - The chips a folder can hold are those a save can collect: the Standard, Mega and Giga chips its library lists
--   (the record's flags 0x40), each with a code, and the e-Reader's chips no library lists (`collectible`: Duo and
--   PrixPowr). EXE4's dark chips have no library flag: the custom screen offers them in battle, and no folder holds
--   one.
-- - A save of one version holds none of the other's Giga chips: its setup's `giga_chips` are the Giga chips it can have
--   (its version's five and the e-Reader's Duo, which a tool's version presets name; a side that states none has every
--   one).
-- - Copies of one chip, in any codes: four of a Standard chip (0x080314F0: the folder's count of it against 4), one of
--   a Mega or of a Giga chip (0x0803153C, 0x080315BC).
-- - The Mega and Giga chips are at most the navi's Mega and Giga levels (the editor's counts, +0x18 and +0x19 of its
--   block, against the limits, +0x1C and +0x1D: 0x08031542, 0x080315C2; their counts by class, 0x08031484).
-- - The Regular chip's MB is at most the navi's Regular memory.
-- - EXE4's folders have no tag chips (they are EXE6's).

local folder_rules = require("@exelib/folder_rules")

-- How many copies of a chip a folder holds, by its class.
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
            and (has(c.flags, "library") or (c :: any).collectible == true)
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
        return COPIES[c.class or ""] or 0
    end,
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
