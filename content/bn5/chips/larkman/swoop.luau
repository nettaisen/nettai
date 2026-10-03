--!strict
-- LarkMan's swoop (BN5's attack object #0xBA, 0x080DD738; BN5's own): LarkMan's body as a hit, which flies in from
-- below the field two columns behind its navi, diagonally up across three columns, circles back round a center 20
-- pixels ahead of where it turns, and flies off backward. The navi chip's (variant 5, from LarkMan's navi) acts
-- and animates while the battle is dimmed. With the navi's command entered (./navi), its body hits for nothing and
-- a follower (./follower) carries its damage.
--
-- WildBird's (variant 6, from the summon table's entry 23's LarkMan) flies faster (row 6), turns faster for 25 more
-- ticks with its own animations by its side (0x080DDC44), and takes no command.
--
-- Not ported: the variants LarkMan the virus's AI spawns (0 to 4, 0x08100942: its own start panels, speeds and
-- turn, a warning effect, waiting on the dimming and on its owner's attack).

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local regions = require("@common/regions")
local navi_chips = require("@common/navi-chips/navi")
local trajectory = require("../../lib/trajectory")
local follower = require("./follower")

local swoop = {}

export type State = {
    -- The navi chip's level, which picks the palette (+0x30), and its variant (Param1: 5, or WildBird's 6).
    level: number,
    variant: number,
    -- It faces left (+5: its side ^ flip).
    left: boolean,
    -- Its column at the start (+6), and its look: 1 facing right, 3 left (+7: from below the field).
    start_x: number,
    dir: number,
    -- The turn: its radius and center (+0x60, +0x6C, +0x70), its angle (16.16 of 256 a turn, +0x68) and step
    -- (+0x64), the animation steps taken (+0x74).
    radius: number,
    center_x: number,
    center_y: number,
    angle: number,
    angle_step: number,
    anim_step: number,
    -- The navi's command was entered (+0x7C), and the follower carrying its damage (+0x48).
    powered: boolean,
    follower: Object?,
}

local SPRITE = asset.sprite("bn5:larkman")
local SOUND_SWOOP = asset.sound("bn5:swoop")
local HIT_MOD = 3

