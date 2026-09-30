--!strict
-- The AntiDmg program's B+Back special: weapon routine 0x3D (`sub_80121BC`)
-- and its stance, action 0x5A (`sub_80EFE7C`). For 31 ticks the navi's
-- trap is armed (the navi state "trap_armed": a hit is caught as AntiDmg's
-- chip would, `sub_802CEF4`); a caught hit (requests 0x200 or 0x400) turns
-- the stance into the counter (`sub_80105F2`: AntiDmg's action 0x47, or
-- AntiSwrd's 0x48 for a sword hit) with the stance's 50 damage. The navi's
-- chips then wait 70 ticks.

export type State = {
    -- Ticks the trap stays armed (AIAttackVars+0x10).
    timer: number,
}

local anti_damage = {
    state = { timer = "u16" },
}

local ACTION = 0x5A
local DAMAGE, LOCKOUT, TICKS = 0x32, 0x46, 0x1E

-- `sub_80121BC`.
function anti_damage.setup(navi: Object): number
    navi.attack_element = 0
    navi.variant = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    for i = 1, 4 do
        navi:set_attack_param(i, 0)
    end
    navi.attack_damage = DAMAGE
    local s = navi:action_state(ACTION) :: State
    s.timer = TICKS
    navi.attack_lockout = LOCKOUT
    return ACTION
end

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

-- `sub_80EFE7C`.
function anti_damage.update(me: Object, s: State)
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
end

return anti_damage
