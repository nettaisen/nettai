--!strict
-- Jealousy's dimming controller (EXE5's effect object #0x36, 0x080E443C; EXE6's code, which no EXE6 chip uses):
-- @exelib/jealousy/controller with EXE5's look: the chips counted (0x080E4562: the most any of the other side's actors
-- holds, a player's hand from its cursor, another actor's own count), its hits' row 0x1A at navis with the heavy spark,
-- and the operation battle's warning over the gauge (0x080E4642) and emptied gauge (0x080E4628).

local collision = require("../../rules/collision")
local common = require("@exelib/jealousy/controller")
local operation_battle = require("../../lib/operation_battle")
local sparks = require("../../lib/sparks")

local controller = {}

-- 0x080E4596: the chips `o` holds: a player its side's hand from the cursor, another actor its own count (+0x1A).
local function held(o: Object): number
    if o.actor_type == "player" then
        return battle.hand_left(o.alliance)
    end
    return o.chips_held
end

-- 0x080E4562: the most chips any of the other side's actors holds.
local function most_held(me: Object): number
    local most = 0
    local other = bit32.bxor(me.alliance, 1)
    for i = 1, 4 do
        local o = battle.alive_actor_slot(other, i)
        if o then
            local n = held(o)
            if n >= most then
                most = n
            end
        end
    end
    return most
end

-- 0x080E4540: Jealousy's `dimming` hook.
controller.hook = common.make {
    id = "jealousy/controller",
    count = most_held,
    is = collision.collision_1a,
    hits = collision.hits_navis,
    spark = sparks.heavy,
    operation_battle = operation_battle,
    warning = asset.sound("err-select-91"),
}

return controller
