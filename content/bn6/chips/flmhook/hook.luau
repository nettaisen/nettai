--!strict
-- FlmHook's hook (the original's effect object #0x8C, `sub_80EA010`): the
-- shared hook (@exelib/flmhook/hook), BN6's kind. It has no sprite; it
-- swings two flames (./fire), one from the column in front of its user and
-- 20 ticks later one from the column two ahead, then goes 10 ticks later.
-- FlmHook1 to 3 bring it as their instant effect (`sub_80EA11C`, instant
-- chip effect 14); the BigHook Giga chip brings it as its navi
-- (`off_802CD5C[23]`, the same routine), its flames the big ones, from
-- columns 4 then 5 (side 1: 3 then 2), and the controller waits for it.
-- See docs/engine/chips.md §3.6.35.

local common = require("@exelib/flmhook/hook")
local fire = require("./fire")

-- What a chip's hook swings: big flames (from fixed columns), and their
-- hit spark.
export type Flames = { big: boolean, spark: Spark }

-- The controller calls the spawner (`sub_80EA11C`) through r3, which the
-- hook's position takes (nothing sets it later).
local SPAWNER_ADDRESS = 0x080EA11D

local made = common.make {
    id = "bn6:flmhook/hook",
    columns = function(me: Object, big: boolean): (number, number)
        if big then
            return 4 - me.alliance, 5 - me.alliance * 3
        end
        return me.panel_x + me.facing, me.panel_x + 2 * me.facing
    end,
    flame = function(owner: Object, x: number, y: number, element: number, damage: number, swing: number, flames: Flames): Object?
        return fire.swing(owner, x, y, element, damage, { swing = swing, big = flames.big, spark = flames.spark })
    end,
    keeps_element = true,
}

local hook = {}

hook.kind = made.kind

-- FlmHook's instant effect (`off_80EC3F0[14]`): the hook with `flames`.
hook.instant = made.instant

-- BigHook's navi hook (`off_802CD5C[23]`): the hook with `flames`,
-- reporting to `controller`.
function hook.summon(flames: Flames): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return made.summon(flames, SPAWNER_ADDRESS)
end

return hook
