--!strict
-- ShadowMan's own chip, SplitUp (EXE5 chip 0x19e), which his custom screen offers until it is picked (rules: the
-- navi's `own_chip`), and its use, his action 10 (0x080F998A), over in the tick it starts: it finds the nearest enemy
-- body ahead (./aim) and sends his shadow (./shadow, actor object #4) to the panel before it, on his side of it, to
-- slash the two panels ahead of it as LongSwrd does; no enemy ahead, no shadow. He himself is idle again at once.
-- The slash's damage is by his level (the chips' damage formula 64: the damage rows' row 15, lib/navi_level), with
-- the Atk+ bonus. Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.
--
-- The shadow takes the chip's record's parameters (+0x10 to +0x13: no wait before its swing, and its third, 1,
-- the shadow no hit reaches that slashes where it is sent). The action's other variant, 1 (the panel before the
-- attack's target object), is his auto battle's.

local navi_level = require("../../lib/navi_level")
local aim = require("./aim")
local shadow = require("./shadow")

-- The chip's record's first parameter: the shadow's wait before its swing.
local WAIT = 0

local action = define.action {
    id = "splitup/action",
    -- 0x080F998A
    update = function(me: Object)
        local x, y = aim.nearest_enemy(me)
        if x and y then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            shadow.send(me, x - me.facing, y, WAIT, damage, int.u32(damage + me.extra))
        end
        me:exit_attack()
    end,
}

return define.chip {
    id = "splitup",
    codes = { "S" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 80, 80, 80, 80, 80, 100, 100 }),
    hit_param = 148,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}
