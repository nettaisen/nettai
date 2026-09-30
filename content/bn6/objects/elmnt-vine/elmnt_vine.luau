--!strict
-- ElmntMan's vine (attack object #0xB9, `sub_80DC4FC`): a vine shooting up
-- from its panel (his own sprite's animation 0x12), hitting it once (a
-- hit region that runs while dimmed), and showing for 30 ticks. His Wood
-- grows them along his row, a panel at a time. It runs while dimmed. See
-- docs/engine/chips.md §3.6.7.

local elmnt_vine = {}

local SPRITE = "08-10"
local ANIM = 0x12

-- `sub_80DC5B4`: a vine on (x, y) with `element` and the damage word;
-- `params` and `z` are the spawner's registers (its count; the row it
-- checked minus 1, which becomes its Z, of which it keeps the fraction).
function elmnt_vine.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("elmnt-vine", Vec3.new(y, element, z), params)
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

-- `sub_80DC520`: on its panel, on the ground (Z's whole part cleared).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80DC5DA`: the hit, running while dimmed (`sub_80C53A6`).
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = 1,
        hit_effect = 4,
        target = 5,
        self_type = 0x0A,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 1,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `sub_80DC588`: grow, then show for 30 ticks.
local function grow(me: Object)
    if me.phase == 0 then
        me.phase = 4
        hit(me)
        battle.play_sound(0x181)
        me.timer = 0x1E
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `loc_80DC562`: gone once the battle is over.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if me.action ~= 0 then
        error(string.format("ElmntMan's vine action %#x reads past its table (off_80DC584)", me.action))
    end
    grow(me)
end

-- `sub_80DC4FC`.
function elmnt_vine.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite_while_dimmed()
end

return elmnt_vine
