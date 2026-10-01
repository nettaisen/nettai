--!strict
-- A segment of an erase slash (the original's attack object #0xC3,
-- `sub_80DD940`): it stands on its panel with its collision on (status
-- 0x10) for its time, or until it has hit something. EraseMan's (the
-- EraseMan chips') run while dimmed; a navi's own (EraseMan's EDeletBm)
-- stand still while dimmed, and go once their owner is out of its action
-- 0xA. See docs/engine/chips.md §3.6.10.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local beam = {}

-- What a segment is spawned with (the original's three parameters).
export type Spec = {
    -- The slash's aim: 0 up-forward, 1 forward, 2 down-forward.
    aim: number,
    -- How long it stands.
    ticks: number,
    -- A navi's own segment (EDeletBm's).
    navis: boolean,
}
export type State = Spec

local SPRITE = asset.sprite("erase-beam")

-- By aim: the height in whole pixels (`dword_80DD9FC`) and the animation
-- (`dword_80DDA04`); a fourth entry is each word's last byte.
local HEIGHTS = { [0] = 0x23, 0x28, 0x2D, 0 }
local ANIMS = { [0] = 1, 0, 2, 0 }

-- How far behind its panel's center it stands, 16.16.
local BACK = 0x8_0000

-- The status its hits carry (paralysis).
local STATUS = 0x10

-- `sub_80DD970`: on its panel at its aim's height, and collision on.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local height, anim = HEIGHTS[s.aim], ANIMS[s.aim]
    if not height then
        error(string.format("erase beam aim %d reads past its tables", s.aim))
    end
    local p = me.pos
    me.pos = Vec3.new(p.x - me.facing * BACK, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), height * 0x10000))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.piercing_break, collision.hits_navis, 1)
    me.collision.status_base = STATUS
    me.collision:set_hit_effect(sparks.erase)
    me.collision:present()
    me.timer = s.ticks
    me:set_lifecycle("update")
end

-- `object_clearCollisionRegion`, and the destroy state.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80DDA08`: resolve the hits; once it hit something its region goes;
-- after its time (or, a navi's, once the navi's action is over) it ends.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        return finish(me)
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if s.navis and (me.related1 :: Object).action ~= 0xA then
        return finish(me)
    end
    if s.navis and battle.dimmed() then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        c:present()
    else
        finish(me)
    end
end

beam.kind = define.kind {
    id = "eraseman/beam",
    pool = "attack",
    state = { aim = "u8", ticks = "u8", navis = "bool" },
    -- `sub_80DD940`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).navis then
            me:update_sprite_while_paused()
        else
            me:update_sprite_while_dimmed()
        end
    end,
}

-- `sub_80DDA5A`: a segment on (x, y) for `owner`, with the damage word
-- `damage`. It runs while dimmed. `pos` is where the spawner's registers
-- leave it (only its Z fraction lasts past its init); EraseMan's is zero.
function beam.spawn(owner: Object, x: number, y: number, element: number, spec: Spec, damage: number, pos: Vec3?): Object?
    local o = battle.spawn(beam.kind, pos or Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.aim, s.ticks, s.navis = spec.aim, spec.ticks, spec.navis
    end
    return o
end

return beam
