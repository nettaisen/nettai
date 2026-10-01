--!strict
-- A thunderbolt (the original's attack object #0x64, `sub_80D0D7C`): it
-- hangs out of sight (Z 255) over its panel for 20 ticks, the panel
-- highlighted 4 ticks out of 8, then strikes: 30 ticks on the ground with
-- its region on for the first 20 (a hit turns it off). What it is spawned
-- with is a `Thunderbolt` (types.d.luau; the original's Param1): the navi
-- chip's (1 and 2) act and step their sprite while dimmed, and the second
-- strike's (2) also hit the eight panels around them once (status 0x12);
-- one that doesn't (0: ElecMan's own AI's) cracks its panel and waits a
-- dimming out (collision off). See docs/engine/chips.md §3.6.16.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")
local thunder = {}

export type State = {
    -- It acts, and animates, while the battle is dimmed (Param1 1 and 2);
    -- else it cracks its panel as it strikes (Param1 0).
    while_dimmed: boolean,
    -- It also hits the eight panels around it (Param1 2).
    spreads: boolean,
}

local SPRITE = asset.sprite("beast-over-burst")
local SOUND_BEAST_OVER_BURST = asset.sound("beast-over-burst")
-- The hit modifier of its own hit and of the hits around it, and their
-- status.
local HIT_MOD, AROUND_HIT_MOD, AROUND_STATUS = 3, 1, statuses.paralyze_150

-- Its actions.
local WARN, STRIKE = 0, 4

-- The panels around it Param1 2 hits (`byte_80D0F74`): {dx, dy}.
local AROUND = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } }

-- Z's whole part (a halfword store), its fraction kept.
local function set_height(me: Object, pixels: number)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(pixels, 16)))
end

-- `sub_80D0DAC`: over its panel, out of sight; collision on, region off.
local function init(me: Object)
    me:set_coordinates_from_panel()
    set_height(me, 0xFF)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(sparks.elec)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

-- `sub_80D0E64`: 20 ticks of warning, the panel highlighted 4 ticks out
-- of 8.
local function warn(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 0x14
        me.timer2 = 0
        return
    end
    local t = me.timer2 + 1
    me.timer2 = t
    if bit32.band(t, 4) == 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(STRIKE)
    end
end

-- `sub_80D0F32`: a hit on each panel around it, running while dimmed
-- (`sub_80C53A6`).
local function hit_around(me: Object)
    for _, o in AROUND do
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + o[1],
            panel_y = me.panel_y + o[2],
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.elec,
            target = collision.hits_navis,
            self_type = collision.thrown,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = AROUND_HIT_MOD,
            status = AROUND_STATUS,
        })
        if box then
            box.run_while_dimmed = true
        end
    end
end

-- `sub_80D0E9E`: the strike: 30 ticks on the ground, the region on for
-- 20.
local function strike(me: Object)
    local c = me.collision
    if me.phase == 0 then
        me.phase = 4
        c:set_region(regions.single)
        set_height(me, 0)
        -- The loaded animation forgotten: the sprite restarts it.
        me.anim_loaded = 0xFF
        battle.play_sound(SOUND_BEAST_OVER_BURST)
        me.timer = 0x1E
        me.timer2 = 0x14
        local s = me.state :: State
        if not s.while_dimmed then
            field.crack(me.panel_x, me.panel_y)
        end
        if s.spreads then
            hit_around(me)
        end
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local lit = me.timer2 - 1
    me.timer2 = lit
    if lit <= 0 then
        c:set_region(nil)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    c:set_region(nil)
    me:set_lifecycle("destroy")
end

local ACTIONS = { [WARN] = warn, [STRIKE] = strike }

-- `sub_80D0E14`: resolve the hits (its region goes once it hit); act and
-- register again, unless it waits a dimming out.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if not (me.state :: State).while_dimmed and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("thunderbolt action %#x reads past its table (off_80D0E5C)", me.action))
    end
    action(me)
    c:present()
end

thunder.kind = define.kind {
    id = "elecman/thunder",
    pool = "attack",
    state = { while_dimmed = "bool", spreads = "bool" },
    -- `sub_80D0D7C`.
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

-- `sub_80D0F0C`: a thunderbolt on (x, y) from `owner` with `element`, the
-- damage word and what it is. `z` is the spawner's register that becomes
-- its Z, whose fraction it keeps.
function thunder.spawn(owner: Object, x: number, y: number, element: number, what: Thunderbolt, damage: number, z: number): Object?
    local o = battle.spawn(thunder.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.while_dimmed = what.while_dimmed == true
        s.spreads = what.spreads == true
    end
    return o
end

return thunder
