--!strict
-- A flame (the original's attack object #0x26, `sub_80C8C74`): it flares up
-- on its panel (5 ticks), burns hitting it (its time less those 5, its
-- region gone once it has hit something), and dies down (2 ticks).
-- HeatMan's (the HeatMan chips' navi, ./navi) acts and steps its
-- sprite while dimmed; his link navi's HeatPres flames (navis/heatman)
-- wait a dimming out, collision on. See docs/engine/chips.md §3.6.15.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local flame = {}

-- What a flame is spawned with (the original's three parameters).
type Spec = HeatFlame
export type State = Spec

local SPRITE = asset.sprite("bn6:heat-flame")
local SOUND = asset.sound("bn6:form-change")

-- Its actions.
local FLARE, BURN, DIE_DOWN = 0x0, 0x4, 0x8
local FLARE_TICKS, DIE_DOWN_TICKS = 5, 2

-- `sub_80C8CA4`: on its panel on the ground (its Z's fraction kept),
-- collision on.
local function init(me: Object)
    local s = me.state :: State
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
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(s.is, collision.hits_navis, 3)
    me.collision:set_hit_effect(sparks.fire)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C8D64`: flaring up, 5 ticks.
local function flare(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND)
        me.timer = FLARE_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(BURN)
    end
end

-- `sub_80C8D8E`: burning for its time less the flare's; then its region
-- goes.
local function burn(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.timer = (me.state :: State).ticks - FLARE_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision:set_region(nil)
    me:set_action(DIE_DOWN)
end

-- `sub_80C8DBC`: dying down, 2 ticks.
local function die_down(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 2
        me.timer = DIE_DOWN_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [FLARE] = flare, [BURN] = burn, [DIE_DOWN] = die_down }

-- `sub_80C8D10`: resolve the hits (its region goes once it hit), act
-- (outside a dimming unless it acts while dimmed), register again.
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
    if (me.state :: State).while_dimmed or not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("flame action %#x reads past its table (off_80C8D58)", me.action))
        end
        action(me)
    end
    c:present()
end

flame.kind = define.kind {
    id = "bn6:heatman/flame",
    pool = "attack",
    state = { while_dimmed = "bool", ticks = "u8", is = "collision" },
    -- `sub_80C8C74`.
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

-- `sub_80C8DE0`: a flame on (x, y) from `owner`, with `element` and the
-- damage word `damage`. It runs while dimmed. `pos` is where the spawner's
-- registers leave it (its panel's row, the element and its Z), of which
-- its init keeps the Z's fraction.
function flame.spawn(owner: Object, x: number, y: number, element: number, spec: Spec, damage: number, pos: Vec3): Object?
    local o = battle.spawn(flame.kind, pos)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.while_dimmed, s.ticks, s.is = spec.while_dimmed, spec.ticks, spec.is
    end
    return o
end

return flame
