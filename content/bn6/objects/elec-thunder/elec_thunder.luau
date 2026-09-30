--!strict
-- A thunderbolt (attack object #0x64, `sub_80D0D7C`): it hangs out of
-- sight (Z 255) over its panel for 20 ticks, the panel highlighted 4 ticks
-- out of 8, then strikes: 30 ticks on the ground with its region on for
-- the first 20 (a hit turns it off). Param1 0 (ElecMan's navi AI's) cracks
-- the panel and waits a dimming out (collision off); 2 also hits the eight
-- panels around it once (status 0x12); 1 and 2 (ElecMan's) act and step
-- their sprite while dimmed. See docs/engine/chips.md §3.6.16.

local elec_thunder = {}

local SPRITE = "14-14"

-- The panels around it Param1 2 hits (`byte_80D0F74`): {dx, dy}.
local AROUND = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } }

-- `sub_80D0F0C`: a thunderbolt on (x, y) from `owner` with `element`, the
-- damage word and `params` {Param1}. `z` is the spawner's register that
-- becomes its Z, whose fraction it keeps.
function elec_thunder.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("elec-thunder", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

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
    me:setup_collision(0xA, 5, 3)
    local c = me.collision
    c.hit_effect = 3
    c.region = 0
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
        me:set_action(4)
    end
end

-- `sub_80D0F32`: a hit on each panel around it, running while dimmed
-- (`sub_80C53A6`; hit modifier 1, status 0x12).
local function hit_around(me: Object)
    for _, o in AROUND do
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + o[1],
            panel_y = me.panel_y + o[2],
            element = me.element,
            z = 0,
            region = 1,
            hit_effect = 3,
            target = 5,
            self_type = 0xA,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 1,
            status = 0x12,
        })
        if box then
            box.run_while_dimmed = true
        end
    end
end

-- `sub_80D0E9E`: the strike: 30 ticks on the ground (sound 0x12E), the
-- region on for 20.
local function strike(me: Object)
    local c = me.collision
    if me.phase == 0 then
        me.phase = 4
        c.region = 1
        set_height(me, 0)
        -- The loaded animation forgotten: the sprite restarts it.
        me.anim_loaded = 0xFF
        battle.play_sound(0x12E)
        me.timer = 0x1E
        me.timer2 = 0x14
        if me:param(1) == 0 then
            field.crack(me.panel_x, me.panel_y)
        end
        if me:param(1) == 2 then
            hit_around(me)
        end
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    local lit = me.timer2 - 1
    me.timer2 = lit
    if lit <= 0 then
        c.region = 0
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    c.region = 0
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = warn, [4] = strike }

-- `sub_80D0E14`: resolve the hits (its region goes once it hit); act and
-- register again, unless it waits a dimming out (Param1 0).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    if me:param(1) == 0 and battle.dimmed() then
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("thunderbolt action %#x reads past its table (off_80D0E5C)", me.action))
    end
    action(me)
    c:present()
end

-- `sub_80D0D7C`.
function elec_thunder.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 0 then
        me:update_sprite_while_paused()
    else
        me:update_sprite_while_dimmed()
    end
end

return elec_thunder
