--!strict
-- GyroMan's bomb (EXE5's attack object #0x1B, 0x080C7C6C; EXE6's `sub_80C793C` is like it, though no EXE6 content drops
-- it): dropped on a panel from high up. Its variant (its first parameter) gives its height, its fall and its wait:
-- 0 (a virus's) 64 pixels up, falling 4 a tick, after a wait by its row; 1 (GyroMan's chip's) and 2 40 up, falling
-- 6, at once, and running while dimmed. Hidden while it waits, it lights its panel as it falls; on a panel that
-- isn't solid it lands with nothing, else it bursts there (an explosion, a sound) and hits for 4 ticks: thrown,
-- hitting navis, hit modifier 3, no spark.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")

export type State = {
    -- Its variant (its first parameter: 0, 1 or 2).
    variant: number,
}

local bomb = {}

local SPRITE = asset.sprite("gyro-bomb")
local SOUND_BURST = asset.sound("hit-bomb-1")
-- By the variant (0x080C7D1C and 0x080C7D0C): its height, and how far it falls a tick.
local HEIGHTS: { [number]: number } = { [0] = 0x40_0000, 0x28_0000, 0x28_0000 }
local FALLS: { [number]: number } = { [0] = 0x4_0000, 0x6_0000, 0x6_0000 }
-- Variant 0's wait by its row (0x080C7DC8: rows 1 to 3).
local DELAYS: { [number]: number } = { 1, 1, 1 }
local SOLID = 0x10
-- How long it hits.
local BURST = 4

local function by_variant(t: { [number]: number }, variant: number, what: string): number
    local v = t[variant]
    if v == nil then
        error(string.format("GyroMan's bomb: variant %d reads past its %s table", variant, what))
    end
    return v
end

-- Its panel lit, if solid (`object_highlightPanel`). Drawn only.
local function light(me: Object)
    if field.check(me.panel_x, me.panel_y, SOLID, 0) then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- Action 0 (0x080C7D74): hidden, it waits (variant 0 by its row; else not at all), then shows; its panel lit after
-- the first tick.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local ticks = 0
        if (me.state :: State).variant == 0 then
            ticks = DELAYS[me.panel_y] or error(string.format("GyroMan's bomb: row %d reads past 0x080C7DC8", me.panel_y))
        end
        me.timer = ticks
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me.visible = true
        me:set_action(4)
    end
    light(me)
end

-- Action 4 (0x080C7DCC): it falls (its panel lit) to the ground; there, on a solid panel it bursts, else it goes.
local function fall(me: Object)
    local p = me.pos
    if p.z > 0 then
        me.pos = Vec3.new(p.x, p.y, int.i32(p.z - me.vel.z))
        light(me)
    elseif field.check(me.panel_x, me.panel_y, SOLID, 0) then
        -- (Only the action byte: its phase stays 0.)
        me.action = 8
    else
        me.collision:set_region(nil)
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

-- Action 8 (0x080C7E1C): it bursts: its panel hit, hidden, a sound, an explosion where it is; its region cleared on
-- a hit; 4 ticks on it goes.
local function burst(me: Object)
    local c = me.collision
    if me.phase == 0 then
        me.phase = 4
        c:set_region(regions.single)
        me.timer = BURST
        me.visible = false
        battle.play_sound(SOUND_BURST)
        battle.effect(me.pos, effects.explosion)
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = fall, [8] = burst }

-- 0x080C7D28: the hits against it resolve and spark; at the battle's end it goes; else its action (unless the
-- battle is dimmed and it doesn't run while dimmed).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        -- (The state byte alone: its action and phase stay.)
        me.lifecycle = "destroy"
        return
    end
    if me.run_while_dimmed or not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("GyroMan's bomb: action %#x reads past its table (0x080C7D68)", me.action))
        end
        action(me)
    end
    c:present()
end

-- 0x080C7C90: its sprite (animation 0, its shadow), hidden; its height and fall by the variant; on its panel; its
-- collision (thrown, hitting navis, hit modifier 3, no spark), no region yet.
local function init(me: Object)
    local variant = (me.state :: State).variant
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = false
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, by_variant(HEIGHTS, variant, "height"))
    local v = me.vel
    me.vel = Vec3.new(v.x, v.y, by_variant(FALLS, variant, "fall"))
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    local c = me.collision
    c:set_hit_effect(nil)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

bomb.kind = define.kind {
    id = "gyroman/bomb",
    pool = "attack",
    state = { variant = "u8" },
    -- 0x080C7C6C.
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
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080C7E72: a bomb of `owner`'s side on (x, y) with `element`, of `variant`, with the damage word; variant 1 runs
-- while dimmed. (Its position is the spawner's registers, which its init overwrites.)
function bomb.drop(owner: Object, x: number, y: number, element: number, variant: number, damage: number): Object?
    local o = battle.spawn(bomb.kind, Vec3.zero)
    if o then
        (o.state :: State).variant = variant
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        if variant == 1 then
            o.run_while_dimmed = true
        end
    end
    return o
end

return bomb