-- By the level (0x080DD844): the dark palette for DS's 4.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 0, 4 }
-- By look (0x080DD850: 0 and 2, from above the field, are the virus's).
local ANIMS: { [number]: number } = { [0] = 8, 6, 9, 0xB }
-- By variant (5 the navi chip's, 6 WildBird's): the speed (0x080DD858's rows), the turn's step (0x080DDB7C's rows:
-- 16.16 of 256 steps), the ticks it lasts less (0x080DDB9C's rows) and more (WildBird's 25).
type Variant = { speed_x: number, speed_y: number, turn_step: number, shorter: number, longer: number }
local VARIANTS: { [number]: Variant } = {
    [5] = { speed_x = 0x8_0000, speed_y = 0x4_CCCC, turn_step = 0xD_AC50, shorter = 1, longer = 0 },
    [6] = { speed_x = 0xC_0000, speed_y = 0x7_3333, turn_step = 0x14_8278, shorter = 0, longer = 25 },
}
local function variant(s: State): Variant
    local v = VARIANTS[s.variant]
    if not v then
        error(string.format("LarkMan's swoop variant %d is a virus's (0x08100942), not ported", s.variant))
    end
    return v
end
-- WildBird's turn animations and the angles each waits for, by its side (0x080DDC88, 0x080DDCA4: eleven each).
local WILD_TURN_ANIMS: { [number]: { number } } = {
    [0] = { 8, 9, 0xB, 6, 8, 9, 0xB, 6, 8, 9, 0xB },
    [1] = { 9, 8, 6, 0xB, 9, 8, 6, 0xB, 9, 8, 6 },
}
local WILD_TURN_ANGLES: { [number]: { number } } = {
    [0] = { 0xC0, 0x100, 0x140, 0x180, 0x1C0, 0x200, 0x240, 0x280, 0x2C0, 0x300, 0x340 },
    [1] = { 0xC0, 0x80, 0x40, 0, -0x40, -0x80, -0xC0, -0x100, -0x140, -0x180, -0x1C0 },
}
-- The turn's animations and the angles each waits for, by look (0x080DDC1C, 0x080DDC2C: three each).
local TURN_ANIMS: { [number]: { number } } = { [0] = { 6, 0xB, 9 }, { 8, 9, 0xB }, { 0xB, 6, 8 }, { 9, 8, 6 } }
local TURN_ANGLES: { [number]: { number } } =
    { [0] = { 0x40, 0x00, 0xC0 }, { 0xC0, 0x100, 0x140 }, { 0x40, 0x80, 0xC0 }, { 0xC0, 0x80, 0x40 } }
-- The sine table (math_sinTable), as the turn reads it.
local sine = trajectory.sine

-- 0x080DDA94 (the navi chip's): two columns behind its navi, below the field, 32 pixels up.
local function place(me: Object, s: State)
    me.panel_y = 4
    local navi = me.related1 :: Object
    local x = navi.panel_x + (if s.left then 2 else -2)
    me.panel_x = int.u8(x)
    s.start_x = int.u8(x)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0x20_0000 + bit32.band(p.z, 0xFFFF))
end

-- 0x080DD76C.
local function init(me: Object)
    local s = me.state :: State
    s.left = bit32.bxor(me.alliance, me.flip) ~= 0
    place(me, s)
    s.dir = (if s.left then 2 else 0) + (if me.panel_y ~= 0 then 1 else 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    local anim = ANIMS[s.dir]
    me.anim, me.anim_loaded = anim, anim
    sprite:set_animation(anim)
    s.anim_step = 0
    sprite.palette = PALETTES[s.level] or 0
    local v = variant(s)
    me.vel = Vec3.new(if s.left then -v.speed_x else v.speed_x, if me.panel_y == 4 then -v.speed_y else v.speed_y, me.vel.z)
    s.powered = false
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_3e, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.plain)
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
    end
    me.collision:present()
    battle.play_sound(SOUND_SWOOP)
    me:set_lifecycle("update")
end

-- 0x080DDD06: the follower goes where the body is.
local function lead(me: Object, s: State)
    local f = s.follower
    if s.powered and f then
        local p, q = me.pos, f.pos
        f.pos = Vec3.new(p.x, p.y, q.z)
    end
end

-- After a move: the panel under it, and its hit there when that changed.
local function moved(me: Object, x: number, y: number)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= x or me.panel_y ~= y then
        me.collision:set_region(regions.single)
    end
end

-- 0x080DDAFC: the turn starts, round a center 20 pixels ahead on the middle row's.
local function start_turn(me: Object, s: State)
    s.radius = 0x1F
    s.center_x = int.u32(me.pos.x + (if s.left then -0x14_0000 else 0x14_0000))
    s.center_y = 0x1C_0000
    local dx = if s.left then 0x14_0000 else -0x14_0000
    local dy = if me.panel_y == 3 then 0x18_0000 else -0x18_0000
    local angle = trajectory.angle(dx, dy)
    me.timer2 = angle
    s.angle = int.shl(angle, 16)
    local v = variant(s)
    local step = v.turn_step
    if not s.left then
        step = -step
    end
    if me.panel_y ~= 3 then
        step = -step
    end
    s.angle_step = step
    me.timer = int.tdiv(0xB8, bit32.rshift(v.turn_step, 16)) - v.shorter + v.longer
end

-- Action 0 (0x080DD900): diagonally up; on reaching the center of the third column ahead, the turn.
local function rise(me: Object)
    local s = me.state :: State
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
    end
    lead(me, s)
    local center = field.center(me.panel_x, me.panel_y)
    local x, y = me.panel_x, me.panel_y
    local p, v = me.pos, me.vel
    local old_x = p.x
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), p.z)
    if center ~= old_x and navi_chips.between(me.pos.x, old_x, center) then
        me:set_coordinates_from_panel()
        if int.u8(me.facing * 3 + s.start_x) == me.panel_x then
            start_turn(me, s)
            me:set_action(4)
            return
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if field.valid(me.panel_x, me.panel_y) and (me.panel_x ~= x or me.panel_y ~= y) then
        me.collision:set_region(regions.single)
    end
