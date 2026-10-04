--!strict
-- NumbrBl's thrower (the original's actor object #0x45, `sub_80C3000`;
-- docs/engine/dimming-chip-effects.md §15): a stand-in for the user
-- (@exelib/dimming/stand_in) who raises the buster and sets balls (./ball)
-- down in front of himself, 18 ticks apart, then gives the user back. His
-- sprite steps while dimmed.

local stand_in = require("@exelib/dimming/stand_in")
local attachment = require("../../objects/attachment/attachment")
local buster = require("../../lib/buster")
local ball = require("./ball")

local numberman = {}

export type State = {
    -- The chip's controller (@exelib/dimming/stand_in).
    controller: Object?,
    -- The balls to set down (Param1: the chip's first parameter).
    balls: number,
    -- The buster arm (ExtraVars+4).
    held: Object?,
}

local WINDUP, BETWEEN, AFTER = 0x14, 0x12, 0x3C
-- When the arm comes down, before he goes.
local ARM_DOWN = 0x2D
local ANIM_IDLE, ANIM_THROW = 0, 0xE
-- The balls sit 16 pixels up and wait 10 ticks (plus one) before rolling.
local BALL_Z, BALL_WAIT = 0x10_0000, 0xA

-- The buster arm's animation by the user's identity (`sub_800F29C` of its
-- NameID: a player record's index 0, MegaMan, or 0x19 on, his Cross and
-- Beast forms, whose arms are their forms' animations); nil for the other
-- navis and anything not a player, who raise no arm.
local function arm_anim(user: Object): number?
    if user.actor_type ~= "player" then
        return nil
    end
    local stats = battle.navi(user.alliance)
    if not stats.navi.forms then
        return nil
    end
    local arm = stats.form.buster_arm
    return if arm then arm.anim else 0
end

local function next_action(me: Object, a: number)
    me.action, me.phase, me.phase_init = a, 0, 0
end

-- `sub_80C3100`: 21 ticks; then the throw's animation, the balls' count
-- and the arm.
local function windup(me: Object)
    if me.phase_init == 0 then
        me.timer = WINDUP
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    local s = me.state :: State
    me.anim = ANIM_THROW
    me.timer2 = s.balls
    local anim = arm_anim(me.related1 :: Object)
    if anim then
        attachment.attach(me, buster.arm, "held", { anim = anim, while_dimmed = true })
    end
    next_action(me, 4)
end

-- `sub_80C314E`: a ball every 18 ticks (the throw, the arm and the form's
-- overlay restarting with each), then 61 ticks.
local function throw(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 1
        me.anim_loaded = 0xFF
        local arm = s.held
        if arm then
            arm.anim_loaded = 0xFF
            local overlay = me.related2
            if overlay then
                overlay.anim_loaded = 0xFF
            end
        end
        me.timer = BETWEEN
        ball.spawn(me, me.panel_x + me.facing, me.panel_y, BALL_Z, BALL_WAIT)
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local n = me.timer2 - 1
    me.timer2 = n
    if n ~= 0 then
        me.phase_init = 0
        return
    end
    me.timer = AFTER
    next_action(me, 8)
end

-- `sub_80C31B0`: the arm let go 15 ticks in; then the user back.
local function done(me: Object)
    if me.timer == ARM_DOWN then
        (me.state :: State).held = nil
        me.anim = ANIM_IDLE
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        stand_in.leave(me)
    end
end

-- `sub_80C30C2`: shown (unless the local player is blind to it), by
-- action.
local function tick(me: Object)
    stand_in.show(me)
    local a = me.action
    if a == 0 then
        windup(me)
    elseif a == 4 then
        throw(me)
    elseif a == 8 then
        done(me)
    else
        error(string.format("NumbrBl's thrower: action %#x reads past off_80C30F4", a))
    end
end

numberman.kind = define.kind {
    id = "numbrbl/numberman",
    pool = "actor",
    state = { controller = "object", balls = "u8", held = "object" },
    -- `sub_80C3000`: by state, then the sprite (stepping while dimmed).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80C3024: drawn as the user, who vanishes; his update
            -- first runs next tick.
            stand_in.init(me)
            me:set_lifecycle("update")
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80C31F0`: NumberMan for `user` on `controller`'s panel, setting down
-- `balls` balls with the damage word `damage`.
function numberman.spawn(controller: Object, user: Object, damage: number, balls: number): Object?
    local o = stand_in.spawn(numberman.kind, controller, user, damage)
    if o then
        (o.state :: State).balls = balls
    end
    return o
end

return numberman
