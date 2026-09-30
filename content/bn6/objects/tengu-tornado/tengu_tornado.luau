--!strict
-- A Tengu tornado (attack object #0x60, `sub_80D0500`): TenguMan's
-- FTornado. It stands on an enemy's panel for 31 ticks and hits it (a
-- one-tick region) on its first tick and every 12 after. It waits while
-- dimmed and goes when the battle is over. [unverified: no lab scenario
-- reaches it; docs/engine/standard-chips.md]

local tengu_tornado = {}

local SPRITE = "10-42"
-- Its hits: region 1, no spark, target 0, self type 0x12, hit modifier 1.
local REGION, TARGET, SELF, HIT_MOD = 1, 0, 0x12, 1
local HIT_EVERY = 0x0C

-- `sub_80D05C4`: a tornado for `owner` on (x, y), with the damage word
-- `damage`; its first parameter is the panel's column (the spawner's loop
-- counter). It runs while dimmed. Its position is the spawner's registers
-- (the panel Y, the element and `z`), of which its init keeps the Z's
-- fraction.
function tengu_tornado.spawn(owner: Object, x: number, y: number, element: number, damage: number, z: number): Object?
    local o = battle.spawn_kind("tengu-tornado", Vec3.new(y, element, z), { x })
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

-- `sub_80D05EC`: a hit on its panel.
local function hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        region = REGION,
        hit_effect = 0xFF,
        target = TARGET,
        self_type = SELF,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

-- `sub_80D0590`: 31 ticks, a hit every 12 from the first.
local function blow(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 0x1E
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

-- `sub_80D0566`.
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

-- `sub_80D0524`: on its panel, on the ground (keeping the Z's fraction).
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

-- `sub_80D0500`.
function tengu_tornado.update(me: Object)
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
end

return tengu_tornado
