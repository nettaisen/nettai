--!strict
-- BlzrdBal's thrower (the original's actor object #4, `sub_80B8A18`;
-- docs/engine/dimming-chip-effects.md §10): a stand-in for the user
-- (@exelib/dimming/stand_in) that throws the snowball (./ball) from 10 pixels
-- in front of it after 35 ticks, unless it stands on a panel that isn't
-- solid, and gives the user back 91 ticks later.

local ball = require("./ball")
local stand_in = require("@exelib/dimming/stand_in")

local thrower = {}

export type State = {
    -- The chip's controller (@exelib/dimming/stand_in).
    controller: Object?,
}

local WINDUP, THROW_ANIM, THROW_ANIM_AT, AFTER = 0x22, 0xC, 4, 0x5A
local SOUND_WINDUP = asset.sound("blizzard-windup")
local SOUND_THROW = asset.sound("blizzard-throw")
-- The ball leaves 10 pixels in front of it.
local AHEAD = 0xA_0000
local SOLID = 0x10

-- `sub_80B8B08`: the windup; on a panel that isn't solid, no throw.
local function throw(me: Object)
    if me.phase_init == 0 then
        me.timer = WINDUP
        me.phase_init = 4
    end
    if me.timer == THROW_ANIM_AT then
        if not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
            me.timer = 1
            me.phase, me.phase_init = 4, 0
            return
        end
        me.anim = THROW_ANIM
        battle.play_sound(SOUND_WINDUP)
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    local p = me.pos
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    ball.spawn(me, int.i32(p.x + AHEAD * me.facing), p.y, me.element, damage)
    battle.play_sound(SOUND_THROW)
    me.timer = AFTER
    me.phase, me.phase_init = 4, 0
end

-- `sub_80B8B6E`: 91 ticks, then the user comes back.
local function done(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        stand_in.leave(me)
    end
end

-- `sub_80B8AD4`: shown (unless the local player is blind to it), by phase.
local function tick(me: Object)
    stand_in.show(me)
    local phase = me.phase
    if phase == 0 then
        throw(me)
    elseif phase == 4 then
        done(me)
    else
        error(string.format("BlzrdBal's thrower: phase %#x reads past its table", phase))
    end
end

thrower.kind = define.kind {
    id = "blzrdbal/thrower",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B8A18`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80B8A3C: drawn as the user, who vanishes; on at once.
            stand_in.init(me)
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

return thrower
