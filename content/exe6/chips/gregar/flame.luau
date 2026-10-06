--!strict
-- A flame of Gregar's breath (the original's attack object #0xCD,
-- `sub_80DF0A4`; its routines are in the US ROMs too, but only the
-- Japanese ROMs' Gregar chip spawns it; docs/engine/beast-chips.md): a
-- burning pillar on a panel, eight pixels up, that hits what stands on
-- it once and burns on for its ticks, acting and stepping while dimmed.
-- Gregar's summon breathes seven (chips/gregar/summon).

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local flame = {}

export type State = {
    -- How long it burns (its Param1).
    ticks: number,
}

local SPRITE = asset.sprite("element-pillar-flames")
-- Its palette: the flames' red.
local PALETTE = 4
-- Eight pixels up.
local Z = 0x8_0000
-- Its hit modifier.
local HIT_MOD = 3

-- `sub_80DF132`: resolve its hits; a hit closes its region (it burns on
-- without hitting); at the end of its time it goes.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        c:present()
        return
    end
    c:set_region(nil)
    me.lifecycle = "destroy"
end

-- `sub_80DF0C8`: on its panel, 8 pixels up, drawn without a shadow, the
-- flames' palette; an attack on what stands there, a heavy spark. Without
-- a collision slot it is freed.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, Z)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = PALETTE
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.heavy)
    me.collision:present()
    me.timer = (me.state :: State).ticks
    me:set_lifecycle("update")
end

flame.kind = new.kind {
    id = "gregar/flame",
    pool = "attack",
    state = { ticks = "u8" },
    -- `sub_80DF0A4`: by state (its end `object_genericDestroy`), then the
    -- sprite, stepping while dimmed.
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

-- `sub_80DF160` (an unnamed spawner after `sub_80DF132`): a flame of
-- `owner`'s on (x, y) with `element` and the damage word, burning `ticks`
-- ticks, acting while dimmed (flag 0x10). (Spawned at the registers: the
-- row, the element and `z`, the summon's bonus.)
function flame.spawn(owner: Object, x: number, y: number, element: number, damage: number, ticks: number, z: number): Object?
    local o = battle.spawn(flame.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        ;(o.state :: State).ticks = ticks
    end
    return o
end

return flame
