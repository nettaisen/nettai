--!strict
-- A flame (the original's attack object #0x26, `sub_80C8C74`): HeatMan's
-- breath, and what BurnSqr's burner sets (chips/burnsqr). It flares up on
-- its panel (5 ticks), burns hitting it (its ticks less 5, its region gone
-- once it has hit something), and dies down (2 ticks). A thrown one counts
-- as a different attacker. HeatMan's act and step their sprite while
-- dimmed; others wait a dimming out, collision on. What it is spawned with
-- is a `HeatFlame` (types.d.luau). See docs/engine/chips.md §3.6.15.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local flame = {}

export type State = {
    -- Ticks it burns, flaring up included (Param2).
    ticks: number,
    -- It acts, and animates, while the battle is dimmed (Param1 4:
    -- HeatMan's).
    while_dimmed: boolean,
    -- It counts as a thrown attack (Param3: collision type `thrown`, else
    -- `attack`).
    thrown: boolean,
}

local SPRITE = asset.sprite("heat-flame")
local SOUND = asset.sound("form-change")
local HIT_MOD = 3

-- Its actions.
local FLARE, BURN, DIE_DOWN = 0, 4, 8

-- `sub_80C8CA4`: on its panel on the ground, collision on.
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
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(if s.thrown then collision.thrown else collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.fire)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C8D64`: flaring up, 5 ticks (sound 0xF7).
local function flare(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND)
        me.timer = 5
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(BURN)
    end
end

-- `sub_80C8D8E`: burning, its ticks less 5; then its region goes.
local function burn(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.timer = (me.state :: State).ticks - 5
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
        me.timer = 2
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
-- (outside a dimming unless it is HeatMan's), register again.
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
    id = "heatman/flame",
    pool = "attack",
    state = { ticks = "u8", while_dimmed = "bool", thrown = "bool" },
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

-- `sub_80C8DE0`: a flame on (x, y) from `owner` with `element`, the damage
-- word and what it is. `z` is the spawner's register that becomes its Z,
-- whose fraction it keeps.
function flame.spawn(owner: Object, x: number, y: number, element: number, what: HeatFlame, damage: number, z: number): Object?
    local o = battle.spawn(flame.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.ticks = what.ticks
        s.while_dimmed = what.while_dimmed == true
        s.thrown = what.thrown == true
    end
    return o
end

return flame
