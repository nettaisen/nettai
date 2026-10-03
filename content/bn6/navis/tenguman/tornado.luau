--!strict
-- A Tengu tornado (the original's attack object #0x60, `sub_80D0500`):
-- TenguMan's FTornado on an enemy's panel. It stands there for 31 ticks
-- and hits the panel (a one-tick hit) on its first tick and every 12
-- after. It waits while dimmed and goes when the battle is over. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local tornado = {}

local SPRITE = asset.sprite("bn6:tengu-tornado")

-- Its hits: pushing, reaching nothing by its target; no spark.
local HIT_MOD = 1
local HIT_EVERY, TICKS = 0x0C, 0x1E

-- `sub_80D05EC`: a hit on its panel.
local function hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = nil,
        target = collision.nothing,
        self_type = collision.pushing,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

-- `sub_80D0590`: 31 ticks, a hit every 12 from the first.
local function blow(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = TICKS
        me.timer2 = HIT_EVERY
    end
    local n = me.timer2 + 1
    me.timer2 = n
    if n >= HIT_EVERY then
        hit(me)
        me.timer2 = 0
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.lifecycle = "destroy"
    end
end

-- `sub_80D0566`: gone when the battle is over; still while dimmed.
local function tick(me: Object)
    if battle.over() then
        me.lifecycle = "destroy"
        return
    end
    if battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("Tengu tornado action %#x reads past off_80D058C", me.action))
    end
    blow(me)
end

-- `sub_80D0524`: on its panel, on the ground (keeping its Z's fraction),
-- with a ground shadow.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

tornado.kind = define.kind {
    id = "bn6:tenguman/tornado",
    pool = "attack",
    -- `sub_80D0500`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80D05C4`: a tornado for `owner` on (x, y), with `element` and the
-- damage word `damage`. It runs while dimmed (and waits). `pos` is where
-- its spawner's registers leave it (its row, the element and a Z), of
-- which its init keeps the Z's fraction.
function tornado.spawn(owner: Object, x: number, y: number, element: number, damage: number, pos: Vec3): Object?
    local o = battle.spawn(tornado.kind, pos)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return tornado
