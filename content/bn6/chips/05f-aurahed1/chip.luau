--!strict
-- AuraHed1/2/3 and StreamHd: action 0x43 (`sub_80EE634`). The navi raises
-- its buster and fires an aura head (attack object #0x92,
-- objects/aura-head) along its row, recoiling half a panel and back over
-- 31 ticks. StreamHd (variant 1) fires five in a row, 16 ticks apart: its
-- first attack parameter counts them (and is each head's palette), and
-- the last one gets its third (the stronger hit modifier) and the longer
-- recoil.

local buster = require("../../lib/buster")
local aura_head = require("../../objects/aura-head/aura_head")

export type State = {
    -- Ticks left in the recoil (AIAttackVars+0x10).
    timer: number,
}

local aura_hed = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local RAISE, FIRE, RECOIL = 0x0, 0x4, 0x8
-- StreamHd's variant fires until its count reaches this.
local LAST = 4
-- The recoil's start speed and slowdown, 16.16.
local RECOIL_SPEED, RECOIL_DRAG = 0x3_0000, 0x8000

-- More heads to come (StreamHd, not yet at its last).
local function streaming(me: Object): boolean
    return me.variant ~= 0 and me:attack_param(1) ~= LAST
end

-- `sub_80EE654`: raise the buster.
local function raise(me: Object)
    me:set_animation(0x0E)
    buster.raise_arm(me)
    me:set_status("using_action", true)
    me.step = FIRE
    me.step_init = 0
end

-- `sub_80EE66E`: fire a head from the navi's panel, and start the recoil.
local function fire(me: Object, s: State)
    local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
    local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
    aura_head.spawn(me, me.panel_x, me.panel_y, me.attack_element, params, damage)
    s.timer = if streaming(me) then 0x0F else 0x1E
    me.vel = Vec3.new(RECOIL_SPEED, me.vel.y, me.vel.z)
    me.step = RECOIL
    me.step_init = 0
end

-- `sub_80EE6A4`: slide back, slowing; then (StreamHd) the next head, or
-- back to the panel and to idle.
local function recoil(me: Object, s: State)
    local v = me.vel.x
    if v ~= 0 then
        local p = me.pos
        me.pos = Vec3.new(p.x - me.facing * v, p.y, p.z)
        me.vel = Vec3.new(v - RECOIL_DRAG, me.vel.y, me.vel.z)
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        return
    end
    me:set_coordinates_from_panel()
    if streaming(me) then
        local n = me:attack_param(1) + 1
        me:set_attack_param(1, n)
        if n == LAST then
            me:set_attack_param(3, 1)
        end
        me.step = FIRE
        me.step_init = 0
        return
    end
    me.overlay = nil
    me:exit_attack()
end

-- `sub_80EE634`.
function aura_hed.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me)
    elseif step == FIRE then
        fire(me, s)
    elseif step == RECOIL then
        recoil(me, s)
    else
        error(string.format("AuraHed phase %#x reads past its phase table", step))
    end
end

return aura_hed
