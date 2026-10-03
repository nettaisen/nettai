--!strict
-- KnightMan's wrecking ball (BN5's attack object #0x17, 0x080C7568; BN5's own: BN6's #0x17 is SpoutMan's geyser,
-- on the same skeleton), swinging around its owner: its angle starts at 0 and turns by its speed (its first
-- parameter) a tick, its position on a circle of its radius around its owner's, turned to the owner's side (x back,
-- toward its own side's end, at angle 0); after its turns (its second parameter) it goes. It hits each panel it
-- comes to (breaking, hitting navis, hit modifier 3, the breaking spark, its status from its third parameter),
-- lighting it; a hit clears its region until it moves on. Its variant (its fourth parameter) picks its sprite, its
-- height and its radius: 0, KnightMan's, his ball 16 pixels up at 30 pixels; 1 another sprite on the ground at 35.
-- (Its sprite is never stepped: the update that would, for its +0x60, never runs. Nor does the check of its +0x68,
-- which nothing sets, ever skip its relighting.)

local collision = require("../../rules/collision")
local math5 = require("../../rules/math")
local panels = require("@common/panels")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

export type State = {
    -- How far it turns a tick (its first parameter), the turns it has left (its second), its status (its third)
    -- and its variant (its fourth).
    speed: number,
    turns: number,
    status: Status?,
    variant: number,
    -- Its radius in pixels (+0x64).
    radius: number,
}

local ball = {}

-- By the variant: its sprite, its height and its radius.
type Look = { sprite: SpriteAsset, z: number, radius: number }
local LOOKS: { [number]: Look } = {
    [0] = { sprite = asset.sprite("bn5:knightman-ball"), z = 0x10_0000, radius = 0x1E },
    [1] = { sprite = asset.sprite("bn5:sprite-0c-05"), z = 0, radius = 0x23 },
}

local function look_of(me: Object): Look
    local v = (me.state :: State).variant
    return LOOKS[if v == 1 then 1 else 0]
end

-- 0x080C76C4: on its circle around its owner: x by the cosine, turned back toward its own side; y by the sine.
local function place(me: Object)
    local s = me.state :: State
    local owner = me.related1 :: Object
    local angle = bit32.band(me.timer, 0xFF)
    local back = -panels.side_direction(me.alliance)
    local cos, sin = math5.sine[angle + 0x40 + 1], math5.sine[angle + 1]
    local op = owner.pos
    me.pos = Vec3.new(
        int.i32(op.x + int.mul32(int.shl(int.mul32(cos, s.radius), 8), back)),
        int.i32(op.y + int.shl(int.mul32(sin, s.radius), 8)),
        me.pos.z
    )
end

-- 0x080C7656: it turns; past a full turn, one of its turns is spent, and with none left it goes (its region
-- cleared); else to its new place.
local function turn(me: Object)
    local s = me.state :: State
    local angle = me.timer + s.speed
    me.timer = int.u16(angle)
    if angle > 0xFF then
        me.timer = bit32.band(angle, 0xFF)
        local left = s.turns - 1
        s.turns = int.u8(left)
        if int.i8(left) <= 0 then
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
    end
    place(me)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- 0x080C761E: the hits against it resolve (a hit clears its region); it turns; on a new panel it hits there again;
-- its panel lit.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    turn(me)
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.single)
    end
    field.highlight(me.panel_x, me.panel_y)
    c:present()
end

-- 0x080C7590: its sprite (animation 20, not yet loaded; its first part drawn with it), shown, facing its way; at
-- angle 0 on its circle, which, its radius not yet set, is its owner's place; its height by its variant; its
-- collision; its radius; its first update.
local function init(me: Object)
    local s = me.state :: State
    local l = look_of(me)
    local sprite = me.sprite
    sprite:load(l.sprite)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0x14, 0xFF
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = 0
    place(me)
    me:set_panel_from_coordinates()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, l.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.breaking, collision.hits_navis, 3)
    local c = me.collision
    c:set_hit_effect(sparks.breaking)
    c:present()
    c.status_base = s.status
    s.radius = l.radius
    me:set_lifecycle("update")
    tick(me)
end

ball.kind = define.kind {
    id = "bn5:knightmn/ball",
    pool = "attack",
    state = { speed = "u8", turns = "u8", status = "status", variant = "u8", radius = "u32" },
    -- 0x080C7568.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

export type Swing = { speed: number, turns: number, status: Status?, variant: number }

-- 0x080C76AC (by way of 0x080C768A): a ball around `owner`, of its side, null, with the damage word, running while
-- dimmed. (Its position is the spawner's registers, which its init overwrites.)
function ball.swing(owner: Object, damage: number, swing: Swing): Object?
    local o = battle.spawn(ball.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.speed, s.turns, s.status, s.variant = swing.speed, swing.turns, swing.status, swing.variant
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.element = 0
        o.run_while_dimmed = true
    end
    return o
end

return ball
