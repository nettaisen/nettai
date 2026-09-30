--!strict
-- A whirlwind (attack object #0x81, `sub_80D5580`), TenguCross Beast's
-- charged chip (action 0x3D): it stands 40 pixels ahead of its panel for
-- Param3 ticks, highlighting the panels it sweeps, and leaves lingering
-- hit regions on its way: with Param1 0 as it passes 45, 30 and 15 ticks
-- left (by `off_80D56C4`'s three waves), with Param1 set all at once when
-- Param3 are left. Once its owner leaves action Param2 it ends 4 ticks
-- later.

local whirlwind = {}

local SPRITE = "10-44"

-- How far ahead of its panel's center it stands (`byte_80D5738`), 16.16.
local AHEAD = 0x28_0000

-- The height of its hits, 16.16, and their hit modifier.
local HIT_Z, HIT_MOD = 0x10_0000, 1

-- The last panels it sweeps: region 4 (the column) two panels ahead.
local SWEEP_REGION = 4

-- A wave's hits (`byte_80D56D0`, `byte_80D56E8`, `byte_80D5700`): each
-- entry's columns ahead, how long it lingers, whether it is there, and the
-- hit's region, spark and collision types.
type Hit = { ahead: number, ticks: number, on: boolean, region: number, hit_effect: number, target: number, self_type: number }

local function hit(ahead: number, ticks: number, region: number): Hit
    return { ahead = ahead, ticks = ticks, on = true, region = region, hit_effect = 0, target = 5, self_type = 0x12 }
end

local NONE: Hit = { ahead = 0, ticks = 0, on = false, region = 0, hit_effect = 0, target = 0, self_type = 0 }

local WAVES: { [number]: { Hit } } = {
    [0] = { hit(0, 0x2D, 1), hit(1, 0x0F, 1), hit(2, 0x0F, 4) },
    [1] = { NONE, hit(1, 0x1E, 1), hit(2, 0x0F, 4) },
    [2] = { NONE, NONE, hit(2, 0x0F, 4) },
}

-- With Param1 0, the wave by the ticks left.
local WAVE_TICKS = { [0x2D] = 0, [0x1E] = 1, [0x0F] = 2 }

-- `sub_80D5718`: a whirlwind from `owner` on (x, y). Its position is the
-- registers (y, the element, 0) until its init places it.
function whirlwind.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("whirlwind", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = owner
    end
    return o
end

-- `sub_80D5684`: one of a wave's hits, lingering `ticks`.
local function strike(me: Object, h: Hit)
    if not h.on then
        return
    end
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + me.facing * h.ahead,
        panel_y = me.panel_y,
        element = me.element,
        z = HIT_Z,
        region = h.region,
        hit_effect = h.hit_effect,
        target = h.target,
        self_type = h.self_type,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if box then
        box.timer = h.ticks
    end
end

-- `sub_80D5634`: 4 more ticks once the owner leaves its action; a wave of
-- hits at its ticks.
local function waves(me: Object)
    if me.action == 0 and (me.related1 :: Object).action ~= me:param(2) then
        me.timer = 4
        me.action = 4
    end
    local wave: number? = 0
    if me:param(1) ~= 0 then
        if me.timer ~= me:param(3) then
            return
        end
    else
        wave = WAVE_TICKS[me.timer]
        if wave == nil then
            return
        end
    end
    for _, h in WAVES[wave :: number] do
        strike(me, h)
    end
end

-- `sub_8109660(x, y, region, 0)`: highlight a region's panels on the field
-- around (x, y), facing right whatever the object's side.
local function highlight_region(x: number, y: number, region: number)
    for _, offset in data.regions[region] do
        local px, py = x + offset[1], y + offset[2]
        if px >= 1 and px <= 6 and py >= 1 and py <= 3 then
            field.highlight(px, py)
        end
    end
end

-- `sub_80D55F8`: the waves; then highlight the panels it sweeps, or end.
local function tick(me: Object)
    waves(me)
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    local front = me.facing
    local x, y = me.panel_x, me.panel_y
    field.highlight(x, y)
    field.highlight(x + front, y)
    highlight_region(x + 2 * front, y, SWEEP_REGION)
end

-- `sub_80D55A4`: ahead of its panel, animation Param4.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = true
    local anim = me:param(4)
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = 0
    me.timer = me:param(3)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, p.z)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D5580`.
function whirlwind.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return whirlwind
