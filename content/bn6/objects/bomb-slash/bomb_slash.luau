--!strict
-- A bomb's lingering hit (attack object #0xA, `sub_80C5F60`): what a
-- kind-1 bomb (objects/bomb) leaves on its panel, drawn as a sword slash.
-- It hits there every tick until it hits something, the battle ends, or
-- its second parameter's ticks run out. No chip throws a kind-1 bomb.

local bomb_slash = {}

local SPRITE = "0c-00"
-- Collision: self type 4, target type 5, hit modifier 3.
local SELF_TYPE, TARGET, HIT_MOD = 0x04, 0x05, 3

-- `sub_80C6018`: a lingering hit for `owner` on (x, y) for `ticks`, with
-- the damage word. (Its position is the spawner's registers, the panel
-- row and element, until its init places it; it keeps no flip.)
function bomb_slash.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    ticks: number,
    damage: number,
    stamina: number
): Object?
    local o = battle.spawn_kind("bomb-slash", Vec3.new(y, element, 0), { 0, ticks, 0, 0 })
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o.damage = damage
        o.stamina = stamina
        o.alliance = owner.alliance
    end
    return o
end

-- `sub_80C5FDE`: resolve last tick's hits; end with the battle, the
-- timer or a hit, else hit again.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if not battle.over() then
        local t = me.timer - 1
        me.timer = t
        if t >= 0 and me.collision.hit_flags == 0 then
            me.collision:present()
            return
        end
    end
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C5F84`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0xFF
    me.timer = me:param(2)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET, HIT_MOD)
    -- `sub_801A146`: the hit spark is the element's.
    me.collision.hit_effect = bit32.band(me.element, 0xF)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C5F60`.
function bomb_slash.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
        if not me.active then
            return
        end
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite()
end

return bomb_slash
