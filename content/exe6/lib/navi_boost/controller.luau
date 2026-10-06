--!strict
-- The navi-changing chips' dimming controller (the original's effect
-- object #0x84, `sub_80E9570`): HubBatc, the arm chips (PunchArm,
-- NeedlArm, PuzzlArm, BoomrArm), BugRSwrd and BgDthThd, and DarkInvs. The
-- usual dimming phases (@exelib/dimming), with an effect by what the chip
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
-- `navi_boost.hub`, `navi_boost.bug(weapon)`, `navi_boost.arm(weapon,
-- palette)` or `navi_boost.dark`.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local shield = require("../../navis/megaman/weapons/shield")

local navi_boost = {}

export type Change = "hub" | "bug" | "arm" | "dark"

export type State = {
    -- What the chip changes.
    change: Change,
    -- The weapon that becomes the charged shot (the original's second chip
    -- parameter names its routine), and the arm effect's palette (its
    -- third).
    weapon: Weapon?,
    palette: number,
}

local WAIT = 0x3C
-- The effects around the navi (the original's effects 0x5A to 0x5C), and
-- the sounds.
local HUB_EFFECT = define.effect { sprite = asset.sprite("hub"), anim = 0, palette = 0 }
local BUG_EFFECT = define.effect { sprite = asset.sprite("bug"), anim = 0, palette = 0 }
local ARM_EFFECT = define.effect { sprite = asset.sprite("charge-glow-a"), anim = 3, palette = 1 }
local HUB_SOUND, BUG_SOUND, LOG_IN_SOUND = asset.sound("hub"), asset.sound("bug"), asset.sound("log-in")
-- When the log-in sound plays, by timer.
local HUB_LOG_IN_AT, ARM_LOG_IN_AT = 0x1C, 0x28
-- HubBatc's buster, custom screen and B+Back special (the NaviCust
-- Shield).
local MIN_BUSTER, HUB_RAPID, HUB_CHARGE, HUB_CUSTOM = 5, 4, 4, 8
local HUB_BACK_SPECIAL = shield.shield

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

-- `sub_80E97BE`: the navi's charged shot is `weapon` now; a buster with a
-- plain counterpart gives way to it (the Beast busters to the plain
-- buster, the Beast's throw of an absorbed obstacle to the Cross's).
local function new_charged_shot(user: Object, weapon: Weapon?)
    user.charge_shot_weapon = weapon
    local buster = user.buster_weapon
    if buster and buster.plain then
        user.buster_weapon = buster.plain
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
        if not stats.back_special_weapon then
            stats.back_special_weapon = HUB_BACK_SPECIAL
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
        local weapon = (me.state :: State).weapon
        battle.navi(me.alliance).charge_shot_weapon = weapon
        new_charged_shot(user, weapon)
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
        battle.navi(me.alliance).charge_shot_weapon = s.weapon
        new_charged_shot(user, s.weapon)
    end
    wait(me, ARM_LOG_IN_AT)
end

-- `sub_80E9784`: DarkInvs.
local function dark_invis(me: Object)
    -- (EXE6's rules/beast starts its Cross special: rules/beast/
    -- cross-special.luau.)
    user_of(me):set_request("takeover", true)
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
    state = { change = { "hub", "bug", "arm", "dark" }, weapon = "weapon", palette = "u8" },
    -- `sub_80E9570`.
    update = dimming_chips.phases { name = "a navi-changing chip", effect = effect },
}

-- `sub_80E979C`: the controller for `user`, making `change` (with the
-- weapon and palette the change takes).
function navi_boost.spawn(user: Object, spec: DimmingChipSpec, change: Change, weapon: Weapon?, palette: number): Object?
    local o = dimming_chips.spawn(navi_boost.kind, user, spec)
    if o then
        local s = o.state :: State
        s.change, s.weapon, s.palette = change, weapon, palette
    end
    return o
end

-- HubBatc's `dimming` hook.
function navi_boost.hub(user: Object, spec: DimmingChipSpec): Object?
    return navi_boost.spawn(user, spec, "hub", nil, 0)
end

-- BugRSwrd's and BgDthThd's: `weapon` becomes the charged shot.
function navi_boost.bug(weapon: Weapon): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        return navi_boost.spawn(user, spec, "bug", weapon, 0)
    end
end

-- An arm chip's: `weapon` becomes the charged shot, with a glow of palette
-- `palette` at the arm.
function navi_boost.arm(weapon: Weapon, palette: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        return navi_boost.spawn(user, spec, "arm", weapon, palette)
    end
end

-- DarkInvs's.
function navi_boost.dark(user: Object, spec: DimmingChipSpec): Object?
    return navi_boost.spawn(user, spec, "dark", nil, 0)
end

return navi_boost
