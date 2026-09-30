--!strict
-- A mote of a dust storm (effect object #0x19, `sub_80E2080`): it drifts
-- down and forward across its panel from a random point, over and over,
-- for Param1 ticks, or until the storm's user leaves the action Param2
-- ties it to (Param2 2: none, and it runs while dimmed).

local dust_storm_mote = {}

local SPRITE = "10-10"
-- Where each drift starts: 20 pixels back and 128 up, then a random step
-- of 15 along its line; its speed, 16.16.
local BACK, TOP = 0x14_0000, 0x80_0000
local VX, VZ = 0x2_AAAA, -0x8_8888
local DRIFT_TICKS = 0xF
-- The actions the user must stay in, by Param2 (`dword_80E21A8`).
local TIED = { [0] = 0x0D, 0x0E, 0x00, 0x5E }
local UNTIED = 2

-- `sub_80E21AC`: a mote of `storm`'s on (x, y), tied to `user`, for
-- `ticks` ticks in mode `mode`. (It spawns at the registers' values,
-- which its init replaces.)
function dust_storm_mote.spawn(storm: Object, user: Object?, x: number, y: number, ticks: number, mode: number): Object?
    local o = battle.spawn_kind("dust-storm-mote", Vec3.new(y, 0, 0), { ticks, mode, 0, 0 })
    if o then
        o.panel_x, o.panel_y = x, y
        o.related1 = user
        o.alliance, o.flip = storm.alliance, storm.flip
    end
    return o
end

-- `sub_80E20B0`.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local r = battle.rng_positive() % 15
    local back = -me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x + back * BACK - int.mul32(VX * back, r), p.y, TOP + int.mul32(VZ, r))
    me.timer = DRIFT_TICKS - r
    me.timer2 = me:param(1)
    me.vel = Vec3.new(me.facing * VX, me.vel.y, VZ)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E213C`.
local function tick(me: Object)
    if battle.over() then
        me:free()
        return
    end
    local mode = me:param(2)
    if mode ~= UNTIED then
        local action = TIED[mode]
        if action == nil then
            error(string.format("dust mote mode %d reads past dword_80E21A8", mode))
        end
        if (me.related1 :: Object).action ~= action then
            me:free()
            return
        end
        if battle.dimmed() then
            return
        end
    end
    local left = me.timer2 - 1
    me.timer2 = left
    if left <= 0 then
        me:free()
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z + me.vel.z)
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me:set_coordinates_from_panel()
    local q = me.pos
    me.pos = Vec3.new(q.x - me.facing * BACK, q.y, TOP)
    me.timer = DRIFT_TICKS
end

-- `sub_80E2080`.
function dust_storm_mote.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
    end
    if me:param(2) == UNTIED then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return dust_storm_mote
