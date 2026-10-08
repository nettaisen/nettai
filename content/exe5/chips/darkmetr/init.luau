--!strict
-- DarkMetr (EXE5 chip 0x0c3): a dark chip: a shower of Fire meteors, 10 and 2 more for each of the first three
-- reactions its side's navi took, one every 10 ticks on one of the enemy navis' panels, each breaking its panel and
-- carrying the bug 0x19 (chips/meteors: instant chip effect 17 with its first parameter set). Its use costs custom
-- drain (rules/light_dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local falling_meteor = require("@exelib/meteors/falling_meteor")
local meteors = require("./meteors/meteors")

-- Its shower: 10 meteors and 2 more for each of the first three reactions its side's navi took (side stat 3), one
-- every 10 ticks, on the navi picked's own panel, each hitting as EXE5's row 0x2B with the bug 0x19 (argument 1),
-- shaking the cameras at 2 and breaking its panel (the meteor's row 4).
local SHOWER = meteors.variant {
    period = 10,
    row = falling_meteor.row {
        is = collision.collision_2b,
        hits = collision.hits_navis,
        hit_mod = 3,
        shake = 2,
        panel = "break",
        bug = 0x19,
        bug_arg = 1,
    },
    aim = function(target: Object?): (number, number)
        if not target then
            error("DarkMetr's meteor shower without a target (0x080E591E reads a panel through a null pointer)")
        end
        return target.panel_x, target.panel_y
    end,
    more = function(me: Object): number
        return math.min(battle.side_stat(me.alliance, 3), 3) * 2 + 10
    end,
}

local chip: Chip = {
    codes = { "F" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 132,
    lockout = 20,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    instant = meteors.instant(SHOWER, 0),
    auto_battle = { class = "at_once" },
}

return { darkmetr = chip }
