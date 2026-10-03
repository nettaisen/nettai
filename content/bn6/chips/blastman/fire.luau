--!strict
-- A fire blast (the original's attack object #0x21, `sub_80C8388`): a
-- flame (BlastMan's sprite) that crosses the field in a straight line from
-- where it spawns, its body a hit region, highlighting the panel it is
-- over, gone when it hits something or its time is up. What it is spawned
-- with is a `FireBlast` (types.d.luau): its speed (the original's Param1
-- picks it from `byte_80C8454`: 8, 10, 10 and 12 pixels a tick for
-- BlastMan's own AI's, 6 for the navi chip's, which acts and steps its
-- sprite while dimmed; the others wait a dimming out) and its direction
-- (Param2). See docs/engine/chips.md §3.6.14.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local fire = {}

export type State = {
    -- Its speed, 16.16 a tick.
    speed: number,
    -- Which way it goes (Param2: 0 left, 1 right, 2 up, 3 down).
    direction: FireBlastDirection,
    -- It acts, and animates, while the battle is dimmed (the navi chip's).
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("bn6:blast-fire")
local HIT_MOD = 3

-- By direction: its step (`byte_80C846C`), how far it goes (16.16,
-- `byte_80C8488`), its animation (`dword_80C8480`) and flip
-- (`dword_80C8478`).
local WAYS: { [FireBlastDirection]: { step: { number }, distance: number, anim: number, flip: number } } = {
    left = { step = { -1, 0 }, distance = 0x118_0000, anim = 0x11, flip = 1 },
    right = { step = { 1, 0 }, distance = 0x118_0000, anim = 0x11, flip = 0 },
    up = { step = { 0, -1 }, distance = 0x78_0000, anim = 0x12, flip = 2 },
    down = { step = { 0, 1 }, distance = 0x78_0000, anim = 0x12, flip = 0 },
}

-- `sub_80C83B8`: on its panel, on the ground, moving; collision on.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local s = me.state :: State
    local speed, way = s.speed, WAYS[s.direction]
    me.vel = Vec3.new(int.mul32(way.step[1], speed), int.mul32(way.step[2], speed), me.vel.z)
    -- A BIOS division, truncating.
    me.timer = int.tdiv(way.distance, speed)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = way.anim
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(way.flip)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.fire)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C84E0`: move, highlighting the panel it is over; gone when its
-- time is up.
local function fly(me: Object)
    local p, v = me.pos, me.vel
    me.pos = Vec3.new(p.x + v.x, p.y + v.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    field.highlight(me.panel_x, me.panel_y)
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C8498`: resolve the hits; a hit (or the battle's end) ends it;
-- act (outside a dimming unless it is the navi chip's), register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or c.hit_flags ~= 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if (me.state :: State).while_dimmed or not battle.dimmed() then
        if me.action ~= 0 then
            error(string.format("fire blast action %#x reads past its table (off_80C84DC)", me.action))
        end
        fly(me)
    end
    c:present()
end

fire.kind = define.kind {
    id = "bn6:blastman/fire",
    pool = "attack",
    state = {
        speed = "u32",
        direction = { "left",
        "right",
        "up",
        "down" },
        while_dimmed = "bool",
    },
    -- `sub_80C8388`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80C8514`: a flame on (x, y) from `owner` with `element`, the damage
-- word and what it is. (Its position is the spawner's registers, all of
-- which its init overwrites.)
function fire.spawn(owner: Object, x: number, y: number, element: number, what: FireBlast, damage: number): Object?
    local o = battle.spawn(fire.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.speed = what.speed
        s.direction = what.direction
        s.while_dimmed = what.while_dimmed == true
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return fire
