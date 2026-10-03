--!strict
-- A capsule (BN5's attack object #0x0C, 0x080C5F18; BN5's own): thrown from its thrower's panel in an arc to a
-- target panel in 40 ticks, the target lit as it flies. A body with 20 HP that the other side can hit (hit modifier
-- by its variant), it hits whatever stands on the target as it comes down there (its last 4 ticks). It bursts (an
-- explosion, a sound) when its HP runs out, when the other side's navi is hit, or when it lands on one; on a panel
-- that isn't solid it is gone; else it sits 20 ticks (50 when running while dimmed), blinking, then bursts: a
-- one-tick hit along its line (its second parameter: 0 its column, 1 its row; thrown, hitting nothing, its
-- variant's hit modifier and status) and an explosion on each of the line's panels. Its variant (its first
-- parameter, 0 to 5) gives its palette, its explosions' palette, its hit modifier and its status (none, confusion,
-- blindness, immobilization, paralysis 90 and 150 ticks); one not running while dimmed (its third parameter not 2)
-- of variant 1 and up hits half as hard. Its target is marked in the battle flag 0x40 mode (objects/dice's marker).

local collision = require("../../rules/collision")
local marker = require("../../lib/navi-chips/marker")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- Its variant (its first parameter: 0 to 5), its line (its second: 0 its column, 1 its row), and its third (2:
    -- it runs while dimmed).
    variant: number,
    line: number,
    mode: number,
}

local capsule = {}

local SPRITE = asset.sprite("bn5:meddy-capsule")
local EXPLOSION = asset.sprite("bn5:explosion")
local SOUND_BURST = asset.sound("bn5:hit-bomb-1")

