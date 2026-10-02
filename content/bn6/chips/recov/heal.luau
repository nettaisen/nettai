--!strict
-- The recovery chips' action (the original's action 0x20, `sub_80EC844`):
-- in the tick after the chip is used, heal the chip's HP (`byte_80EC870`'s
-- row, by the original's subtype) through the ruleset's heal, which an
-- opponent's armed AntiRecv turns into damage (chips/antirecv/controller);
-- count the recovery in the side's statistics, and back to idle.

local heal = {}

-- The side statistic counting recoveries (`sub_800AB46`).
local RECOVERIES = 5

-- A chip's recovery of `hp`; `id`, the action's key when no chip
-- definition holds it; `mood`, what the heal adds to the side's mood
-- (BN5's 0x080EC484 adds the use's +0x0A after the heal; BN6's none).
function heal.action(spec: { id: string?, hp: number, mood: number? }): Action
    -- `sub_80EC844`. (The heal's result, whether the trap sprang, is
    -- tested and ignored.)
    local function update(me: Object, _s: any)
        me:heal(spec.hp, true)
        if spec.mood then
            battle.gain_mood(me.alliance, spec.mood)
        end
        battle.bump_side_stat(me.alliance, RECOVERIES, 1)
        me:exit_attack()
    end
    return define.action { id = spec.id, args = spec, update = update }
end

return heal
