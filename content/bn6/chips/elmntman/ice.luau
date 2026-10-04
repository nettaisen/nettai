--!strict
-- ElmntMan's ice (the original's attack object #0x8E, `sub_80D6D80`): a
-- block of ice on a panel, which it turns to ice, hitting what is there
-- (status 0x50) for its ticks, its region gone once it has hit something.
-- His Aqua puts three in the column ahead of him (the original's Param3 1:
-- it acts and steps its sprite while dimmed); his own AI outside a dimming
-- (Param3 0: it waits one out, its collision off). See
-- docs/engine/chips.md §3.6.7.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local ice = {}

export type State = {
    -- How long it stands (Param2).
    ticks: number,
    -- It acts, and animates, while the battle is dimmed (Param3).
    while_dimmed: boolean,
}

local SPRITE = asset.sprite("elmnt-ice")

-- Its height, in whole pixels (a halfword store over Z's whole part).
local HEIGHT = 8
-- Its hit: its modifier, and the status it gives (freezing).
local HIT_MOD, STATUS = 3, statuses.freeze_150

-- `sub_80D6DB0`: on its panel at its height; collision on, its region
-- off until it acts.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), HEIGHT * 0x10000))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 1
    me.anim_loaded = 1
    sprite:set_animation(1)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c.status_base = STATUS
    c:set_hit_effect(sparks.aqua)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

-- `sub_80D6E70`: turn its panel to ice and hit it for its ticks.
local function stand(me: Object)
    local c = me.collision
    if me.phase == 0 then
        me.phase = 4
        c:set_region(regions.single)
        field.set_type(me.panel_x, me.panel_y, "ice")
        me.timer = (me.state :: State).ticks
        return
    end
    if c.hit_flags ~= 0 then
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

-- `sub_80D6E1E`: resolve the hits (its region goes once it hit), then act
-- and register again; outside a dimming unless it runs in one.
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
        c.hit_flags = 0
    end
    if not (me.state :: State).while_dimmed and battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("ElmntMan's ice action %#x reads past its table (off_80D6E6C)", me.action))
    end
    stand(me)
    c:present()
end

ice.kind = define.kind {
    id = "elmntman/ice",
    pool = "attack",
    state = { ticks = "u8", while_dimmed = "bool" },
    -- `sub_80D6D80`.
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

-- `sub_80D6EB0`: ice on (x, y) with `element` and the damage word, for
-- `ticks`, acting while dimmed or not. `z` is the spawner's register that
-- becomes its Z, whose fraction the ice keeps.
function ice.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    ticks: number,
    while_dimmed: boolean,
    damage: number,
    z: number
): Object?
    local o = battle.spawn(ice.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.ticks = ticks
        s.while_dimmed = while_dimmed
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return ice
