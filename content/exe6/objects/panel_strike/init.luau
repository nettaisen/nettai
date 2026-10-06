--!strict
-- A panel strike (the original's attack object #0x09, `sub_80C5DDC`): it
-- highlights its panel 4 ticks out of 8 for its warning, then shows a
-- burst and hits the panel once; it goes when the burst's animation ends.
-- Bass's shots (chips/bass) and MachGun's (chips/machgun) are these.
--
-- What it is spawned with is a `PanelStrike` (types.d.luau; the
-- original's parameters): whether the burst cracks an uncracked panel
-- (Param1), the ticks before it (Param2), its hit modifier (Param3: 0
-- gives 3, 1 gives 1, anything else 0), and whether the burst comes only
-- on a solid panel (Param4; else it just goes). One that runs while dimmed
-- (`sub_80C5F4A`) outlasts the battle's end. See docs/engine/chips.md
-- §3.6.33.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local panel_strike = {}

export type State = {
    cracks: boolean,
    warning: number,
    hit_mod: number,
    solid_only: boolean,
}

local SPRITE = asset.sprite("panel-strike")
local SOUND_COL_ARMY_2 = asset.sound("col-army-2")

-- `sub_80C5E00`: on its panel, its collision on with its region off.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local s = me.state :: State
    me.timer = s.warning
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, s.hit_mod)
    local c = me.collision
    c:set_hit_effect(sparks.plain)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
end

-- `sub_80C5E84`: the highlight, then the burst and its hit (and a crack),
-- or, where it wants a solid panel and there is none, nothing.
local function wait(me: Object)
    if bit32.band(me.timer, 4) == 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local c = me.collision
    local s = me.state :: State
    if s.solid_only and not field.solid(me.panel_x, me.panel_y) then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    c:set_region(regions.single)
    if s.cracks then
        -- sub_8109794: crack it unless it is cracked already.
        if bit32.band(field.flags(me.panel_x, me.panel_y), 0x40) == 0 then
            field.crack(me.panel_x, me.panel_y)
        end
    end
    battle.play_sound(SOUND_COL_ARMY_2)
    me:set_action(4)
end

-- `sub_80C5F0C`: the region off; gone when the burst ends.
local function burst(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.collision:set_region(nil)
    end
    if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = burst }

-- `sub_80C5E44`: resolve the hits, act, register again; gone once the
-- battle is over (unless it runs while dimmed).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if not me.run_while_dimmed and battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("panel strike action %#x reads past its table (off_80C5E7C)", me.action))
    end
    action(me)
    c:present()
end

panel_strike.kind = new.kind {
    id = "panel-strike",
    pool = "attack",
    state = { cracks = "bool", warning = "u8", hit_mod = "u8", solid_only = "bool" },
    -- `sub_80C5DDC`.
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

-- `sub_80C5F2C` (`while_dimmed`: `sub_80C5F4A`): a strike on (x, y) with
-- `element`, the damage word and what it is, on `owner`'s side. `z` is the
-- spawner's register that becomes its Z (its init keeps it).
function panel_strike.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    what: PanelStrike,
    damage: number,
    z: number,
    while_dimmed: boolean
): Object?
    local o = battle.spawn(panel_strike.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.cracks = what.cracks == true
        s.warning = what.warning
        s.hit_mod = what.hit_mod
        s.solid_only = what.solid_only == true
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        if while_dimmed then
            o.run_while_dimmed = true
        end
    end
    return o
end

return panel_strike
