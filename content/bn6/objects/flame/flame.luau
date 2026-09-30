--!strict
-- A flame (attack object #0x04, `sub_80C53C0`): FireBrn's and WideBrn's
-- fire. It burns on its panel for Param1 ticks with its collision on;
-- on its fourth tick, while Param2 is left, it lights the next panel
-- forward (and the ones above and below it too when Param2 had bit 0x80,
-- WideBrn's, the first time), each new flame with Param2 one less. When it
-- goes out it cracks its panel if Param3 says so. Param4 picks its
-- animation and where it starts: 0 and 2 at its owner's hand (attach
-- point 0x12 of the NameID in its first related slot, 40 pixels back), 1
-- on its panel.
--
-- The game gives a flame a pointer (ExtraVars+0) to a word that keeps it
-- burning while nonzero. FireBrn's is null, and reading address 0 outside
-- the BIOS gets the BIOS's last fetched opcode, never zero, so FireBrn's
-- flames burn their full time; the chain passes the pointer on.

local SPRITE = "0c-0e"
-- Collision types: what it is, what it reacts to, its hit modifier; its
-- hit spark.
local SELF_TYPE, TARGET_TYPE, HIT_MOD, HIT_EFFECT = 0x0A, 5, 3, 1
-- Its animation by Param4 (`byte_80C54DC`).
local ANIMS = { [0] = 0, 1, 1, 0 }
-- Param2's bit for a wide flame (three rows).
local WIDE = 0x80
-- The owner's attach point it starts at, and how far back of it.
local HAND, BACK = 0x12, 0x28
-- Ticks before it lights the next panel.
local SPREAD_AT = 4

export type State = {
    -- It lights three panels at once (ExtraVars+4).
    wide: boolean,
}

local flame = {
    state = { wide = "bool" },
}

-- `sub_80C5578`: a flame for `owner` on (x, y), with the four parameters
-- (lifetime, spread and the wide bit, crack, look) and the damage word
-- `damage`. Its position is the spawner's registers (the panel Y, the
-- element and 0) until its init places it.
function flame.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("flame", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80C5590`: the next flame on (x, y), with this one's parameters
-- and owner.
local function spread_to(me: Object, x: number, y: number)
    local params = { me:param(1), me:param(2), me:param(3), me:param(4) }
    local damage = me.damage + me.stamina * 0x10000
    local o = flame.spawn(me, x, y, me.element, params, damage)
    if o then
        o.related1 = me.related1
    end
end

-- `sub_80C552A`: on its fourth tick, light the panel ahead (and above and
-- below it, if wide); out after Param1 ticks.
local function burn(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t == 0 and me:param(2) ~= 0 then
        me:set_param(2, me:param(2) - 1)
        local x, y = me.panel_x + me.facing, me.panel_y
        spread_to(me, x, y)
        if (me.state :: State).wide then
            spread_to(me, x, y - 1)
            spread_to(me, x, y + 1)
        end
    end
    local left = me:param(1) - 1
    me:set_param(1, left)
    if left == 0 then
        me.collision.region = 0
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
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
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
    local spread = me:param(2)
    if bit32.btest(spread, WIDE) then
        spread = bit32.band(spread, bit32.bnot(WIDE))
        me:set_param(2, spread);
        (me.state :: State).wide = true
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    local look = me:param(4)
    local anim = ANIMS[look]
    if anim == nil then
        error(string.format("flame look %d reads past byte_80C54DC", look))
    end
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    -- (Look 1's offset from `byte_80C54D0` is 0.)
    if look ~= 1 then
        local owner = me.related1 :: Object
        local ax, az = battle.attach_point(owner.name_id, HAND, me.alliance, me.flip)
        p = Vec3.new(int.i32(p.x - me.facing * BACK * 0x10000 + ax * 0x10000), p.y, int.i32(p.z + az * 0x10000))
    end
    -- (Its Y and Z also rise by its spread count, in pixels.)
    me.pos = Vec3.new(p.x, int.i32(p.y + spread * 0x10000), int.i32(p.z + spread * 0x10000))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    local c = me.collision
    c.region = 1
    c.hit_effect = HIT_EFFECT
    c:present()
    me.timer = SPREAD_AT
    me:set_lifecycle("update")
end

-- `sub_80C53E4`: out; it cracks its panel if Param3 says so.
local function out(me: Object)
    if me:param(3) ~= 0 then
        field.crack(me.panel_x, me.panel_y)
    end
    me:destroy()
end

-- `sub_80C53C0`.
function flame.update(me: Object)
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
end

return flame
