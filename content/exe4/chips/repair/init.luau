--!strict
-- Repair (EXE4 chip 0x073; its record at 0x080197EC + 0x73 * 0x2C): the spawners' action 0x20 (lib/spawners), its
-- variant 0x0B (0x0800C720, EXE5's 0x0800E26C but for its constants): each field object the user's side owns (the
-- registry's three slots, the battle's +0x68) that has HP left is back to its full HP, with the heal's sparkle at it
-- (effect row 7) and its sound (0x9A).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local roles = require("../rules/roles")
local spawners = require("../lib/spawners")

-- The heal's (the rules' roles: effect row 7, sound 0x9A).
local SPARKLE = (roles.effects :: any).recovery :: Effect
local SOUND = (roles.sounds :: any).recovery :: Sound

-- 0x0800C720.
local function repair(user: Object, _element: number, _damage: number)
    for i = 1, 3 do
        local o = field.object_slot(user.alliance, i)
        if o and o.hp ~= 0 then
            o.hp = o.max_hp
            battle.effect(o.pos, SPARKLE)
            battle.play_sound(SOUND)
        end
    end
end

local chip: Chip = {
    action = spawners.action { id = "repair/action", spawn = repair },
    codes = { "I", "P", "R", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { repair = chip }