type Variant = {
    -- Its palette (0x080C5FE8), the hit modifier of its body and of its burst (0x080C5FF4, 0x080C62A8), its
    -- status (0x080C6010, the same as its burst's), and its explosions (the effect table's row 0, its palette
    -- raised by 0x080C62C4's).
    palette: number,
    hit_mod: number,
    burst_hit_mod: number,
    status: Status?,
    explosion: Effect,
}

-- The effect table's row 0 (an explosion), its palette raised.
local EXPLOSIONS: { [number]: Effect } = {}
for palette = 0, 4 do
    EXPLOSIONS[palette] = define.effect { sprite = EXPLOSION, anim = 0, palette = palette }
end

local VARIANTS: { [number]: Variant } = {
    [0] = { palette = 0, hit_mod = 3, burst_hit_mod = 3, status = nil, explosion = EXPLOSIONS[0] },
    [1] = { palette = 1, hit_mod = 3, burst_hit_mod = 3, status = statuses.confuse_480, explosion = EXPLOSIONS[4] },
    [2] = { palette = 2, hit_mod = 3, burst_hit_mod = 3, status = statuses.blind_480, explosion = EXPLOSIONS[1] },
    [3] = { palette = 3, hit_mod = 1, burst_hit_mod = 1, status = statuses.immobilize_60, explosion = EXPLOSIONS[2] },
    [4] = { palette = 4, hit_mod = 0, burst_hit_mod = 0, status = statuses.paralyze_90, explosion = EXPLOSIONS[3] },
    [5] = { palette = 4, hit_mod = 0, burst_hit_mod = 0, status = statuses.paralyze_150, explosion = EXPLOSIONS[3] },
}
-- By its line (0x080C628C, 0x080C6298, 0x080C633C): its burst's region, the panels it explodes on, and its
-- target's marker style.
type Line = { region: Region, panels: { { number } }, style: number }
local LINES: { [number]: Line } = {
    [0] = { region = regions.wide, panels = { { 0, 0 }, { 0, 1 }, { 0, -1 } }, style = 4 },
    [1] = { region = regions.region_1d, panels = { { 0, 0 }, { -1, 0 }, { 1, 0 } }, style = 0x1D },
}
-- The other side's navi's body among what hit it, by side (0x080C60A8), and a navi's body on a panel (0x080C61B0).
local NAVI_HIT = { [0] = 0x0580_0000, 0x0A80_0000 }
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local SOLID = 0x10
local HP = 0x14
local FLIGHT = 0x28
local GRAVITY = -0xC000
-- Its color while it blinks (0x080C61FC).
local BLINK = { [0] = 0, [4] = 0x3DEF }

local function variant_of(me: Object): Variant
    local v = VARIANTS[(me.state :: State).variant]
    if not v then
        error(string.format("a capsule's variant %d reads past its tables (0x080C5FE8)", (me.state :: State).variant))
    end
    return v
end

local function line_of(me: Object): Line
    local l = LINES[(me.state :: State).line]
    if not l then
        error(string.format("a capsule's line %d reads past its tables (0x080C628C)", (me.state :: State).line))
    end
    return l
end

local function while_dimmed(me: Object): boolean
    return (me.state :: State).mode == 2
end

-- Gone at its next update (the lifecycle and action halfword stored together).
local function gone(me: Object)
    me.phase_init = 0
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
    me.action = 0
    me.phase = 0
    me.timer2 = 0
end

-- It bursts where it is: an explosion, a sound; gone.
local function blow_up(me: Object)
    battle.play_sound(SOUND_BURST)
    battle.effect(me.pos, EXPLOSIONS[0])
end

-- Action 0 (0x080C60B0; its phase byte the entry flag): aimed on the first tick to come down 8 pixels below its
-- target's center in 40 ticks; its target lit 8 ticks of every 16; in its last 4 ticks, over its target, it hits
-- there. Landed: on a navi of the other side it bursts at once; on a solid panel it sits; else it is gone.
local function fly(me: Object)
    local fx, fy = me.future_panel_x, me.future_panel_y
    if me.phase == 0 then
        me.phase = 4
        local x, y = field.center(fx, fy)
        me.vel = trajectory.in_ticks(me.pos, x, y, -0x8_0000, FLIGHT, GRAVITY)
        me.timer = FLIGHT
        me.timer2 = 0
        return
    end
    me.timer2 = int.u16(me.timer2 + 1)
    if bit32.band(me.timer2, 8) == 0 then
        field.highlight(fx, fy)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        local flags = field.flags(me.panel_x, me.panel_y)
        if bit32.band(flags, SOLID) ~= 0 then
            if bit32.band(flags, ENEMY_NAVI[me.alliance]) == 0 then
                me.anim = (me.state :: State).line
                local p = me.pos
                me.pos = Vec3.new(p.x, p.y, 0)
                me.action = 4
                me.phase, me.phase_init = 0, 2
                me.timer2 = 0
                return
            end
            blow_up(me)
        end
        gone(me)
        return
    end
    if left <= 3 and me.panel_x == fx and me.panel_y == fy then
        me.collision:set_region(regions.single)
        me.phase_init = 1
    end
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(int.i32(p.x + v.x), int.i32(p.y + v.y), int.i32(p.z + v.z))
    me.vel = Vec3.new(v.x, v.y, int.i32(v.z + GRAVITY))
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- Action 4 (0x080C61B8; its phase byte the entry flag): it sits 21 ticks (51 running while dimmed), blinking every
-- 4; its target marked.
local function sit(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = if while_dimmed(me) then 0x32 else 0x14
        return
    end
    me.sprite.color_shader = BLINK[bit32.band(me.timer, 4)]
    marker.spawn(me, me.future_panel_x, me.future_panel_y, 1, line_of(me).style, true)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me.action = 8
        me.phase = 0
    end
end

-- Action 8 (0x080C6204): it bursts (a sound): a one-tick hit along its line, running while dimmed, and an
-- explosion on each of the line's panels on the field; gone.
local function burst(me: Object)
    local v, l = variant_of(me), line_of(me)
    me.collision:set_region(nil)
    battle.play_sound(SOUND_BURST)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = l.region,
        hit_effect = sparks.plain,
        target = collision.nothing,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = v.burst_hit_mod,
        status = v.status,
    })
    if box then
        box.run_while_dimmed = true
    end
    for _, o in l.panels do
        local x, y = me.panel_x + o[1], me.panel_y + o[2]
        if x >= 1 and x <= 6 and y >= 1 and y <= 3 then
            local cx, cy = field.center(x, y)
            battle.effect(Vec3.new(cx, cy, 0), v.explosion)
        end
    end
    me:set_lifecycle("destroy")
    me.action = 0
    me.phase = 0
