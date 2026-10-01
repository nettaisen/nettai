--!strict
-- ElmntMan's lightning bolt (the original's attack object #0xB8,
-- `sub_80DC3F8`): it
-- strikes its panel, hitting it once (a hit region that runs while
-- dimmed) and breaking it (cracking it if it holds something), and shows
-- for 16 ticks. His Elec strikes the column three ahead of him, a row at
-- a time. It runs while dimmed. The fraction of its Z is its spawner's
-- register garbage (the object loop's count of the objects of the one
-- before it, times 4), which its init keeps (compat's
-- `scratch_z_fraction`). See docs/engine/chips.md §3.6.7.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local bolt = {}

local SPRITE = asset.sprite("beast-over-burst")
local SOUND_BEAST_OVER_BURST = asset.sound("beast-over-burst")

-- `sub_80DC41C`: on its panel, on the ground (Z's whole part cleared).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80DC4DE`: the hit, running while dimmed (`sub_80C53A6`).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.elec,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80DC484`: strike, then show for 16 ticks.
local function strike(me: Object)
    if me.phase == 0 then
        me.phase = 4
        hit(me)
        field.break_panel(me.panel_x, me.panel_y)
        battle.play_sound(SOUND_BEAST_OVER_BURST)
        me.timer = 0x10
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DC45E`: gone once the battle is over.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if me.action ~= 0 then
        error(string.format("ElmntMan's bolt action %#x reads past its table (off_80DC480)", me.action))
    end
    strike(me)
end

bolt.kind = define.kind {
    id = "elmntman/bolt",
    pool = "attack",
    -- `sub_80DC3F8`.
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

-- `sub_80DC4B8`: a bolt on (x, y) with `element` and the damage word.
function bolt.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(bolt.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return bolt
