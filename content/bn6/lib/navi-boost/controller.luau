--!strict
-- The navi-changing chips' dimming controller (the original's effect
-- object #0x84, `sub_80E9570`): HubBatc, the arm chips (PunchArm,
-- NeedlArm, PuzzlArm, BoomrArm), BugRSwrd and BgDthThd, and DarkInvs. The
-- usual dimming phases (lib/dimming), with an effect by what the chip
-- changes (`off_80E95C8`, by the original's first chip parameter):
--
--   "hub" (0, HubBatc, `sub_80E95D8`): the buster at attack level 5 or
--     more, rapid and charge 4, the custom screen at 8, the hand-shrink
--     bug held off a turn, a B+Back special if the navi had none, and the
--     shoes and undershirt, then 60 ticks;
--   "bug" (1, BugRSwrd and BgDthThd, `sub_80E96C6`) and "arm" (2, the
--     arms, `sub_80E9714`): the charged shot becomes the chip's weapon,
--     then 60 ticks;
--   "dark" (3, DarkInvs, `sub_80E9784`): the navi's cross-special request.
--
-- A chip's `dimming` hook (`sub_80E979C`, `off_802CCB4[38]`) is
-- `navi_boost.hub`, `navi_boost.bug(routine)`, `navi_boost.arm(routine,
-- palette)` or `navi_boost.dark`. A weapon is still its routine's number
-- (the navi's weapon slots take numbers until the weapons are
-- definitions).

local collision = require("../../rules/collision")
local dimming_chips = require("../dimming")

local navi_boost = {}

export type Change = "hub" | "bug" | "arm" | "dark"

export type State = {
    -- What the chip changes.
    change: Change,
    -- The weapon routine that becomes the charged shot (the original's
    -- second chip parameter), and the arm effect's palette (its third).
    routine: number,
    palette: number,
}

local WAIT = 0x3C
-- The effects around the navi (the original's effects 0x5A to 0x5C), and
-- the sounds.
local HUB_EFFECT = define.effect { sprite = asset.sprite("hub"), anim = 0, palette = 0 }
local BUG_EFFECT = define.effect { sprite = asset.sprite("bug"), anim = 0, palette = 0 }
local ARM_EFFECT = define.effect { sprite = asset.sprite("charge-glow-a"), anim = 3, palette = 1 }
local HUB_SOUND, BUG_SOUND, LOG_IN_SOUND = asset.sound("hub"), asset.sound("bug"), asset.sound("log-in-77")
-- When the log-in sound plays, by timer.
local HUB_LOG_IN_AT, ARM_LOG_IN_AT = 0x1C, 0x28
-- HubBatc's buster, custom screen and B+Back special (a weapon routine:
-- the NaviCust Shield's).
local MIN_BUSTER, HUB_RAPID, HUB_CHARGE, HUB_CUSTOM, HUB_BACK_SPECIAL = 5, 4, 4, 8, 0x3B
-- No weapon in a slot.
local NO_WEAPON = 0xFF
-- The buster weapon routines a new charged shot replaces (`sub_80E97BE`):
-- the Beast busters (3 and 4) become the plain buster, and the Beast's
-- throw of an absorbed obstacle (0x2C) the Cross's (0x2B).
local FALZAR_BEAST_BUSTER, GREGAR_BEAST_BUSTER, BUSTER = 3, 4, 0
local BEAST_THROW, THROW = 0x2C, 0x2B

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
        dimming_chips.done(me)
    end
end

-- `sub_80E97BE`: the navi's charged shot is weapon routine `routine` now;
-- a buster that aliased the old charged shot goes.
local function new_charged_shot(user: Object, routine: number)
    user.charge_shot_routine = routine
    local buster = user.buster_routine
    if buster == GREGAR_BEAST_BUSTER or buster == FALZAR_BEAST_BUSTER then
        user.buster_routine = BUSTER
    elseif buster == BEAST_THROW then
        user.buster_routine = THROW
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
        battle.effect(Vec3.new(p.x + dx * 0x10000, p.y, p.z + dy * 0x10000), HUB_EFFECT)
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
        if stats.back_special_routine == NO_WEAPON then
            stats.back_special_routine = HUB_BACK_SPECIAL
        end
        user:set_status("air_shoe", true)
        user:set_status("float_shoe", true)
        user:set_status("undershirt", true)
        stats.float_shoes = true
        stats.air_shoes = true
        stats.undershirt = true
        -- sub_80107C0: bodies' hit modifier (3 in link battles); the body
        -- floats now (`sub_801A082`).
        local hit_mod = if battle.link() then 3 else 0
        user:reset_collision_types(collision.floating_navi, collision.navi_target, hit_mod)
    end
    wait(me, HUB_LOG_IN_AT)
end

-- `sub_80E96C6`: BugRSwrd and BgDthThd.
local function bug_charge(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer = WAIT
        local user = user_of(me)
        battle.effect(Vec3.new(user.pos.x, user.pos.y, 0), BUG_EFFECT)
        battle.play_sound(BUG_SOUND)
        local routine = (me.state :: State).routine
        battle.navi(me.alliance).charge_shot_routine = routine
        new_charged_shot(user, routine)
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
        local s = me.state :: State
        battle.effect(Vec3.new(p.x + dx * 0x10000, p.y, p.z), ARM_EFFECT, 0, s.palette)
        battle.navi(me.alliance).charge_shot_routine = s.routine
        new_charged_shot(user, s.routine)
    end
    wait(me, ARM_LOG_IN_AT)
end

-- `sub_80E9784`: DarkInvs.
local function dark_invis(me: Object)
    user_of(me):set_request("cross_special", true)
    dimming_chips.done(me)
end

-- `sub_80E95B4`.
local function effect(me: Object)
    local change = (me.state :: State).change
    if change == "hub" then
        hub_batch(me)
    elseif change == "bug" then
        bug_charge(me)
    elseif change == "arm" then
        arm(me)
    else
        dark_invis(me)
    end
end

navi_boost.kind = define.kind {
    id = "navi-boost",
    pool = "effect",
    state = { change = { "hub", "bug", "arm", "dark" }, routine = "u8", palette = "u8" },
    -- `sub_80E9570`.
    update = dimming_chips.phases { name = "a navi-changing chip", effect = effect },
}

-- `sub_80E979C`: the controller for `user`, making `change` (with the
-- weapon routine and palette the change takes).
function navi_boost.spawn(user: Object, spec: DimmingChipSpec, change: Change, routine: number, palette: number): Object?
    local o = dimming_chips.spawn(navi_boost.kind, user, spec)
    if o then
        local s = o.state :: State
        s.change, s.routine, s.palette = change, routine, palette
    end
    return o
end

-- HubBatc's `dimming` hook.
function navi_boost.hub(user: Object, spec: DimmingChipSpec): Object?
    return navi_boost.spawn(user, spec, "hub", 0, 0)
end

-- BugRSwrd's and BgDthThd's: weapon `routine` becomes the charged shot.
function navi_boost.bug(routine: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        return navi_boost.spawn(user, spec, "bug", routine, 0)
    end
end

-- An arm chip's: weapon `routine` becomes the charged shot, with a glow
-- of palette `palette` at the arm.
function navi_boost.arm(routine: number, palette: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        return navi_boost.spawn(user, spec, "arm", routine, palette)
    end
end

-- DarkInvs's.
function navi_boost.dark(user: Object, spec: DimmingChipSpec): Object?
    return navi_boost.spawn(user, spec, "dark", 0, 0)
end

return navi_boost