end

local ACTIONS = { [0] = fly, [4] = sit, [8] = burst }

-- 0x080C6028: the hits against it resolve and spark; at the battle's end it is gone; out of HP, or hitting the
-- other side's navi, it bursts; else its action (unless dimmed and not running while dimmed); its target marked
-- as it flies.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me:take_damage(0) < 0 or bit32.band(c.hit_flags, NAVI_HIT[me.alliance]) ~= 0 then
        blow_up(me)
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if while_dimmed(me) or not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("a capsule's action %#x reads past its table (0x080C6098)", me.action))
        end
        local flying = me.action == 0
        action(me)
        if flying then
            -- 0x080C6306: its target marked a tick.
            marker.spawn(me, me.future_panel_x, me.future_panel_y, 1, 1, false)
        end
    end
    if me.lifecycle == "destroy" then
        c:set_region(nil)
        return
    end
    c:present()
end

-- 0x080C5F4C: on its thrower's panel, 32 pixels up; its sprite (animation 2, its shadow), shown, its variant's
-- palette, facing its way; its damage halved (variant 1 and up, not running while dimmed); 20 HP; its collision
-- (hit by the other side, hitting it back, its variant's hit modifier and status, no spark), no region yet.
local function init(me: Object)
    local s = me.state :: State
    local v = variant_of(me)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0x20_0000)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 2, 2
    sprite:set_animation(2)
    sprite.palette = v.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if s.mode == 0 and s.variant ~= 0 then
        me.damage = bit32.rshift(me.damage, 1)
    end
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_thrown_body, collision.hit_by_other_side, v.hit_mod)
    me.hp, me.max_hp = HP, HP
    local c = me.collision
    c:set_hit_effect(nil)
    c.status_base = v.status
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

capsule.kind = define.kind {
    id = "bn5:capsule",
    pool = "attack",
    state = { variant = "u8", line = "u8", mode = "u8" },
    -- 0x080C5F18.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            -- 0x080C62CC.
            me.collision:set_region(nil)
            me:destroy()
            return
        end
        if while_dimmed(me) then
            me:update_sprite_while_dimmed()
        elseif not battle.dimmed() then
            me:update_sprite()
        end
    end,
}

export type Throw = {
    -- Its variant (0 to 5) and its line (0 its column, 1 its row).
    variant: number,
    line: number,
    -- The panel it lands on.
    target_x: number,
    target_y: number,
    -- The damage word.
    damage: number,
}

-- 0x080C62D8: a capsule of `thrower`'s side and facing from its panel, with `element`, running while dimmed (its
-- third parameter 2). (Its position is the spawner's registers, which its init overwrites.)
function capsule.throw(thrower: Object, element: number, throw: Throw): Object?
    local o = battle.spawn(capsule.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.variant, s.line, s.mode = throw.variant, throw.line, 2
        o.future_panel_x, o.future_panel_y = throw.target_x, throw.target_y
        o.panel_x, o.panel_y = thrower.panel_x, thrower.panel_y
        o.element = element
        o:set_damage_word(throw.damage)
        o.alliance, o.flip = thrower.alliance, thrower.flip
        o.run_while_dimmed = true
    end
    return o
end

return capsule
