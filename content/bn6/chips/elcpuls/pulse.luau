--!strict
-- An electric pulse (attack object #0x8C, `sub_80D6A20`): ElcPuls's and
-- DestPuls's beam. It stands where it was fired, out from its owner's hand
-- (the owner's attach point 0x1C), and covers the panel ahead and the
-- column past it for its ticks, or until it has hit something; hits pull
-- and take their status from the pulse unless its element gave one. What
-- differs between the chips is the pulse (`ElecPulse`, types.d.luau, the
-- original's parameters: hit modifier, status, ticks, and a look that
-- gives the palette, `dword_80D6B24`, and from the third look a bug).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local pulse = {}

export type State = {
    -- Objects it ends with it (`+0x64`): none of its spawners sets them.
    linked: { Object? },
    pulse: ElecPulse,
}

local SPRITE = asset.sprite("elec-pulse")
local SOUND = asset.sound("elec-pulse")
-- How far back from its panel it starts (whole pixels), before the hand's
-- offset.
local BACK = 0x28
local HAND = 0x1C

-- A pulse.
function pulse.variant(v: ElecPulse): ElecPulse
    return define.record("elec-pulse", v)
end

-- Gone: its region cleared, and the objects linked to it told to end
-- (their timer set to 1).
local function finish(me: Object)
    me.collision:set_region(nil)
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
        c:set_region(nil)
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
    local v = (me.state :: State).pulse
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = v.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = v.ticks
    me:set_coordinates_from_panel()
    local p = me.pos
    local owner = me.related1 :: Object
    local hx, hz = owner:attach_point(HAND)
    me.pos = Vec3.new(p.x - me.facing * BACK * 0x10000 + hx * 0x10000, p.y, p.z + hz * 0x10000)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.piercing, collision.hits_navis, v.hit_mod)
    local c = me.collision
    c:set_region(regions.fan)
    c:set_hit_effect(nil)
    if c.status_base == nil then
        c.status_base = v.status
    end
    local bug = v.bug
    if bug then
        c.bugs = bug + (v.bug_arg or 0) * 0x100
    end
    c:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

pulse.kind = define.kind {
    id = "elcpuls/pulse",
    pool = "attack",
    state = { linked = "object[3]", pulse = "record:elec-pulse" },
    -- `sub_80D6A20`.
    update = function(me: Object)
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
    end,
}

-- `sub_80D6B78`: `v` for `owner` on (x, y) with the damage word
-- `damage`, stored in the owner's first related slot (the game passes the
-- slot; ElcPuls's is that one). Its position is the spawner's registers
-- (y, the element, 0) until its init places it.
function pulse.spawn(owner: Object, x: number, y: number, element: number, v: ElecPulse, damage: number): Object?
    local o = battle.spawn(pulse.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        owner.related1 = o
        local s = o.state :: State
        s.pulse = v
    end
    return o
end

return pulse
