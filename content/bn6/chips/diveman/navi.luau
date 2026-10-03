--!strict
-- DiveMan (the original's actor object #0xB, `sub_80B99C0`), the navi of
-- the DiveMan chips, brought by the navi chip controller. He surfaces,
-- waits, dives with the field shaking, and raises a tidal wave (./wave)
-- at his side's front edge of each row, then leaves. See
-- docs/engine/chips.md §3.6.27.

local navi_chips = require("../../lib/navi-chips/navi")
local wave = require("./wave")

local diveman = {}

export type State = {
    -- The controller waiting for him.
    controller: Object?,
}

local SPRITE = asset.sprite("bn6:diveman")
local SOUND_SURFACE = asset.sound("bn6:sand-worm")

-- What he is taken for: his actor record, and what he wears (his overlay, `sub_8010F96`).
local IDENTITY = define.identity {
    id = "bn6:diveman",
    class = "navi",
    version = 0,
    ai_index = 0xD,
    ice = "large",
    parts = {
        body = {
            sprite = asset.sprite("bn6:diveman"),
            anims = 136,
            behind = { { 0x0A, 0x0B }, { 0x23, 0x37 }, { 0x4C, 0x73 }, { 0x87, 0x87 } },
            anim_offset = 0x0A,
        },
    },
}

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B9B6F

-- The waves: in each row, from the far column toward him, the first
-- panel of his side's color (`byte_80B9BF0`, { require, forbid }), if it
-- is solid with no non-navi body or neutral object (`byte_80B9C04`).
local HIS_COLOR = { [0] = { 0, 0x20 }, [1] = { 0x20, 0 } }
local WAVE_FLOOR, WAVE_FORBID = 0x10, 0x0380_0000

-- His actions.
local SURFACE, WAIT, DIVE, WAVES, RISE, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14

-- `sub_80B99E4`: on his panel, on the ground (Z's whole part cleared),
-- with his parts.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground")
    me:add_parts(IDENTITY, 1)
    me:set_lifecycle("update")
end

-- `sub_80B9A60`: he surfaces; 19 ticks.
local function surface(me: Object)
    if me.phase == 0 then
        me.anim = 7
        battle.play_sound(SOUND_SURFACE)
        me.visible = true
    end
    if navi_chips.wait(me, 0x13, true) then
        me:set_action(WAIT)
    end
end

-- `sub_80B9A96`: 30 ticks.
local function wait(me: Object)
    if navi_chips.wait(me, 0x1E, true) then
        me:set_action(DIVE)
    end
end

-- `sub_80B9ABA`: he dives, the field shaking; 30 ticks.
local function dive(me: Object)
    if me.phase == 0 then
        me.anim = 4
        battle.shake_camera(0, 0x1E)
    end
    if navi_chips.wait(me, 0x1E, false) then
        me:set_action(WAVES)
    end
end

-- `sub_80B9B94`: a wave at his side's front edge of each row, its Z the
-- row - 1 (what the last panel check leaves in r3). (The game also
-- compares the column with his side's back column, to no effect.)
local function raise_waves(me: Object)
    local color = HIS_COLOR[me.alliance]
    local start = if me.alliance == 0 then 6 else 1
    local step = if me.alliance == 0 then -1 else 1
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    for row = 1, 3 do
        local x = start
        while field.valid(x, row) and not field.check(x, row, color[1], color[2]) do
            x += step
        end
        if field.valid(x, row) and field.check(x, row, WAVE_FLOOR, WAVE_FORBID) then
            wave.raise(me, x, row, me.element, damage, true, row - 1)
        end
    end
end

-- `sub_80B9AEA`: the waves; 60 ticks, the field shaking again at 30, the
-- dive's end at 20.
local function waves(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 5
        raise_waves(me)
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x1E then
        battle.shake_camera(2, 0x1E)
    elseif left == 0x14 then
        me.anim = 0
    elseif left <= 0 then
        me:set_action(RISE)
    end
end

-- `sub_80B9B30`: 14 ticks.
local function rise(me: Object)
    if me.phase == 0 then
        me.anim = 8
    end
    if navi_chips.wait(me, 0xE, false) then
        me:set_action(LEAVE)
    end
end

-- `sub_80B9B58`: at once, he goes (his parts with him) and the controller
-- moves on.
local function leave(me: Object)
    me:remove_parts(IDENTITY)
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = { [SURFACE] = surface, [WAIT] = wait, [DIVE] = dive, [WAVES] = waves, [RISE] = rise, [LEAVE] = leave }

diveman.kind = define.kind {
    id = "bn6:diveman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B99C0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("DiveMan action %#x reads past his table (off_80B9A48)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80B9B6E`: DiveMan for `user`, reporting to `controller` (the
-- DiveMan chips' navi hook, `off_802CD5C[13]`).
function diveman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(diveman.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return diveman
