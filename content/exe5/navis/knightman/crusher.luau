--!strict
-- KCrusher's iron ball (EXE5's attack object #0x15, 0x080C724C; EXE5's own), which KnightMan throws (./chip): his
-- ball (sprite 10-47, animation 20) leaves 40 pixels ahead of him and 42 up in a puff of dust, flies along his row at
-- its speed (its first parameter: KCrusher's 5 pixels a tick), and hits each panel it comes over (breaking, hitting
-- navis, hit modifier 3, the breaking spark), lighting it; a hit takes its region off until it is over the next
-- panel. It goes once it is 150 pixels from the field's middle. It waits while dimmed, and flies on when the
-- battle is over.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local crusher = {}

export type State = {
    -- Its speed, in sixteenths of a pixel a tick (its first parameter).
    speed: number,
}

local SPRITE = asset.sprite("knightman-ball")
local ANIM = 20
local HIT_MOD = 3
-- How far from the field's middle it flies, in pixels.
local REACH = 150

-- 0x080C7316: on along its row; past its reach, gone.
local function fly(me: Object)
    local p = me.pos
    local x = int.asr(p.x, 16)
    if x + REACH < 0 or x + REACH > 2 * REACH then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- 0x080C72E4: the hits against it resolve (a hit clears its region); it flies; over a new panel it hits again; its
-- panel lit.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    fly(me)
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.single)
    end
    field.highlight(me.panel_x, me.panel_y)
    c:present()
end

-- 0x080C7270: its sprite (its first part drawn with it), shown, facing its way; over the panel under it, flying
-- ahead; the dust where it starts; its collision; its first update.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- (`sprite_noShadow`: the first part is drawn with the sprite.)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, 0xFF
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(me.facing * s.speed * 0x1000, me.vel.y, me.vel.z)
    battle.effect(me.pos, effects.dust)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.breaking, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.breaking)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

crusher.kind = define.kind {
    id = "knightman/crusher",
    pool = "attack",
    state = { speed = "u8" },
    -- 0x080C724C
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080C7342: a ball at `pos` of `owner`'s side and facing, null, with `speed` and the damage word `damage`.
function crusher.throw(owner: Object, pos: Vec3, speed: number, damage: number): Object?
    local o = battle.spawn(crusher.kind, pos)
    if o then
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        o.element = 0
        local s = o.state :: State
        s.speed = speed
    end
    return o
end

return crusher
