--!strict
-- The patch cards' FireAm (`megaman/fire-arm`, B↓FireAm, FireMan's card): weapon routine 0x4E (0x0800FC94) and action
-- 0x57 (0x080F0074, EXE5's own: EXE6's action 0x57 is DustCross's charged shot, of the same dispatch only). The navi
-- raises ElemRage's claw (the attachment table's row 0x34, 0c-33, its animation 1) and sends ElemRage's flame
-- (chips/elemrage: attack object #0x98) along its row from the panel ahead, at its attach point 20: 5 flames, each
-- standing 30 ticks (the attack's parameters 0x1E05), Fire, 50 damage, in the flames' palette 0, none spreading or
-- paralyzing. 30 ticks in, 6 more, and it is back to idle.

local attachment = require("../../../objects/attachment")
local elemrage = require("../../../chips/elemrage/elemrage")

-- The attack: 50 damage, hit parameter 20, Fire; its parameters: 5 flames (Param1), 30 ticks (Param2).
local DAMAGE, HIT_PARAM, FIRE = 50, 20, 1
local COUNT, TICKS = 5, 0x1E

local ANIM, ARM_ANIM, ATTACH_POINT = 10, 1, 20
local LAST_TICK, RECOVER_TICKS = 30, 5

export type State = {
    -- Ticks into the action (AIAttackVars+0x10).
    timer: number,
}

-- 0x080F0090: the flame out; the claw raised; on after the 30th tick.
local function fire(me: Object, s: State)
    if me.step_init == 0 then
        -- (The damage word takes the Atk+ bonus, AIAttackVars+8: none, the routine cleared it.)
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        -- (`sub_8018842` reads the attach point unflipped: the flame turns it its own way.)
        local fx, dz = me:attach_point(ATTACH_POINT)
        local dx = fx * me.facing
        elemrage.spawn_flame(me, me.panel_x + me.facing, me.panel_y, me.attack_element, int.shl(dz, 16), {
            count = COUNT,
            spread = false,
            ticks = TICKS,
            offset = int.i8(dx),
            paralyzes = false,
            attack_480 = false,
            palette = 0,
            hit = false,
            sent = false,
        }, damage)
        me.anim = ANIM
        me:open_counter_window()
        attachment.attach(me, elemrage.claw, "overlay", { anim = ARM_ANIM })
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    s.timer += 1
    if s.timer >= LAST_TICK then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080F0114: 6 ticks, then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

local ACTION = new.action {
    id = "megaman/fire-arm/action",
    state = { timer = "u16" },
    -- 0x080F0074.
    update = function(me: Object, s: State)
        if me.step == 0 then
            fire(me, s)
        elseif me.step == 4 then
            recover(me, s)
        else
            error(string.format("FireAm's phase %#x reads past 0x080F0088", me.step))
        end
    end,
}

-- 0x0800FC94: 50 damage, hit parameter 20, Fire; no charge, no bonus, no lockout; its parameters.
return new.weapon {
    id = "megaman/fire-arm",
    -- The charge table's row 0x4E, by Charge 0 to 5: past 4 it reads on into row 0x4F's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 80 },
    setup = function(navi: Object): Action
        navi.attack_damage = DAMAGE
        navi.hit_param = HIT_PARAM
        navi.attack_element = FIRE
        navi.charged = 0
        navi.extra = 0
        navi.attack_lockout = 0
        return ACTION
    end,
}
