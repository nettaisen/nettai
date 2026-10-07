--!strict
-- EXE4's recoveries, the chips' action 0x1D (0x080EC71A): EXE5's heal (0x080EC484, @exelib/recov/heal: the navi heals
-- the variant's HP, 0x080EC74C's 10, 30, 50, 80, 120, 150, 200, 300 and DrkRecov's 1000, through the roles' recovery
-- sparkle and sound, which an armed AntiRecv turns into damage; the side's recoveries counted), whose side's mood gains
-- the use's +0x0A (0x0800F51E) whether or not an AntiRecv turned the heal. Each chip composes its own with its
-- variant's HP and its record's counter byte.

local heal = require("@exelib/recov/heal")

local recov = {}

function recov.action(spec: { id: string, hp: number, mood: number }): Action
    return heal.action { id = spec.id, hp = spec.hp, mood = spec.mood, mood_after_trap = true }
end

return recov
