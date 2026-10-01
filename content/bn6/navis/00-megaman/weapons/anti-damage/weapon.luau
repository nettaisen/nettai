--!strict
-- The AntiDmg program's B+Back special (`megaman/anti-damage`): the
-- original's weapon routine 0x3D (`sub_80121BC`) and its stance, action
-- 0x5A (`sub_80EFE7C`). For 31 ticks the navi's
-- trap is armed (the navi state "trap_armed": a hit is caught as AntiDmg's
-- chip would, `sub_802CEF4`); a caught hit (requests 0x200 or 0x400) turns
-- the stance into the counter (`sub_80105F2`: AntiDmg's action 0x47, or
-- AntiSwrd's 0x48 for a sword hit) with the stance's 50 damage. The navi's
-- chips then wait 70 ticks.

export type State = {
    -- Ticks the trap stays armed (AIAttackVars+0x10).
    timer: number,
}

local anti_damage = {}

local DAMAGE, LOCKOUT, TICKS = 0x32, 0x46, 0x1E

-- `sub_80121BC`.
local function setup(navi: Object): Action
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = DAMAGE
    local s = navi:action_state(anti_damage.action) :: State
    s.timer = TICKS
    navi.attack_lockout = LOCKOUT
    return anti_damage.action
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

anti_damage.action = define.action {
    id = "megaman/anti-damage/action",
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

anti_damage.weapon = define.weapon {
    id = "megaman/anti-damage",
    name = "AntiDmg",
    -- (A B+Back special, which nothing charges: the charge table's row
    -- 0x3D by Charge 0 to 7, reading on into the next routine's row.)
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
    setup = setup,
    -- (Transitional: the number the NaviCust's program names it by.)
    legacy = { routines = { 0x3D } },
}

return anti_damage
