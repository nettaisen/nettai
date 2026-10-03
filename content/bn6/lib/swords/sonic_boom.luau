--!strict
-- The sonic boom (the original's attack object #0x58, `sub_80CF7F0`): a
-- slash that flies forward 16 pixels up over a column of three panels,
-- from the sword chips that throw one (SonicBom, SprSonic and the chips
-- sharing their action; Z Saver's fourth slash). It ends on what it hits,
-- or goes through (then hitting each panel once). Its sprite stays on its
-- first frame. See docs/engine/shot-chips.md §12.1. Its look (sprite,
-- height, palettes, collision type) is a game's: BN6's below
-- (`sonic_boom.kind`); another game's kind of it is
-- `sonic_boom.make_kind(id, look)` (BN5's).

local collision = require("../../rules/collision")
local regions = require("@common/regions")

local sonic_boom = {}

export type State = {
    -- Its hit modifier (Param1) and whether it goes through (Param2).
    hit_mod: number,
    pierces: boolean,
}

-- A game's sonic boom: its sprite, its height, its palette when it goes
-- through, else by its element (`element_mask` of its element byte
-- indexing `palettes`), its collision type.
export type Look = {
    sprite: SpriteAsset,
    z: number,
    piercing_palette: number,
    palettes: { [number]: number },
    element_mask: number,
    is: CollisionType,
}

-- BN6's: 16 pixels up; its primary element's palette (`byte_80CF8A8`); a
-- slash.
local BN6: Look = {
    sprite = asset.sprite("bn6:sword-slash"),
    z = 0x10_0000,
    piercing_palette = 0xB,
    palettes = { [0] = 0, 1, 2, 3, 4, 0, 0, 0 },
    element_mask = 0xF,
    is = collision.slash,
}
-- Its speed (16.16 pixels a tick) as its throwers set it.
sonic_boom.SPEED = 0x8_0000
-- The hits that stop it (or that it goes through).
local HITS = 0x3F800000

local function finish(me: Object)
    me.collision:set_region(nil)
    me.lifecycle = "destroy"
end

-- `sub_80CF8F8`: forward by its speed; its panel and its collision's from
-- its position; off the field it ends.
local function move(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    local c = me.collision
    c.panel_x, c.panel_y = me.panel_x, me.panel_y
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
    end
end

-- `sub_80CF8B0`: the hits against it resolve (no spark); the battle's end
-- ends it, and so does a hit, unless it goes through (its region off until
-- its next panel); it moves.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    if battle.over() then
        finish(me)
        return
    end
    if bit32.band(c.hit_flags, HITS) ~= 0 then
        if not s.pierces then
            finish(me)
            return
        end
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    move(me)
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.wide)
    end
    c:present()
end

-- `sub_80CF810`: shown on its panel 16 pixels up, flying the way it faces
-- at the speed it was given; a slash of its side over the column.
local function init(me: Object, look: Look)
    if battle.over() then
        me:free()
        return
    end
    local s = me.state :: State
    me.visible = true
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "with_sprite"
    me.anim = 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if s.pierces then
        sprite.palette = look.piercing_palette
    else
        local e = bit32.band(me.element, look.element_mask)
        local palette = look.palettes[e]
        if palette == nil then
            error(string.format("element %#x reads past byte_80CF8A8", e))
        end
        sprite.palette = palette
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, look.z)
    me.timer = 0
    me.vel = Vec3.new(int.i32(me.facing * me.vel.x), me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(look.is, collision.hits_navis, s.hit_mod)
    me.collision:set_region(regions.wide)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- A sonic boom kind (`id`) of `look`.
function sonic_boom.make_kind(id: string, look: Look): Kind
    return define.kind {
        id = id,
        pool = "attack",
        state = { hit_mod = "u8", pierces = "bool" },
        -- `sub_80CF7F0`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }
end

sonic_boom.kind = sonic_boom.make_kind("bn6:sonic-boom", BN6)

-- `sub_80CF91E` and its throwers' speed: a sonic boom from `owner` on
-- panel (x, y), with `element`, the damage word and what it is, flying at
-- `SPEED`; nil if the attack pool is full. It takes the owner's side and
-- flip, runs later this tick, right after its spawner; its init sets its
-- Z. (A game's own kind of it: `kind`.)
function sonic_boom.throw(owner: Object, x: number, y: number, element: number, damage: number, boom: SonicBoom, kind: Kind?): Object?
    local o = battle.spawn(kind or sonic_boom.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.hit_mod = boom.hit_mod
        s.pierces = boom.pierces
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.vel = Vec3.new(sonic_boom.SPEED, 0, 0)
    end
    return o
end

return sonic_boom
