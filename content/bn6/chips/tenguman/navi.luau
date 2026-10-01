--!strict
-- TenguMan, the navi of the TenguMan chips (the original's actor object
-- #0x0C, `sub_80B9C14`), brought by their navi chip controller. He
-- appears, leaps out of sight, then sweeps the field himself, his body a
-- hit region (a column of three): first dashing along the back row from
-- behind his side's
-- edge, speeding up, then diving diagonally across from the top row.
-- Each time he reaches a new column his region comes back on (a hit turns
-- it off). Then he goes, and the controller moves on. See
-- docs/engine/chips.md §3.6.13.

local panels = require("../../lib/panels")
local navi_chips = require("../../lib/navi-chips/navi")
local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

export type State = {
    -- The controller waiting for him (its pointer sits in his
    -- ExtraVars[0]).
    controller: Object?,
}

local tenguman = {}

local SPRITE = asset.sprite("tenguman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_TENGU_MAN = asset.sound("tengu-man")

-- His body's hit region while he sweeps, and his hit's modifier.
local REGION = regions.wide
local HIT_MOD = 3

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B9F0F

-- By side: where each sweep starts (`dword_80B9E34`, `dword_80B9EF4`) and
-- where its highlighted region sits (`dword_80B9E3C`, `off_80B9EFC`).
local DASH_FROM = { [0] = { 0, 3 }, { 7, 3 } }
local DASH_MARK = { [0] = { 1, 3 }, { 6, 3 } }
local DIVE_FROM = { [0] = { 2, 0 }, { 5, 0 } }
local DIVE_MARK = { [0] = { 4, 2 }, { 3, 2 } }
-- The regions he highlights: the dash's (region 0x23: three rows of six
-- from the mark) and the dive's (0x25: a band slanting across the mark).
local DASH_REGION = define.region {
    panels = {
        { 0, -1 }, { 1, -1 }, { 2, -1 }, { 3, -1 }, { 4, -1 }, { 5, -1 },
        { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 },
        { 0, 1 }, { 1, 1 }, { 2, 1 }, { 3, 1 }, { 4, 1 }, { 5, 1 },
    },
}
local DIVE_REGION = define.region {
    panels = { { -2, -1 }, { -1, -1 }, { 0, -1 }, { -1, 0 }, { 0, 0 }, { 1, 0 }, { 0, 1 }, { 1, 1 }, { 2, 1 } },
}

-- The dash: its speed and acceleration forward; the dive: its speed
-- forward and down (16.16).
local DASH_SPEED, DASH_ACCEL = 0x5_0000, 0xA000
local DIVE_SPEED_X, DIVE_SPEED_Y = 0xC_0000, 0x7_3333

-- Z's whole part (a halfword store), its fraction kept.
local function set_height(me: Object, pixels: number)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(bit32.band(pixels, 0xFFFF), 16)))
end

-- `sub_80B9C38`: on his panel, on the ground; collision on, region off.
-- Without a collision slot he goes at once (a byte store of the state).
local function init(me: Object)
    me:set_coordinates_from_panel()
    set_height(me, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me.lifecycle = "destroy"
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.heavy)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4, a word store of the timers), or count down;
-- true once the timer is 0 or below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer, me.timer2 = ticks, 0
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- `sub_80B9CDC`: he appears; 3 ticks.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 3) then
        me:set_action(4)
    end
end

-- `sub_80B9D12`: 30 ticks.
local function stand(me: Object)
    if countdown(me, 0x1E) then
        me:set_action(8)
    end
end

-- `sub_80B9D36`: the leap, 23 ticks; at 20 left he is out of sight (Z 255,
-- no ground shadow).
local function leap(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 4
        me.timer = 0x17
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x14 then
        set_height(me, 0xFF)
        me.sprite.shadow = "with_sprite"
        return
    end
    if left <= 0 then
        me:set_action(0xC)
    end
end

-- A sweep's start: region on, 16 pixels up with a shadow, on `from`.
local function start_sweep(me: Object, anim: number, from: { number })
    me.anim = anim
    set_height(me, 0x10)
    me.sprite.shadow = "ground"
    me.panel_x, me.panel_y = from[1], from[2]
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- Move one tick, the panel and collision following; the column he was
-- in.
local function sweep(me: Object): number
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    local old_x = me.panel_x
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    return old_x
end

-- Highlight the sweep's region 4 ticks out of 8.
local function highlight(me: Object, mark_region: Region, mark: { number })
    local t = me.timer2 + 1
    me.timer2 = t
    if bit32.band(t, 4) == 0 then
        panels.highlight_region(mark[1], mark[2], mark_region, me.facing)
    end
end

-- `sub_80B9D6E`: the dash along the back row, 40 ticks, speeding up
-- (his cry at 30 left).
local function dash(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.collision:set_region(REGION)
        start_sweep(me, 0x13, DASH_FROM[me.alliance])
        me.vel = Vec3.new(me.facing * DASH_SPEED, 0, me.vel.z)
        me.timer = 0x28
        me.timer2 = 0
        return
    end
    local old_x = sweep(me)
    local v = me.vel
    me.vel = Vec3.new(v.x + me.facing * DASH_ACCEL, v.y, v.z)
    if me.panel_x ~= old_x then
        me.collision:set_region(REGION)
    end
    highlight(me, DASH_REGION, DASH_MARK[me.alliance])
    local left = me.timer - 1
    me.timer = left
    if left == 0x1E then
        battle.play_sound(SOUND_TENGU_MAN)
        return
    end
    if left <= 0 then
        me:set_action(0x10)
    end
end

-- `sub_80B9E40`: the dive across the field from the top row
-- (0x28_0000 * 5 / DIVE_SPEED_X ticks, a BIOS division; his cry);
-- then his region goes, and so does he (a byte store of the state).
local function dive(me: Object)
    if me.phase == 0 then
        me.phase = 4
        start_sweep(me, 0x14, DIVE_FROM[me.alliance])
        me.vel = Vec3.new(me.facing * DIVE_SPEED_X, DIVE_SPEED_Y, me.vel.z)
        me.timer = (0x28_0000 * 5) // DIVE_SPEED_X
        battle.play_sound(SOUND_TENGU_MAN)
        me.timer2 = 0
        return
    end
    local old_x = sweep(me)
    if me.panel_x ~= old_x then
        me.collision:set_region(REGION)
    end
    highlight(me, DIVE_REGION, DIVE_MARK[me.alliance])
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision:set_region(nil)
    me.lifecycle = "destroy"
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = leap, [0xC] = dash, [0x10] = dive }

-- `sub_80B9C9C`: resolve the hits (a hit turns his region off), act,
-- register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("TenguMan action %#x reads past his table (off_80B9CC8)", me.action))
    end
    action(me)
    c:present()
end

-- `sub_80B9F00`: the controller moves on, and he goes (his collision slot
-- stays taken, as in the game).
local function finish(me: Object)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

tenguman.kind = define.kind {
    id = "tenguman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B9C14`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            finish(me)
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80B9F0E`: TenguMan for `user`, reporting to `controller` (the
-- TenguMan chips' navi hook, `off_802CD5C[9]`). His position is the
-- spawner's registers until his init places him.
function tenguman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(tenguman.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return tenguman
