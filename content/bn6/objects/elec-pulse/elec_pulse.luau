--!strict
-- An electric pulse (attack object #0x8C, `sub_80D6A20`): ElcPuls's and
-- DestPuls's beam, and a virus's. It stands where it was fired, out from
-- its owner's hand (the owner's attach point 0x1C), and covers region 0x15
-- (the panels ahead) for its third parameter's ticks, or until it has hit
-- something; hits pull (collision type 0xB) and take their status from its
-- second parameter unless its element gave one. Its first parameter is the
-- hit modifier, its fourth the look: palette 1, 2, 0 or 3, and from 2 a
-- bug (2: 0x18 with argument 1, 3: HP bug 0xF6).

local elec_pulse = {}

export type State = {
    -- Objects it ends with it (`+0x64`): none of its spawners sets them.
    linked: { Object? },
}

elec_pulse.state = { linked = "object[3]" }

local SPRITE = "10-40"
-- The palette by look (`dword_80D6B24`).
local PALETTES = { [0] = 1, 2, 0, 3 }
-- How far back from its panel it starts (whole pixels), before the hand's
-- offset.
local BACK = 0x28
local HAND = 0x1C
local REGION = 0x15

-- `sub_80D6B78`: a pulse for `owner` on (x, y) with the damage word
-- `damage` and parameters `params`, stored in the owner's first related
-- slot (the game passes the slot; ElcPuls's is that one). Its position is
-- the spawner's registers (y, the element, 0) until its init places it.
function elec_pulse.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("elec-pulse", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        owner.related1 = o
    end
    return o
end

-- Gone: its region cleared, and the objects linked to it told to end
-- (their timer set to 1).
local function finish(me: Object)
    me.collision.region = 0
    me:set_lifecycle("destroy")
    local s = me.state :: State
    for i = 1, 3 do
        local o = s.linked[i]
        if o then
            o.timer = 1
        end
    end
end

-- `sub_80D6B28`: resolve its hits; stop hitting once it hit something;
-- gone when its time is up or the battle is over.
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
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        return finish(me)
    end
    c:present()
    me:highlight_collision_panels()
end

-- `sub_80D6A44`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    local look = me:param(4)
    local palette = PALETTES[look]
    if palette == nil then
        error(string.format("electric pulse look %d reads past dword_80D6B24", look))
    end
    sprite.palette = palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = me:param(3)
    me:set_coordinates_from_panel()
    local p = me.pos
    local owner = me.related1 :: Object
    local hx, hz = owner:attach_point(HAND)
    me.pos = Vec3.new(p.x - me.facing * BACK * 0x10000 + hx * 0x10000, p.y, p.z + hz * 0x10000)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x0B, 5, me:param(1))
    local c = me.collision
    c.region = REGION
    c.hit_effect = 0xFF
    if c.status_base == 0 then
        c.status_base = me:param(2)
    end
    if look == 3 then
        c.bugs = 0xF6
    elseif look >= 2 then
        c.bugs = 0x18 + 0x100
    end
    c:present()
    battle.play_sound(0xF3)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D6A20`.
function elec_pulse.update(me: Object)
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

return elec_pulse
