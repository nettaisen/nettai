--!strict
-- The navi-changing chips' dimming controller (effect object #0x84,
-- `sub_80E9570`): HubBatc, the arm chips (PunchArm, NeedlArm, PuzzlArm,
-- BoomrArm), BugRSwrd and BgDthThd, and DarkInvs. The usual dimming phases
-- (the dimming service), with an effect by Param1 (`off_80E95C8`):
--
--   0 HubBatc (`sub_80E95D8`): the buster at attack level 5 or more, rapid
--     and charge 4, the custom screen at 8, the hand-shrink bug held off a
--     turn, a B+Back special if the navi had none, and the shoes and
--     undershirt, then 60 ticks;
--   1 BugRSwrd, BgDthThd (`sub_80E96C6`) and 2 the arms (`sub_80E9714`):
--     the charged shot becomes weapon routine Param2, then 60 ticks;
--   3 DarkInvs (`sub_80E9784`): the navi's cross-special request.
--
-- The chips' script (all are dimming chip subtype 38): `dimming_chip` is
-- their controller's spawner, `off_802CCB4[38]`.

export type State = {
    -- The chip and the Atk+ / cross bonus the telop shows (object +0x30,
    -- +0x32).
    chip: number,
    bonus: number,
}

local navi_boost = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x3C
-- The effects around the navi (effect #0 looks), and the sounds.
local HUB_LOOK, BUG_LOOK, ARM_LOOK = 0x5A, 0x5B, 0x5C
local HUB_SOUND, BUG_SOUND, LOG_IN_SOUND = 0x119, 0x143, 0x77
-- When the log-in sound plays, by timer.
local HUB_LOG_IN_AT, ARM_LOG_IN_AT = 0x1C, 0x28
-- HubBatc's buster, custom screen and B+Back special.
local MIN_BUSTER, HUB_RAPID, HUB_CHARGE, HUB_CUSTOM, HUB_BACK_SPECIAL = 5, 4, 4, 8, 0x3B
-- The buster weapon routines a new charged shot replaces (`sub_80E97BE`):
-- these two become none, and 0x2C becomes 0x2B.
local NO_BUSTER, BUSTER_ALIAS_FROM, BUSTER_ALIAS_TO = 0, 0x2C, 0x2B
-- The collision types a HubBatc'd navi has (`sub_801A082`).
local BODY_SELF_TYPE, BODY_TARGET_TYPE = 0x10, 2

-- `sub_80E979C`: the controller for `user`, on its panel, with the chip's
-- parameters and damage word. (Its position is the spawner's registers,
-- which nothing reads.)
function navi_boost.dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn_kind("navi-boost", Vec3.zero, spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = spec.chip, spec.bonus
    return o
end

local function user_of(me: Object): Object
    local user = me.related1
    if not user then
        error("the navi-changing chips' controller has no user (it reads through a null pointer)")
    end
    return user
end

-- `sub_8018810`: the user's attach point 0, facing the controller's way.
local function attach_offset(me: Object, user: Object): (number, number)
    local x, y = user:attach_point(0)
    return x * user.facing * me.facing, y
end

-- Count down to the undim; `log_in_at`: when the log-in sound plays.
local function wait(me: Object, log_in_at: number?)
    if log_in_at and me.timer == log_in_at then
        battle.play_sound(LOG_IN_SOUND)
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E97BE`: the navi's charged shot is weapon routine `routine` now;
-- a buster that aliased the old charged shot goes.
local function new_charged_shot(user: Object, routine: number)
    user.charge_shot_routine = routine
    local buster = user.buster_routine
    if buster == 4 or buster == 3 then
        user.buster_routine = NO_BUSTER
    elseif buster == BUSTER_ALIAS_FROM then
        user.buster_routine = BUSTER_ALIAS_TO
    end
end

-- `sub_80E95D8`: HubBatc.
local function hub_batch(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer = WAIT
        local user = user_of(me)
        local dx, dy = attach_offset(me, user)
        local p = user.pos
        battle.effect(Vec3.new(p.x + dx * 0x10000, p.y, p.z + dy * 0x10000), HUB_LOOK)
        battle.play_sound(HUB_SOUND)
        local stats = battle.navi(user.alliance)
        local buster = user:buster_damage()
        if buster < MIN_BUSTER then
            stats.attack = stats.attack + MIN_BUSTER - buster
        end
        stats.rapid = HUB_RAPID
        stats.charge = HUB_CHARGE
        stats.custom_level = HUB_CUSTOM
        local shrink = stats.hand_shrink_turn
        local turn = battle.turn()
        if shrink ~= 0 and turn >= shrink then
            stats.hand_shrink_turn = turn + 1
        end
        if stats.back_special_routine == 0xFF then
            stats.back_special_routine = HUB_BACK_SPECIAL
        end
        user:set_status("air_shoe", true)
        user:set_status("float_shoe", true)
        user:set_status("undershirt", true)
        stats.float_shoes = true
        stats.air_shoes = true
        stats.undershirt = true
        -- sub_80107C0: bodies' hit modifier (3 in link battles).
        local hit_mod = if battle.link() then 3 else 0
        user:reset_collision_types(BODY_SELF_TYPE, BODY_TARGET_TYPE, hit_mod)
    end
    wait(me, HUB_LOG_IN_AT)
end

-- `sub_80E96C6`: BugRSwrd and BgDthThd.
local function bug_charge(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer = WAIT
        local user = user_of(me)
        battle.effect(Vec3.new(user.pos.x, user.pos.y, 0), BUG_LOOK)
        battle.play_sound(BUG_SOUND)
        battle.navi(me.alliance).charge_shot_routine = me:param(2)
        new_charged_shot(user, me:param(2))
    end
    wait(me, nil)
end

-- `sub_80E9714`: the arm chips. (The game shifts the attach point's X a
-- second time where it means its Y, so the effect gets no height.)
local function arm(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer = WAIT
        local user = user_of(me)
        local dx = attach_offset(me, user)
        local p = user.pos
        battle.effect(Vec3.new(p.x + dx * 0x10000, p.y, p.z), ARM_LOOK, 0, me:param(3))
        battle.navi(me.alliance).charge_shot_routine = me:param(2)
        new_charged_shot(user, me:param(2))
    end
    wait(me, ARM_LOG_IN_AT)
end

-- `sub_80E9784`: DarkInvs.
local function dark_invis(me: Object)
    user_of(me):set_request("cross_special", true)
    me:set_action(0xC)
end

local EFFECTS: { [number]: (Object) -> () } = { [0] = hub_batch, [1] = bug_charge, [2] = arm, [3] = dark_invis }

-- `sub_80E95B4`.
local function effect(me: Object)
    local f = EFFECTS[me:param(1)]
    if not f then
        error(string.format("navi-changing chip Param1 %d reads past off_80E95C8", me:param(1)))
    end
    f(me)
end

-- `sub_80E9570`: the dimming's phases around the effect.
function navi_boost.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("the navi-changing chips' action %#x reads past their table", a))
        end
    else
        dimming.finish(me)
    end
end

return navi_boost
