--!strict
-- A segment of an erase slash (attack object #0xC3, `sub_80DD940`): it
-- stands on its panel with its collision on (status 0x10) for Param2
-- ticks, or until it has hit something. Param1 is the slash's aim.
--
-- EraseMan's (Param3 0) run while dimmed. A navi's own (Param3 set) stand
-- still while dimmed, and go once their owner is out of its action 0xA.
-- See docs/engine/chips.md §3.6.10.

local erase_beam = {}

local SPRITE = "10-51"

-- By aim (Param1): the height in whole pixels (`dword_80DD9FC`) and the
-- animation (`dword_80DDA04`); a fourth entry is each word's last byte.
local HEIGHTS = { [0] = 0x23, 0x28, 0x2D, 0 }
local ANIMS = { [0] = 1, 0, 2, 0 }

-- How far behind its panel's center it stands, 16.16.
local BACK = 0x8_0000

-- `sub_80DDA5A`: a segment on (x, y) for `owner`, with the damage word
-- `damage` and `params` {aim, ticks, navi's (Param3)}. It runs while
-- dimmed. `pos` is where the spawner's registers leave it (only its Z
-- fraction lasts past its init); EraseMan's is zero.
function erase_beam.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    pos: Vec3?
): Object?
    local o = battle.spawn_kind("erase-beam", pos or Vec3.zero, params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80DD970`: on its panel at its aim's height, and collision on.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local aim = me:param(1)
    local height, anim = HEIGHTS[aim], ANIMS[aim]
    if not height then
        error(string.format("erase beam aim %d reads past its tables", aim))
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
    me:setup_collision(0x16, 5, 1)
    me.collision.status_base = 0x10
    me.collision.hit_effect = 0xC
    me.collision:present()
    me.timer = me:param(2)
    me:set_lifecycle("update")
end

-- `object_clearCollisionRegion`, and the destroy state.
local function finish(me: Object)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80DDA08`: resolve the hits; once it hit something its region goes;
-- after its time (or, a navi's, once the navi's action is over) it ends.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        return finish(me)
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    local navis = me:param(3) ~= 0
    if navis and (me.related1 :: Object).action ~= 0xA then
        return finish(me)
    end
    if navis and battle.dimmed() then
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

-- `sub_80DD940`.
function erase_beam.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(3) ~= 0 then
        me:update_sprite_while_paused()
    else
        me:update_sprite_while_dimmed()
    end
end

return erase_beam
