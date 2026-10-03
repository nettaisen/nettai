--!strict
-- The anti-damage stance, as BN5 and BN6 share it (BN6's action 0x5A,
-- `sub_80EFE7C`: the AntiDmg program's B+Back; BN5's 0x3D, 0x080EE9F4, the
-- same code: ShadowSoul's B+Back). For its ticks (AIAttackVars+0x10, which
-- its weapon's setup sets) the navi's trap is armed (the navi state
-- "trap_armed": a hit is caught as AntiDmg's chip would); a caught hit
-- (requests 0x200 or 0x400) turns the stance into the counter
-- (`sub_80105F2`: the roles' `anti_damage_counter`, or `anti_sword_counter`
-- for a sword hit) with the stance's damage word and lockout. Each game
-- names its action (`anti_damage.stance(id)`).

local anti_damage = {}

export type State = {
    -- Ticks the trap stays armed (AIAttackVars+0x10).
    timer: number,
}

-- The stance action `id`.
function anti_damage.stance(id: string): Action
    -- `sub_80EFEAC`: the trap is armed until the timer runs out.
    local function stance(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me:set_navi_state("trap_armed", true)
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:set_navi_state("trap_armed", false)
            me:exit_attack()
        end
    end

    return define.action {
        id = id,
        state = { timer = "u16" },
        -- `sub_80EFE7C`.
        update = function(me: Object, s: State)
            if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
                me:start_stance_counter()
                return
            end
            local step = me.step
            if step == 0 then
                stance(me, s)
            else
                error(string.format("the AntiDmg stance's phase %#x reads past its table", step))
            end
        end,
    }
end

return anti_damage
