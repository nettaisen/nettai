--!strict
-- The navi-changing chips' dimming controller (the original's effect
-- object #0x84, `sub_80E9570`): HubBatc, the arm chips (PunchArm,
-- NeedlArm, PuzzlArm, BoomrArm), BugRSwrd and BgDthThd, and DarkInvs. The
-- usual dimming phases (@exelib/dimming), with an effect, its chip's
-- change (`navi_boost.change`; the original picks a routine of
-- `off_80E95C8` by its first chip parameter):
--
--   HubBatc's (0, `sub_80E95D8`, chips/hubbatc): the buster at attack
--     level 5 or more, rapid and charge 4, the custom screen at 8, the
--     hand-shrink bug held off a turn, a B+Back special if the navi had
--     none, and the shoes and undershirt, then 60 ticks;
--   BugRSwrd's and BgDthThd's (1, `sub_80E96C6`, `navi_boost.bug`) and the
--     arms' (2, `sub_80E9714`, `navi_boost.arm`): the charged shot becomes
--     the chip's weapon, then 60 ticks;
--   DarkInvs's (3, `sub_80E9784`, chips/darkinvs): the navi's cross-special
--     request.
--
-- A chip's `dimming` hook (`sub_80E979C`, `off_802CCB4[38]`) is
-- `navi_boost.hook(change)`.

local dimming_chips = require("@exelib/dimming")

local navi_boost = {}

-- What a chip changes: the controller's effect, its chip's own.
export type Change = { run: (me: Object) -> () }

function navi_boost.change(run: (me: Object) -> ()): Change
    return new.record("navi-boost-change", { run = run })
end

export type State = {
    change: Change,
}

-- The ticks a change waits before the undim.
navi_boost.WAIT = 0x3C
-- The effects around the navi (the original's effects 0x5B and 0x5C), and
-- the sounds.
local BUG_EFFECT = new.effect { sprite = asset.sprite("bug"), anim = 0, palette = 0 }
local ARM_EFFECT = new.effect { sprite = asset.sprite("charge-glow-a"), anim = 3, palette = 1 }
local BUG_SOUND, LOG_IN_SOUND = asset.sound("bug"), asset.sound("log-in")
-- When the arms' log-in sound plays, by timer.
local ARM_LOG_IN_AT = 0x28

-- The controller's user (its first related object).
function navi_boost.user_of(me: Object): Object
    local user = me.related1
    if not user then
        error("the navi-changing chips' controller has no user (it reads through a null pointer)")
    end
    return user
end

-- `sub_8018810`: the user's attach point 0, facing the controller's way.
function navi_boost.attach_offset(me: Object, user: Object): (number, number)
    local x, y = user:attach_point(0)
    return x * user.facing * me.facing, y
end

-- Count down to the undim; `log_in_at`: when the log-in sound plays.
function navi_boost.wait(me: Object, log_in_at: number?)
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

navi_boost.kind = new.kind {
    id = "navi-boost",
    pool = "effect",
    state = { change = "record:navi-boost-change" },
    -- `sub_80E9570`, its effect `sub_80E95B4`.
    update = dimming_chips.phases {
        name = "a navi-changing chip",
        effect = function(me: Object)
            (me.state :: State).change.run(me)
        end,
    },
}

-- `sub_80E979C`: a chip's `dimming` hook, the controller for the user,
-- making `change`.
function navi_boost.hook(change: Change): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(navi_boost.kind, user, spec)
        if o then
            (o.state :: State).change = change
        end
        return o
    end
end

-- BugRSwrd's and BgDthThd's hook (`sub_80E96C6`): `weapon` becomes the
-- charged shot.
function navi_boost.bug(weapon: Weapon): (user: Object, spec: DimmingChipSpec) -> Object?
    return navi_boost.hook(navi_boost.change(function(me: Object)
        if me.phase == 0 then
            me.phase = 1
            me.timer = navi_boost.WAIT
            local user = navi_boost.user_of(me)
            battle.effect(Vec3.new(user.pos.x, user.pos.y, 0), BUG_EFFECT)
            battle.play_sound(BUG_SOUND)
            battle.navi(me.alliance).charge_shot_weapon = weapon
            new_charged_shot(user, weapon)
        end
        navi_boost.wait(me, nil)
    end))
end

-- An arm chip's hook (`sub_80E9714`): `weapon` becomes the charged shot,
-- with a glow of palette `palette` at the arm. (The game shifts the attach
-- point's X a second time where it means its Y, so the glow gets no
-- height.)
function navi_boost.arm(weapon: Weapon, palette: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return navi_boost.hook(navi_boost.change(function(me: Object)
        if me.phase == 0 then
            me.phase = 1
            me.timer = navi_boost.WAIT
            local user = navi_boost.user_of(me)
            local dx = navi_boost.attach_offset(me, user)
            local p = user.pos
            battle.effect(Vec3.new(p.x + dx * 0x10000, p.y, p.z), ARM_EFFECT, 0, palette)
            battle.navi(me.alliance).charge_shot_weapon = weapon
            new_charged_shot(user, weapon)
        end
        navi_boost.wait(me, ARM_LOG_IN_AT)
    end))
end

return navi_boost