end

-- 0x080DDBD4: the next of the turn's animations, once the angle passes its mark.
local function turn_anim(me: Object, s: State)
    local n = s.anim_step
    if n >= 3 then
        return
    end
    local mark = TURN_ANGLES[s.dir][n + 1]
    local angle = me.timer2
    if s.dir == 0 or s.dir == 3 then
        if angle > mark then
            return
        end
    elseif angle < mark then
        return
    end
    me.anim = TURN_ANIMS[s.dir][n + 1]
    s.anim_step = n + 1
end

-- 0x080DDC44 (WildBird's): the next of its eleven turn animations, once the angle (signed) passes its mark.
local function wild_turn_anim(me: Object, s: State)
    local n = s.anim_step
    if n >= 11 then
        return
    end
    local side = if s.left then 1 else 0
    local mark = WILD_TURN_ANGLES[side][n + 1]
    local angle = int.i16(me.timer2)
    if side == 1 then
        if angle > mark then
            return
        end
    elseif angle < mark then
        return
    end
    me.anim = WILD_TURN_ANIMS[side][n + 1]
    s.anim_step = n + 1
end

-- Action 4 (0x080DD99E): round the center; then off.
local function turn(me: Object)
    local s = me.state :: State
    lead(me, s)
    local x, y = me.panel_x, me.panel_y
    s.angle = int.u32(s.angle + s.angle_step)
    me.timer2 = bit32.rshift(s.angle, 16)
    -- 0x080DDBA4.
    local a = bit32.band(me.timer2, 0xFF)
    local p = me.pos
    me.pos = Vec3.new(
        int.i32(s.center_x + int.shl(int.mul32(sine(a + 64), s.radius), 8)),
        int.i32(s.center_y + int.shl(int.mul32(sine(a), s.radius), 8)),
        p.z
    )
    moved(me, x, y)
    if s.variant == 6 then
        wild_turn_anim(me, s)
    else
        turn_anim(me, s)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(8)
    end
end

-- Action 8 (0x080DDA00): off backward, gone off the field.
local function leave(me: Object)
    local s = me.state :: State
    lead(me, s)
    local x, y = me.panel_x, me.panel_y
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x - v.x), int.i32(p.y + v.y), p.z)
    moved(me, x, y)
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = rise, [4] = turn, [8] = leave }

-- 0x080DDCD0: the navi's command was entered: its body hits for nothing, white-blue, and a follower carries its
-- damage.
local function power_up(me: Object, s: State)
    if s.powered then
        return
    end
    local navi = me.related1
    if not (navi and navi.state.commanded) then
        return
    end
    s.powered = true
    me:reset_collision_types(collision.collision_46, collision.hits_navis, HIT_MOD)
    me.collision.self_damage = 0
    me.sprite.color_shader = 0x1E0
    s.follower = follower.spawn(me, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
end

-- 0x080DD890: resolve the hits; act; register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me.lifecycle = "destroy"
        return
    end
    local s = me.state :: State
    if s.variant == 5 then
        power_up(me, s)
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("LarkMan's swoop action %#x reads past its table (0x080DD8F4)", me.action))
    end
    action(me)
    c:present()
end

swoop.kind = define.kind {
    id = "bn5:larkman/swoop",
    pool = "attack",
    state = {
        level = "u8",
        variant = "u8",
        left = "bool",
        start_x = "u8",
        dir = "u8",
        radius = "u32",
        center_x = "u32",
        center_y = "u32",
        angle = "u32",
        angle_step = "i32",
        anim_step = "u8",
        powered = "bool",
        follower = "object",
    },
    -- 0x080DD738.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080DDA60: LarkMan's swoop from `navi` of `variant` (the navi chip's 5, WildBird's 6), the chip's level, element
-- and damage word.
function swoop.spawn(navi: Object, level: number, element: number, damage: number, variant: number): Object?
    local o = battle.spawn(swoop.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.level = level
        s.variant = variant
        o.element = element
        o.related1 = navi
        o:set_damage_word(damage)
        o.alliance, o.flip = navi.alliance, navi.flip
        o.run_while_dimmed = true
    end
    return o
end

return swoop
