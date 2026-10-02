--!strict
-- The turn (the original's navi action 0x3B, `sub_80EDF0C`): where turning
-- is enabled (the navi's "can_turn" state), L or R raises a request, and
-- the idle navi answers it by turning round two ticks later. The ruleset
-- starts it by role (`actions.turn`, rules/roles.luau).

local turn = {}

export type State = {
    -- The countdown to the turn (AIAttackVars+0x10).
    timer: number,
}

local WAIT = 1

turn.action = define.action {
    id = "bn6:megaman/turn",
    state = { timer = "u16" },
    update = function(me: Object, s: State)
        if me.step == 0 then
            s.timer = WAIT
            me.step = 4
        end
        -- The wait ends the tick the count goes below zero.
        local left = s.timer - 1
        s.timer = left
        if left >= 0 then
            return
        end
        -- Turned to face its own side's way for L's request, the other way
        -- for R's (`sub_800F2C6`: the sprite, and the HP number's place,
        -- which the engine doesn't draw); neither: as it stands.
        local flip: number? = nil
        if me:request("turn_l") then
            flip = 0
        elseif me:request("turn_r") then
            flip = 1
        end
        if flip then
            me.flip = flip
            me.sprite:set_flip(bit32.bxor(me.alliance, flip))
        end
        me:set_request("turn_l", false)
        me:set_request("turn_r", false)
        me:exit_attack()
    end,
}

return turn
