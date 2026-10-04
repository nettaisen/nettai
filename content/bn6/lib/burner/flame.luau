--!strict
-- A flame (the original's attack object #0x04, `sub_80C53C0`): FireBrn's,
-- WideBrn's and HeatCross's charged shot's fire. It burns on its panel for
-- its ticks with its collision on; on its fourth tick, while it has spread
-- left, it lights the next panel forward (and the ones above and below it
-- too, the first time, if it is wide: WideBrn's), each new flame with one
-- spread less. When it goes out it cracks its panel if it cracks. Its look
-- picks its animation and where it starts: at its owner's hand (attach
-- point 0x12 of the owner's NameID, 40 pixels back) or on its panel.
--
-- The game gives a flame a pointer (ExtraVars+0) to a word that keeps it
-- burning while nonzero. FireBrn's is null, and reading address 0 outside
-- the BIOS gets the BIOS's last fetched opcode, never zero, so FireBrn's
-- flames burn their full time; the chain passes the pointer on.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../sparks")

local flame = {}

-- What a flame burns like (`Flame`, types.d.luau).
export type Spec = Flame

export type State = {
    -- Ticks left to burn (Param1), panels left to spread (Param2), and it
    -- cracks its panel as it goes out (Param3).
    ticks: number,
    spread: number,
    cracks: boolean,
    -- Its animation and where it starts (Param4's row of `byte_80C54DC`).
    anim: number,
    on_panel: boolean,
    -- It lights three panels at once (ExtraVars+4).
    wide: boolean,
}

local SPRITE = asset.sprite("bn6:flame")
-- The owner's attach point it starts at, and how far back of it.
local HAND, BACK = 0x12, 0x28
-- Ticks before it lights the next panel.
local SPREAD_AT = 4

-- `sub_80C5578`: a flame for `owner` on (x, y) with the damage word
-- `damage`, burning as `spec` says. Its position is the spawner's
-- registers (the panel Y, the element and 0) until its init places it.
function flame.spawn(owner: Object, x: number, y: number, element: number, damage: number, spec: Spec): Object?
    local o = battle.spawn(flame.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.ticks, s.spread, s.wide = spec.ticks, spec.spread, spec.wide or false
        s.cracks, s.anim, s.on_panel = spec.cracks or false, spec.anim or 0, spec.on_panel or false
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80C5590`: the next flame on (x, y), burning as long as this one has
-- left, with one spread less, and this one's owner.
local function spread_to(me: Object, x: number, y: number)
    local s = me.state :: State
    local damage = me.damage + me.stamina * 0x10000
    local spec: Spec = { ticks = s.ticks, spread = s.spread, cracks = s.cracks, anim = s.anim, on_panel = s.on_panel }
    local o = flame.spawn(me, x, y, me.element, damage, spec)
    if o then
        o.related1 = me.related1
    end
end

-- `sub_80C552A`: on its fourth tick, light the panel ahead (and above and
-- below it, if wide); out after its ticks.
local function burn(me: Object)
    local s = me.state :: State
    local t = me.timer - 1
    me.timer = t
    if t == 0 and s.spread ~= 0 then
        s.spread = s.spread - 1
        local x, y = me.panel_x + me.facing, me.panel_y
        spread_to(me, x, y)
        if s.wide then
            spread_to(me, x, y - 1)
            spread_to(me, x, y + 1)
        end
    end
    local left = s.ticks - 1
    s.ticks = left
    if left == 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C54E0`: resolve its hits (one hit and its region is gone), burn,
-- and light its panel.
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
    if me.action ~= 0 then
        error(string.format("flame action %#x reads past its table", me.action))
    end
    burn(me)
    c:present()
    field.highlight(me.panel_x, me.panel_y)
end

-- `sub_80C53FA`.
local function init(me: Object)
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) then
        me:free()
        return
    end
    local s = me.state :: State
    local spread = s.spread
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = s.anim
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    -- (On its panel, `byte_80C54D0`'s offset is 0.)
    if not s.on_panel then
        -- The owner's hand: its sprite's attach point (`sub_8018810`),
        -- which the game turns the flame's way, not the owner's.
        local owner = me.related1 :: Object
        local ax, az = owner:attach_point(HAND)
        if owner.facing ~= me.facing then
            ax = -ax
        end
        p = Vec3.new(int.i32(p.x - me.facing * BACK * 0x10000 + ax * 0x10000), p.y, int.i32(p.z + az * 0x10000))
    end
    -- (Its Y and Z also rise by its spread count, in pixels.)
    me.pos = Vec3.new(p.x, int.i32(p.y + spread * 0x10000), int.i32(p.z + spread * 0x10000))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(sparks.fire)
    c:present()
    me.timer = SPREAD_AT
    me:set_lifecycle("update")
end

-- `sub_80C53E4`: out; it cracks its panel if it cracks.
local function out(me: Object)
    if (me.state :: State).cracks then
        field.crack(me.panel_x, me.panel_y)
    end
    me:destroy()
end

flame.kind = define.kind {
    id = "bn6:flame",
    pool = "attack",
    state = {
        ticks = "u8",
        spread = "u8",
        cracks = "bool",
        anim = "u8",
        on_panel = "bool",
        wide = "bool",
    },
    -- `sub_80C53C0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            out(me)
        end
        if me.active then
            me:update_sprite()
        end
    end,
}

return flame
