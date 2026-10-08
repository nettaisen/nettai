--!strict
-- A strike from the air (EXE5's attack object #0x1C, 0x080C7EA0; EXE6 keeps its code, `sub_80C7C18`, which no EXE6
-- content spawns): a burst of dust (sprite 10-0F) on a panel, up to 15 pixels across and 7 in depth from its center
-- by two draws, that hits navis there on the tick it appears and goes at its first update (it sets a timer of 10,
-- and its update ends it while the timer hasn't run out). Its hit modifier is its spawner's (by its first parameter:
-- 1 for 0, 0 otherwise; GyroMan's shots from the air, ./buster, 0).

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local strike = {}

export type State = {
    -- Its hit modifier.
    hit_mod: number,
}

local SPRITE = asset.sprite("air-strike")
local TICKS = 10

-- 0x080C7EC4: on its panel, on the floor, off its center by two draws; its dust (animation 0, its shadow), facing its
-- side's way; an attack on navis with the plain spark, on its panel at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    local dx = bit32.band(battle.rng_positive(), 0xF)
    local dy = bit32.band(battle.rng_positive(), 7)
    me.pos = Vec3.new(int.i32(p.x + dx * 0x1_0000), int.i32(p.y + dy * 0x1_0000), 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, (me.state :: State).hit_mod)
    me.collision:set_hit_effect(sparks.plain)
    me.collision:present()
    me.timer = TICKS
    me:set_lifecycle("update")
end

-- 0x080C7F54: its hits resolved; gone at the battle's end, on a hit, and (unless the battle is dimmed) while its
-- timer hasn't run out: at once.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local gone = battle.over() or c.hit_flags ~= 0
    if not gone and not battle.dimmed() then
        local left = me.timer - 1
        me.timer = int.u16(left)
        gone = left >= 0
    end
    if gone then
        c:set_region(nil)
        -- (The state byte alone.)
        me.lifecycle = "destroy"
        return
    end
    c:present()
end

strike.kind = new.kind {
    id = "gyroman/strike",
    pool = "attack",
    state = { hit_mod = "u8" },
    -- 0x080C7EA0.
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
            -- 0x080C7F8C.
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080C7F94: a strike of `owner`'s side and facing on panel (x, y), with the element, its hit modifier and the
-- damage word.
function strike.spawn(owner: Object, x: number, y: number, element: number, hit_mod: number, damage: number): Object?
    local o = battle.spawn(strike.kind, Vec3.zero)
    if o then
        (o.state :: State).hit_mod = hit_mod
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

return strike
